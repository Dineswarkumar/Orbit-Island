//! Opt-in diagnostic instrumentation and deterministic test drivers (`--features bughunt-probe`).

use orbit_platform::process::{self, ProcessCounters};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BughuntMode {
    None,
    SettingsOpen,
    SettingsBench,
    Bug1ThirtyMin,
}

pub fn parse_mode() -> BughuntMode {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let val = if let Some(rest) = arg.strip_prefix("--bughunt-mode=") {
            Some(rest.to_string())
        } else if arg == "--bughunt-mode" {
            args.next()
        } else {
            None
        };
        if let Some(raw) = val {
            return match raw.to_ascii_lowercase().as_str() {
                "settings-open" => BughuntMode::SettingsOpen,
                "settings-bench" => BughuntMode::SettingsBench,
                "bug1-30m" => BughuntMode::Bug1ThirtyMin,
                _ => BughuntMode::None,
            };
        }
    }
    BughuntMode::None
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ProbeDirective {
    #[default]
    None,
    OpenSettings,
    CloseSettings,
    OpenPanel,
    ClosePanel,
    QuitApp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BenchStage {
    WaitInitialTrim,
    WaitOpenFrame,
    WaitIdleBetween,
    Done,
}

pub struct BughuntProbe {
    mode: BughuntMode,
    start: Instant,
    // settings-open driver
    settings_open_triggered: bool,
    // settings-bench driver (trials 0..5 cold post-trim, 5..10 warm no-trim)
    bench_stage: BenchStage,
    bench_trial: u32,
    bench_phase_started: Instant,
    // bug1-30m driver
    bug1_next_minute: u32,
    bug1_panel_cycles_done: u32,
    bug1_panel_open: bool,
    bug1_panel_phase_at: Instant,
    bug1_settings_cycles_done: u32,
    bug1_settings_open: bool,
    bug1_settings_phase_at: Instant,
    bug1_quit_sent: bool,
}

impl BughuntProbe {
    pub fn new(mode: BughuntMode) -> Self {
        let now = Instant::now();
        tracing::info!("PROBE initialized mode={mode:?}");
        Self {
            mode,
            start: now,
            settings_open_triggered: false,
            bench_stage: BenchStage::WaitInitialTrim,
            bench_trial: 0,
            bench_phase_started: now,
            bug1_next_minute: 0,
            bug1_panel_cycles_done: 0,
            bug1_panel_open: false,
            bug1_panel_phase_at: now + Duration::from_secs(600),
            bug1_settings_cycles_done: 0,
            bug1_settings_open: false,
            bug1_settings_phase_at: now + Duration::from_secs(1200),
            bug1_quit_sent: false,
        }
    }

    pub fn want_panel_hover(&self) -> bool {
        self.mode == BughuntMode::Bug1ThirtyMin && self.bug1_panel_open
    }

    pub fn on_open_settings(&self, trimmed_before: bool) -> (Instant, ProcessCounters) {
        let started_at = Instant::now();
        let c = process::capture_counters();
        tracing::info!(
            "PROBE open_settings tid={} trimmed_before={} priv_kb={} ws_kb={} faults={} handles={} threads={} gdi={} user={}",
            c.tid,
            trimmed_before,
            c.private_usage_bytes / 1024,
            c.working_set_bytes / 1024,
            c.page_fault_count,
            c.handle_count,
            c.thread_count,
            c.gdi_objects,
            c.user_objects,
        );
        (started_at, c)
    }

    pub fn on_access_status_finished(ms: f64, tid: u32, status: &str) {
        tracing::info!(
            "PROBE settings_access_thread tid={} dur_ms={:.2} status={}",
            tid,
            ms,
            status
        );
    }

    pub fn on_settings_deferred_frame(
        started_at: Option<Instant>,
        start_counters: ProcessCounters,
        first_logged: &mut bool,
        frames_since_open: &mut u32,
        prev_frame_at: &mut Option<Instant>,
        ui_build_ms: f64,
        total_cb_ms: f64,
    ) {
        let now = Instant::now();
        *frames_since_open = frames_since_open.saturating_add(1);
        if !*first_logged {
            *first_logged = true;
            let total_ms = started_at.map_or(total_cb_ms, |t0| t0.elapsed().as_secs_f64() * 1000.0);
            let after = process::capture_counters();
            let fault_delta = after.page_fault_count.saturating_sub(start_counters.page_fault_count);
            let priv_delta_kb =
                (after.private_usage_bytes as i64 - start_counters.private_usage_bytes as i64) / 1024;
            let ws_delta_kb =
                (after.working_set_bytes as i64 - start_counters.working_set_bytes as i64) / 1024;
            tracing::info!(
                "PROBE settings_first_frame_swapped total_ms={:.2} ui_build_ms={:.2} cb_ms={:.2} fault_delta={} priv_delta_kb={} ws_delta_kb={} priv_kb={} ws_kb={} handles={} threads={} gdi={} user={}",
                total_ms,
                ui_build_ms,
                total_cb_ms,
                fault_delta,
                priv_delta_kb,
                ws_delta_kb,
                after.private_usage_bytes / 1024,
                after.working_set_bytes / 1024,
                after.handle_count,
                after.thread_count,
                after.gdi_objects,
                after.user_objects,
            );
        } else if *frames_since_open <= 8 {
            let interval_ms = prev_frame_at
                .map_or(0.0, |p| now.duration_since(p).as_secs_f64() * 1000.0);
            tracing::info!(
                "PROBE settings_frame idx={} cb_ms={:.2} interval_ms={:.2}",
                *frames_since_open,
                total_cb_ms,
                interval_ms
            );
        }
        *prev_frame_at = Some(now);
    }

    pub fn on_close_settings(&mut self) {
        let c = process::capture_counters();
        tracing::info!(
            "PROBE close_settings priv_kb={} ws_kb={} faults={} handles={} gdi={} user={}",
            c.private_usage_bytes / 1024,
            c.working_set_bytes / 1024,
            c.page_fault_count,
            c.handle_count,
            c.gdi_objects,
            c.user_objects,
        );
    }

    /// Drives deterministic scenarios on `OrbitApp::ui` without modifying UI visuals.
    pub fn tick(
        &mut self,
        ctx: &egui::Context,
        settings_open: bool,
        settings_frames: u32,
        trimmed: bool,
    ) -> ProbeDirective {
        let now = Instant::now();
        match self.mode {
            BughuntMode::None => ProbeDirective::None,
            BughuntMode::SettingsOpen => {
                if !self.settings_open_triggered {
                    if now.duration_since(self.start) >= Duration::from_secs(4) {
                        self.settings_open_triggered = true;
                        tracing::info!("PROBE driver settings-open triggering open_settings");
                        return ProbeDirective::OpenSettings;
                    }
                    ctx.request_repaint_after(Duration::from_millis(250));
                }
                ProbeDirective::None
            }
            BughuntMode::SettingsBench => {
                match self.bench_stage {
                    BenchStage::WaitInitialTrim => {
                        if now.duration_since(self.start) >= Duration::from_secs(5) && trimmed {
                            self.bench_trial = 0;
                            self.bench_stage = BenchStage::WaitOpenFrame;
                            self.bench_phase_started = now;
                            tracing::info!(
                                "PROBE bench_trial idx={} kind=cold action=open",
                                self.bench_trial
                            );
                            return ProbeDirective::OpenSettings;
                        }
                        ctx.request_repaint_after(Duration::from_millis(200));
                    }
                    BenchStage::WaitOpenFrame => {
                        if settings_open && settings_frames >= 5 {
                            self.bench_stage = BenchStage::WaitIdleBetween;
                            self.bench_phase_started = now;
                            tracing::info!(
                                "PROBE bench_trial idx={} action=close",
                                self.bench_trial
                            );
                            return ProbeDirective::CloseSettings;
                        }
                        ctx.request_repaint_of(egui::ViewportId::from_hash_of("orbit-settings"));
                        ctx.request_repaint_after(Duration::from_millis(50));
                    }
                    BenchStage::WaitIdleBetween => {
                        let next_trial = self.bench_trial + 1;
                        if next_trial >= 10 {
                            self.bench_stage = BenchStage::Done;
                            let c = process::capture_counters();
                            tracing::info!(
                                "PROBE settings_bench_done trials=10 priv_kb={} ws_kb={} handles={} gdi={} user={}",
                                c.private_usage_bytes / 1024,
                                c.working_set_bytes / 1024,
                                c.handle_count,
                                c.gdi_objects,
                                c.user_objects
                            );
                            return ProbeDirective::QuitApp;
                        }
                        let is_cold = next_trial < 5;
                        let wait = if is_cold {
                            Duration::from_millis(4500)
                        } else {
                            Duration::from_millis(800)
                        };
                        let trim_ok = !is_cold || trimmed;
                        if !settings_open
                            && now.duration_since(self.bench_phase_started) >= wait
                            && trim_ok
                        {
                            self.bench_trial = next_trial;
                            self.bench_stage = BenchStage::WaitOpenFrame;
                            self.bench_phase_started = now;
                            tracing::info!(
                                "PROBE bench_trial idx={} kind={} action=open",
                                self.bench_trial,
                                if is_cold { "cold" } else { "warm" }
                            );
                            return ProbeDirective::OpenSettings;
                        }
                        ctx.request_repaint_after(Duration::from_millis(150));
                    }
                    BenchStage::Done => {}
                }
                ProbeDirective::None
            }
            BughuntMode::Bug1ThirtyMin => {
                let elapsed = now.duration_since(self.start);
                let elapsed_secs = elapsed.as_secs_f64();
                let phase = if elapsed_secs < 600.0 {
                    "A_idle"
                } else if elapsed_secs < 1200.0 {
                    "B_panel"
                } else {
                    "C_settings"
                };

                // Minute-by-minute telemetry inside orbit-egui.log
                let minute_mark = Duration::from_secs(u64::from(self.bug1_next_minute) * 60);
                if self.bug1_next_minute <= 30 && elapsed >= minute_mark {
                    let c = process::capture_counters();
                    tracing::info!(
                        "PROBE bug1_minute min={} elapsed_s={:.1} phase={} priv_mb={:.2} ws_mb={:.2} handles={} threads={} gdi={} user={}",
                        self.bug1_next_minute,
                        elapsed_secs,
                        phase,
                        c.private_usage_bytes as f64 / (1024.0 * 1024.0),
                        c.working_set_bytes as f64 / (1024.0 * 1024.0),
                        c.handle_count,
                        c.thread_count,
                        c.gdi_objects,
                        c.user_objects,
                    );
                    self.bug1_next_minute = self.bug1_next_minute.saturating_add(1);
                }

                let mut directive = ProbeDirective::None;

                // Phase B: open panel (5s) + close (10s), 20 times starting at t=600s
                if elapsed_secs >= 600.0 && elapsed_secs < 1200.0 && self.bug1_panel_cycles_done < 20
                {
                    if now >= self.bug1_panel_phase_at {
                        if !self.bug1_panel_open {
                            self.bug1_panel_open = true;
                            self.bug1_panel_phase_at = now + Duration::from_secs(5);
                            tracing::info!(
                                "PROBE bug1_panel cycle={} action=open elapsed_s={:.1}",
                                self.bug1_panel_cycles_done + 1,
                                elapsed_secs
                            );
                            directive = ProbeDirective::OpenPanel;
                        } else {
                            self.bug1_panel_open = false;
                            self.bug1_panel_cycles_done += 1;
                            self.bug1_panel_phase_at = now + Duration::from_secs(10);
                            tracing::info!(
                                "PROBE bug1_panel cycle={} action=close elapsed_s={:.1}",
                                self.bug1_panel_cycles_done,
                                elapsed_secs
                            );
                            directive = ProbeDirective::ClosePanel;
                        }
                    }
                } else if self.bug1_panel_open {
                    self.bug1_panel_open = false;
                    directive = ProbeDirective::ClosePanel;
                }

                // Phase C: open Settings, wait 10s, close, wait 10s between, 10 times starting at t=1200s
                if matches!(directive, ProbeDirective::None)
                    && elapsed_secs >= 1200.0
                    && self.bug1_settings_cycles_done < 10
                    && now >= self.bug1_settings_phase_at
                {
                    if !self.bug1_settings_open {
                        self.bug1_settings_open = true;
                        self.bug1_settings_phase_at = now + Duration::from_secs(10);
                        tracing::info!(
                            "PROBE bug1_settings cycle={} action=open elapsed_s={:.1}",
                            self.bug1_settings_cycles_done + 1,
                            elapsed_secs
                        );
                        directive = ProbeDirective::OpenSettings;
                    } else {
                        self.bug1_settings_open = false;
                        self.bug1_settings_cycles_done += 1;
                        self.bug1_settings_phase_at = now + Duration::from_secs(10);
                        tracing::info!(
                            "PROBE bug1_settings cycle={} action=close elapsed_s={:.1}",
                            self.bug1_settings_cycles_done,
                            elapsed_secs
                        );
                        directive = ProbeDirective::CloseSettings;
                    }
                }

                if matches!(directive, ProbeDirective::None)
                    && elapsed_secs >= 1860.0
                    && !self.bug1_quit_sent
                {
                    self.bug1_quit_sent = true;
                    let c = process::capture_counters();
                    tracing::info!(
                        "PROBE bug1_30m_done elapsed_s={:.1} priv_mb={:.2} ws_mb={:.2} handles={} gdi={} user={}",
                        elapsed_secs,
                        c.private_usage_bytes as f64 / (1024.0 * 1024.0),
                        c.working_set_bytes as f64 / (1024.0 * 1024.0),
                        c.handle_count,
                        c.gdi_objects,
                        c.user_objects,
                    );
                    return ProbeDirective::QuitApp;
                }

                // Schedule exact next transition / minute-log deadline (avoid artificial timer churn).
                let mut next_wake_at = now + Duration::from_secs(60);
                if self.bug1_next_minute <= 30 {
                    let next_min_at =
                        self.start + Duration::from_secs(u64::from(self.bug1_next_minute) * 60);
                    next_wake_at = next_wake_at.min(next_min_at);
                }
                if elapsed_secs < 600.0 || (elapsed_secs < 1200.0 && self.bug1_panel_cycles_done < 20)
                {
                    next_wake_at = next_wake_at.min(self.bug1_panel_phase_at);
                }
                if self.bug1_settings_cycles_done < 10 {
                    next_wake_at = next_wake_at.min(self.bug1_settings_phase_at);
                }
                if !self.bug1_quit_sent {
                    next_wake_at = next_wake_at.min(self.start + Duration::from_secs(1860));
                }
                let delay = next_wake_at
                    .saturating_duration_since(now)
                    .max(Duration::from_millis(50));
                ctx.request_repaint_after(delay);
                directive
            }
        }
    }
}
