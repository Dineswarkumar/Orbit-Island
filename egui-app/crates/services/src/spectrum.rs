//! Gated 30 fps audio spectrum service: runs ONLY while audio is playing AND panel is visible.
//! Otherwise sleeps on a Condvar with zero polling and zero CPU usage.

use crate::loopback::LoopbackCapture;
use crate::media::{ComGuard, sample_audio_spectrum_meter};
use crate::spectrum_dsp::SpectrumAnalyzer;
use crossbeam_channel::Sender;
use orbit_core::events::{AudioSpectrumState, CoreEvent, WakeFn};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub struct SpectrumService {
    playing: Arc<AtomicBool>,
    visible: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    park: Arc<(Mutex<()>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl SpectrumService {
    pub fn spawn(
        tx: Sender<CoreEvent>,
        wake: WakeFn,
        playing: Arc<AtomicBool>,
        visible: Arc<AtomicBool>,
        park: Arc<(Mutex<()>, Condvar)>,
    ) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let pl = playing.clone();
        let vis = visible.clone();
        let st = stop.clone();
        let pk = park.clone();

        let thread = std::thread::Builder::new()
            .name("orbit-spectrum".into())
            .spawn(move || {
                let _com = match ComGuard::new() {
                    Ok(g) => Some(g),
                    Err(e) => {
                        tracing::error!("COM init failed on orbit-spectrum: {e}");
                        None
                    }
                };
                let mut prev_peak = 0.0f32;
                let mut smooth_low = 0.0f32;
                let mut phase = 0.0f32;
                let mut was_active = false;
                let mut loopback: Option<(LoopbackCapture, SpectrumAnalyzer)> = None;
                let mut ring = VecDeque::with_capacity(4096);
                let mut retry_loopback_at = Instant::now();

                while !st.load(Ordering::Relaxed) {
                    if !(pl.load(Ordering::Acquire) && vis.load(Ordering::Acquire)) {
                        if was_active {
                            was_active = false;
                            loopback = None;
                            ring.clear();
                            prev_peak = 0.0;
                            smooth_low = 0.0;
                            if tx
                                .send(CoreEvent::SpectrumUpdated(AudioSpectrumState::default()))
                                .is_err()
                            {
                                break;
                            }
                            wake();
                        }
                        let guard = pk.0.lock().unwrap_or_else(|e| e.into_inner());
                        if !st.load(Ordering::Relaxed)
                            && !(pl.load(Ordering::Acquire) && vis.load(Ordering::Acquire))
                        {
                            drop(pk.1.wait(guard));
                        }
                        continue;
                    }

                    if loopback.is_none() && Instant::now() >= retry_loopback_at {
                        match LoopbackCapture::new() {
                            Ok(capture) => {
                                let analyzer = SpectrumAnalyzer::new(capture.sample_rate());
                                loopback = Some((capture, analyzer));
                                ring.clear();
                            }
                            Err(_) => {
                                retry_loopback_at = Instant::now() + Duration::from_secs(5);
                            }
                        }
                    }

                    let spectrum = if let Some((capture, analyzer)) = loopback.as_mut() {
                        match capture.read_into(&mut ring) {
                            Ok(()) => {
                                let samples = ring.make_contiguous();
                                let start = samples.len().saturating_sub(2048);
                                let recent = &samples[start..];
                                let rms = if recent.is_empty() {
                                    0.0
                                } else {
                                    (recent.iter().map(|s| s * s).sum::<f32>()
                                        / recent.len() as f32)
                                        .sqrt()
                                };
                                let (wave_bars, compact_bars) = analyzer.analyze(samples);
                                if rms < 1e-4 {
                                    AudioSpectrumState::default()
                                } else {
                                    AudioSpectrumState {
                                        peak: wave_bars.iter().copied().fold(0.0, f32::max),
                                        compact_bars,
                                        wave_bars,
                                    }
                                }
                            }
                            Err(err) => {
                                tracing::warn!("WASAPI loopback capture stopped: {err}");
                                loopback = None;
                                ring.clear();
                                retry_loopback_at = Instant::now() + Duration::from_secs(1);
                                sample_audio_spectrum_meter(
                                    &mut prev_peak,
                                    &mut smooth_low,
                                    &mut phase,
                                )
                            }
                        }
                    } else {
                        sample_audio_spectrum_meter(&mut prev_peak, &mut smooth_low, &mut phase)
                    };

                    let is_active = spectrum.peak > 0.001;
                    if is_active || was_active {
                        was_active = is_active;
                        if tx.send(CoreEvent::SpectrumUpdated(spectrum)).is_err() {
                            break;
                        }
                        wake();
                    }

                    let delay = if is_active { 33 } else { 150 };
                    let guard = pk.0.lock().unwrap_or_else(|e| e.into_inner());
                    if !st.load(Ordering::Relaxed)
                        && pl.load(Ordering::Acquire)
                        && vis.load(Ordering::Acquire)
                    {
                        drop(pk.1.wait_timeout(guard, Duration::from_millis(delay)));
                    }
                }
            })?;

        Ok(Self {
            playing,
            visible,
            stop,
            park,
            thread: Some(thread),
        })
    }

    pub fn set_visible(&self, visible: bool) {
        let prev = self.visible.swap(visible, Ordering::AcqRel);
        if prev != visible {
            self.park.1.notify_all();
        }
    }

    pub fn is_active(&self) -> bool {
        self.playing.load(Ordering::Acquire) && self.visible.load(Ordering::Acquire)
    }
}

impl Drop for SpectrumService {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.park.1.notify_all();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrum_stays_parked_when_not_playing_or_not_visible() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let wake: WakeFn = Arc::new(|| {});
        let playing = Arc::new(AtomicBool::new(false));
        let visible = Arc::new(AtomicBool::new(false));
        let park = Arc::new((Mutex::new(()), Condvar::new()));

        let svc = SpectrumService::spawn(tx, wake, playing.clone(), visible.clone(), park)
            .expect("spawn spectrum");
        assert!(!svc.is_active());

        // Visible alone without playing must stay inactive.
        svc.set_visible(true);
        assert!(!svc.is_active());

        // Playing alone without visible must stay inactive.
        svc.set_visible(false);
        playing.store(true, Ordering::Release);
        assert!(!svc.is_active());

        std::thread::sleep(Duration::from_millis(80));
        assert!(rx.try_recv().is_err(), "parked spectrum must emit zero events");
    }
}
