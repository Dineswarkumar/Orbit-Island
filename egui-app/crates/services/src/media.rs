//! Windows GSMTC media session monitor and transport control service.

use crossbeam_channel::Sender;
use orbit_core::events::{
    AudioSpectrumState, CoreEvent, MediaControlsInfo, MediaSessionSnapshot, MediaSourceInfo,
    MediaTimelineInfo, MediaTrackInfo, WakeFn,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    pub fn new() -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        // SAFETY: initializes COM MTA for the current worker thread.
        unsafe {
            use windows_sys::Win32::Foundation::{RPC_E_CHANGED_MODE, S_FALSE, S_OK};
            use windows_sys::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
            let hr = CoInitializeEx(std::ptr::null_mut(), COINIT_MULTITHREADED as u32);
            if hr == S_OK || hr == S_FALSE {
                Ok(Self { initialized: true })
            } else if hr == RPC_E_CHANGED_MODE {
                Ok(Self { initialized: false })
            } else {
                Err(format!("CoInitializeEx failed with HRESULT 0x{:08X}", hr as u32))
            }
        }
        #[cfg(not(target_os = "windows"))]
        Ok(Self { initialized: false })
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        if self.initialized {
            // SAFETY: balanced CoUninitialize for the CoInitializeEx above.
            unsafe {
                windows_sys::Win32::System::Com::CoUninitialize();
            }
        }
    }
}

pub fn format_source_title(id: &str) -> String {
    let lower = id.to_lowercase();
    if lower.contains("spotify") {
        "Spotify".to_string()
    } else if lower.contains("instagram") {
        "Instagram".to_string()
    } else if lower.contains("whatsapp") {
        "WhatsApp".to_string()
    } else if lower.contains("telegram") {
        "Telegram".to_string()
    } else if lower.contains("discord") {
        "Discord".to_string()
    } else if lower.contains("netflix") {
        "Netflix".to_string()
    } else if lower.contains("tidal") {
        "Tidal".to_string()
    } else if lower.contains("deezer") {
        "Deezer".to_string()
    } else if lower.contains("amazonmusic") || lower.contains("amazon music") {
        "Amazon Music".to_string()
    } else if lower.contains("youtube") {
        "YouTube".to_string()
    } else if lower.contains("chrome") {
        "Google Chrome".to_string()
    } else if lower.contains("edge") {
        "Microsoft Edge".to_string()
    } else if lower.contains("brave") {
        "Brave".to_string()
    } else if lower.contains("firefox") {
        "Firefox".to_string()
    } else if lower.contains("opera") {
        "Opera".to_string()
    } else if lower.contains("vivaldi") {
        "Vivaldi".to_string()
    } else if lower.contains("zunemusic")
        || lower.contains("microsoft.zunemusic")
        || lower.contains("mediaplayer")
    {
        "Media Player".to_string()
    } else if lower.contains("vlc") {
        "VLC".to_string()
    } else if lower.contains("apple") || lower.contains("itunes") {
        "Apple Music".to_string()
    } else {
        let candidate = id.strip_suffix(".exe").unwrap_or(id);
        let clean_id = candidate.split('!').next().unwrap_or(candidate);
        let clean_id = clean_id.split('_').next().unwrap_or(clean_id);
        let clean_id = clean_id.rsplit('.').next().unwrap_or(clean_id).trim();
        if clean_id.is_empty() {
            "Media Source".to_string()
        } else {
            clean_id.to_string()
        }
    }
}

#[cfg(target_os = "windows")]
mod win {
    use super::*;
    use base64::Engine;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionMediaProperties,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Storage::Streams::DataReader;

    static THUMBNAIL_CACHE: Mutex<Option<(String, Option<String>)>> = Mutex::new(None);

    fn load_thumbnail_from_stream(
        props: &GlobalSystemMediaTransportControlsSessionMediaProperties,
    ) -> Option<String> {
        let thumb_ref = props.Thumbnail().ok()?;
        let stream_op = thumb_ref.OpenReadAsync().ok()?;
        let stream = stream_op.join().ok()?;
        let size = stream.Size().unwrap_or(0);
        if size == 0 || size >= 5 * 1024 * 1024 {
            return None;
        }
        let reader = DataReader::CreateDataReader(&stream).ok()?;
        reader.LoadAsync(size as u32).ok()?.join().ok()?;
        let mut buffer = vec![0u8; size as usize];
        reader.ReadBytes(&mut buffer).ok()?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&buffer);
        Some(format!("data:image/jpeg;base64,{b64}"))
    }

    pub type SessionManagerCache = Option<GlobalSystemMediaTransportControlsSessionManager>;

    pub fn get_current_media_session(pinned_source: Option<&str>) -> Option<MediaSessionSnapshot> {
        let mut cache = None;
        get_current_media_session_cached(&mut cache, pinned_source)
    }

    pub fn get_current_media_session_cached(
        manager_cache: &mut SessionManagerCache,
        pinned_source: Option<&str>,
    ) -> Option<MediaSessionSnapshot> {
        if manager_cache.is_none() {
            let manager_async =
                GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?;
            *manager_cache = manager_async.join().ok();
        }
        let manager = manager_cache.as_ref()?;
        let Ok(sessions) = manager.GetSessions() else {
            *manager_cache = None;
            return None;
        };
        let mut all_sessions: Vec<GlobalSystemMediaTransportControlsSession> = Vec::new();
        for i in 0..sessions.Size().unwrap_or(0) {
            if let Ok(sess) = sessions.GetAt(i) {
                all_sessions.push(sess);
            }
        }
        if all_sessions.is_empty() {
            return None;
        }

        let target_session = if let Some(pinned) = pinned_source {
            all_sessions
                .iter()
                .find(|s| {
                    s.SourceAppUserModelId()
                        .map(|id| id.to_string_lossy().eq_ignore_ascii_case(pinned))
                        .unwrap_or(false)
                })
                .cloned()
        } else {
            None
        }
        .or_else(|| {
            all_sessions
                .iter()
                .find(|s| {
                    s.GetPlaybackInfo()
                        .and_then(|info| info.PlaybackStatus())
                        .map(|st| st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                        .unwrap_or(false)
                })
                .cloned()
        })
        .or_else(|| manager.GetCurrentSession().ok())
        .or_else(|| all_sessions.first().cloned())?;

        let source_id = target_session
            .SourceAppUserModelId()
            .map(|s| s.to_string_lossy())
            .unwrap_or_else(|_| "Unknown Player".to_string());

        let playback_info = target_session.GetPlaybackInfo().ok()?;
        let status_enum = playback_info.PlaybackStatus().ok()?;
        let playback_status = match status_enum {
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing => "playing",
            GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused => "paused",
            _ => "stopped",
        }
        .to_string();

        let ctrl_flags = playback_info.Controls().ok();
        let controls = MediaControlsInfo {
            can_play: ctrl_flags
                .as_ref()
                .and_then(|c| c.IsPlayEnabled().ok())
                .unwrap_or(false),
            can_pause: ctrl_flags
                .as_ref()
                .and_then(|c| c.IsPauseEnabled().ok())
                .unwrap_or(false),
            can_skip_next: ctrl_flags
                .as_ref()
                .and_then(|c| c.IsNextEnabled().ok())
                .unwrap_or(false),
            can_skip_previous: ctrl_flags
                .as_ref()
                .and_then(|c| c.IsPreviousEnabled().ok())
                .unwrap_or(false),
            can_seek: ctrl_flags
                .as_ref()
                .and_then(|c| c.IsPlaybackPositionEnabled().ok())
                .unwrap_or(false),
        };

        let props = target_session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
        let title = props.Title().map(|s| s.to_string_lossy()).unwrap_or_default();
        let artist = props.Artist().map(|s| s.to_string_lossy()).unwrap_or_default();
        let album = props
            .AlbumTitle()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default();

        let cache_key = format!("{source_id}:{title}:{artist}:{album}");
        let thumbnail_base64 = {
            let mut cache = THUMBNAIL_CACHE
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some((ref key, ref thumb)) = *cache {
                if key == &cache_key {
                    thumb.clone()
                } else {
                    let loaded = load_thumbnail_from_stream(&props);
                    *cache = Some((cache_key, loaded.clone()));
                    loaded
                }
            } else {
                let loaded = load_thumbnail_from_stream(&props);
                *cache = Some((cache_key, loaded.clone()));
                loaded
            }
        };

        let timeline = target_session.GetTimelineProperties().ok().and_then(|tl| {
            let start = tl.StartTime().map(|t| t.Duration).unwrap_or(0);
            let end = tl.EndTime().map(|t| t.Duration).unwrap_or(0);
            let pos = tl.Position().map(|t| t.Duration).unwrap_or(0);
            let duration_ticks = end - start;
            const MAX_DURATION_TICKS: i64 = 48 * 3600 * 10_000_000;
            if duration_ticks > 0 && duration_ticks <= MAX_DURATION_TICKS {
                let duration_secs = duration_ticks as f64 / 10_000_000.0;
                let mut pos_ticks = pos - start;
                if playback_status == "playing"
                    && let Ok(last_updated_dt) = tl.LastUpdatedTime()
                {
                    let last_updated = last_updated_dt.UniversalTime;
                    if last_updated > 0 {
                        let unix_dur = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        let now_windows_ticks = (unix_dur.as_secs() as i64) * 10_000_000
                            + (unix_dur.subsec_nanos() as i64 / 100)
                            + 116_444_736_000_000_000i64;
                        if now_windows_ticks >= last_updated {
                            let elapsed = now_windows_ticks - last_updated;
                            if elapsed < 3600 * 10_000_000 {
                                pos_ticks += elapsed;
                            }
                        }
                    }
                }
                let clamped_pos_ticks = pos_ticks.clamp(0, duration_ticks);
                Some(MediaTimelineInfo {
                    position_secs: clamped_pos_ticks as f64 / 10_000_000.0,
                    duration_secs,
                })
            } else {
                None
            }
        });

        let is_pinned = pinned_source
            .map(|p| p.eq_ignore_ascii_case(&source_id))
            .unwrap_or(false);
        let source_title = format_source_title(&source_id);

        Some(MediaSessionSnapshot {
            source_id,
            source_title,
            playback_status,
            track: MediaTrackInfo {
                title,
                artist,
                album,
                thumbnail_base64,
            },
            timeline,
            controls,
            is_pinned,
        })
    }

    pub fn get_available_media_sources(pinned_source: Option<&str>) -> Vec<MediaSourceInfo> {
        let mut result = Vec::new();
        let Ok(manager_async) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        else {
            return result;
        };
        let Ok(manager) = manager_async.join() else {
            return result;
        };
        let Ok(sessions) = manager.GetSessions() else {
            return result;
        };

        for i in 0..sessions.Size().unwrap_or(0) {
            if let Ok(sess) = sessions.GetAt(i)
                && let Ok(id_hstring) = sess.SourceAppUserModelId()
            {
                let source_id = id_hstring.to_string_lossy();
                if source_id.is_empty() || result.iter().any(|r: &MediaSourceInfo| r.source_id == source_id)
                {
                    continue;
                }
                let source_title = format_source_title(&source_id);
                let is_playing = sess
                    .GetPlaybackInfo()
                    .and_then(|info| info.PlaybackStatus())
                    .map(|st| st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                    .unwrap_or(false);
                let is_pinned = pinned_source
                    .map(|p| p.eq_ignore_ascii_case(&source_id))
                    .unwrap_or(false);
                result.push(MediaSourceInfo {
                    source_id,
                    source_title,
                    is_playing,
                    is_pinned,
                });
            }
        }
        result
    }

    fn resolve_target_session(
        pinned_source: Option<&str>,
    ) -> Result<GlobalSystemMediaTransportControlsSession, String> {
        let manager_async = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .map_err(|e| e.to_string())?;
        let manager = manager_async.join().map_err(|e| e.to_string())?;
        let sessions = manager.GetSessions().map_err(|e| e.to_string())?;
        let mut target = None;
        if let Some(pinned) = pinned_source {
            for i in 0..sessions.Size().unwrap_or(0) {
                if let Ok(sess) = sessions.GetAt(i)
                    && let Ok(id) = sess.SourceAppUserModelId()
                    && id.to_string_lossy().eq_ignore_ascii_case(pinned)
                {
                    target = Some(sess);
                    break;
                }
            }
        }
        target
            .or_else(|| manager.GetCurrentSession().ok())
            .ok_or_else(|| "No active media session found".to_string())
    }

    pub fn send_media_command(command: &str, pinned_source: Option<&str>) -> Result<(), String> {
        let target = resolve_target_session(pinned_source)?;
        match command {
            "play" => target
                .TryPlayAsync()
                .map_err(|e| e.to_string())?
                .join()
                .map_err(|e| e.to_string())?,
            "pause" => target
                .TryPauseAsync()
                .map_err(|e| e.to_string())?
                .join()
                .map_err(|e| e.to_string())?,
            "toggle" => target
                .TryTogglePlayPauseAsync()
                .map_err(|e| e.to_string())?
                .join()
                .map_err(|e| e.to_string())?,
            "next" => target
                .TrySkipNextAsync()
                .map_err(|e| e.to_string())?
                .join()
                .map_err(|e| e.to_string())?,
            "previous" => target
                .TrySkipPreviousAsync()
                .map_err(|e| e.to_string())?
                .join()
                .map_err(|e| e.to_string())?,
            _ => return Err(format!("Unsupported media command: {command}")),
        };
        Ok(())
    }

    pub fn seek_media(position_secs: f64, pinned_source: Option<&str>) -> Result<(), String> {
        let target = resolve_target_session(pinned_source)?;
        let ticks = (position_secs * 10_000_000.0) as i64;
        target
            .TryChangePlaybackPositionAsync(ticks)
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn sample_audio_spectrum_meter(
        prev_peak: &mut f32,
        smooth_low: &mut f32,
        phase: &mut f32,
    ) -> AudioSpectrumState {
        use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
        use windows::Win32::Media::Audio::{
            IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
        };
        use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};

        let (raw_peak, left, right) = unsafe {
            let Ok(enumerator) =
                CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
            else {
                return AudioSpectrumState::default();
            };
            let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) else {
                return AudioSpectrumState::default();
            };
            let Ok(meter) = device.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None) else {
                return AudioSpectrumState::default();
            };
            let peak = meter.GetPeakValue().unwrap_or(0.0).clamp(0.0, 1.0);
            let count = meter.GetMeteringChannelCount().unwrap_or(0);
            if count >= 2 {
                // Fixed stack buffer avoids per-frame heap allocation at 30 fps.
                let mut ch = [0.0f32; 8];
                let n = (count as usize).min(ch.len());
                if meter.GetChannelsPeakValues(&mut ch[..n]).is_ok() {
                    (peak, ch[0].clamp(0.0, 1.0), ch[1].clamp(0.0, 1.0))
                } else {
                    (peak, peak, peak)
                }
            } else {
                (peak, peak, peak)
            }
        };

        if raw_peak < 0.004 {
            *prev_peak = 0.0;
            *smooth_low *= 0.55;
            if *smooth_low < 0.004 {
                *smooth_low = 0.0;
                return AudioSpectrumState::default();
            }
        }

        let level = raw_peak.powf(0.62).clamp(0.0, 1.0);
        *smooth_low = (*smooth_low * 0.68 + level * 0.32).clamp(0.0, 1.0);
        let transient = ((level - *prev_peak).abs() * 2.3).clamp(0.0, 1.0);
        *prev_peak = level;

        let stereo_diff = (left - right).abs().powf(0.65).clamp(0.0, 1.0);
        let bass = *smooth_low;
        let mid = (level * 0.78 + stereo_diff * 0.42).clamp(0.0, 1.0);
        let treble = (level * 0.52 + transient * 0.72).clamp(0.0, 1.0);

        *phase = (*phase + 0.65 + level * 0.95) % std::f32::consts::TAU;
        let p = *phase;

        let compact_bars = [
            (bass * (0.78 + 0.22 * p.sin().abs())).clamp(0.0, 1.0),
            (mid * (0.75 + 0.25 * (p + 1.3).sin().abs())).clamp(0.0, 1.0),
            (treble * (0.74 + 0.26 * (p + 2.5).sin().abs())).clamp(0.0, 1.0),
            (((bass + mid) * 0.5) * (0.76 + 0.24 * (p + 3.8).sin().abs())).clamp(0.0, 1.0),
        ];

        let mut wave_bars = [0.0f32; 20];
        for (idx, bar) in wave_bars.iter_mut().enumerate() {
            let base = if idx < 6 {
                bass
            } else if idx < 14 {
                mid
            } else {
                treble
            };
            let harmonic = ((p + (idx as f32) * 0.62).sin() * 0.5 + 0.5) * 0.35 + 0.65;
            *bar = (base * harmonic).clamp(0.0, 1.0);
        }

        AudioSpectrumState {
            peak: level,
            compact_bars,
            wave_bars,
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod win {
    use super::*;
    pub type SessionManagerCache = Option<()>;
    pub fn get_current_media_session(_pinned_source: Option<&str>) -> Option<MediaSessionSnapshot> {
        None
    }
    pub fn get_current_media_session_cached(
        _manager_cache: &mut SessionManagerCache,
        _pinned_source: Option<&str>,
    ) -> Option<MediaSessionSnapshot> {
        None
    }
    pub fn get_available_media_sources(_pinned_source: Option<&str>) -> Vec<MediaSourceInfo> {
        Vec::new()
    }
    pub fn send_media_command(_command: &str, _pinned_source: Option<&str>) -> Result<(), String> {
        Err("Media sessions unsupported on this platform".to_string())
    }
    pub fn seek_media(_position_secs: f64, _pinned_source: Option<&str>) -> Result<(), String> {
        Err("Media sessions unsupported on this platform".to_string())
    }
    pub fn sample_audio_spectrum_meter(
        _prev_peak: &mut f32,
        _smooth_low: &mut f32,
        _phase: &mut f32,
    ) -> AudioSpectrumState {
        AudioSpectrumState::default()
    }
}

pub use win::{
    get_available_media_sources, get_current_media_session, sample_audio_spectrum_meter, seek_media,
    send_media_command,
};

pub struct MediaService {
    pinned_source: Arc<Mutex<Option<String>>>,
    playing_flag: Arc<AtomicBool>,
    spectrum_park: Arc<(Mutex<()>, Condvar)>,
    last_snapshot: Arc<Mutex<Option<MediaSessionSnapshot>>>,
    stop: Arc<AtomicBool>,
    park: Arc<(Mutex<()>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl MediaService {
    pub fn spawn(
        tx: Sender<CoreEvent>,
        wake: WakeFn,
        playing_flag: Arc<AtomicBool>,
        spectrum_park: Arc<(Mutex<()>, Condvar)>,
    ) -> std::io::Result<Self> {
        let pinned_source = Arc::new(Mutex::new(None::<String>));
        let last_snapshot = Arc::new(Mutex::new(None::<MediaSessionSnapshot>));
        let stop = Arc::new(AtomicBool::new(false));
        let park = Arc::new((Mutex::new(()), Condvar::new()));

        let pin_bg = pinned_source.clone();
        let play_bg = playing_flag.clone();
        let spec_bg = spectrum_park.clone();
        let snap_bg = last_snapshot.clone();
        let stop_bg = stop.clone();
        let park_bg = park.clone();

        let thread = std::thread::Builder::new()
            .name("orbit-media".into())
            .spawn(move || {
                let _com = match ComGuard::new() {
                    Ok(g) => Some(g),
                    Err(e) => {
                        tracing::error!("COM init failed on orbit-media: {e}");
                        None
                    }
                };
                let mut manager_cache: win::SessionManagerCache = None;
                let mut last_source_id = String::new();
                let mut last_title = String::new();
                let mut last_status = String::new();
                let mut last_duration = 0.0f64;
                let mut last_can_seek = false;
                let mut last_pos = 0.0f64;
                let mut last_pinned = false;

                while !stop_bg.load(Ordering::Relaxed) {
                    let pin = pin_bg
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    let session = win::get_current_media_session_cached(&mut manager_cache, pin.as_deref());
                    let is_playing = session
                        .as_ref()
                        .map(|s| s.playback_status == "playing")
                        .unwrap_or(false);

                    let prev_playing = play_bg.swap(is_playing, Ordering::AcqRel);
                    if prev_playing != is_playing {
                        spec_bg.1.notify_all();
                    }

                    let cur_source_id = session
                        .as_ref()
                        .map(|s| s.source_id.as_str())
                        .unwrap_or("")
                        .to_string();
                    let cur_title = session
                        .as_ref()
                        .map(|s| s.track.title.as_str())
                        .unwrap_or("")
                        .to_string();
                    let cur_status = session
                        .as_ref()
                        .map(|s| s.playback_status.as_str())
                        .unwrap_or("")
                        .to_string();
                    let cur_duration = session
                        .as_ref()
                        .and_then(|s| s.timeline.as_ref())
                        .map(|t| t.duration_secs)
                        .unwrap_or(0.0);
                    let cur_can_seek = session
                        .as_ref()
                        .map(|s| s.controls.can_seek)
                        .unwrap_or(false);
                    let cur_pos = session
                        .as_ref()
                        .and_then(|s| s.timeline.as_ref())
                        .map(|t| t.position_secs)
                        .unwrap_or(0.0);
                    let cur_pinned = session.as_ref().map(|s| s.is_pinned).unwrap_or(false);

                    let changed = cur_source_id != last_source_id
                        || cur_title != last_title
                        || cur_status != last_status
                        || (cur_duration - last_duration).abs() >= 0.5
                        || cur_can_seek != last_can_seek
                        || cur_pinned != last_pinned
                        || (is_playing && (cur_pos - last_pos).abs() >= 0.8);

                    if changed {
                        last_source_id = cur_source_id;
                        last_title = cur_title;
                        last_status = cur_status;
                        last_duration = cur_duration;
                        last_can_seek = cur_can_seek;
                        last_pos = cur_pos;
                        last_pinned = cur_pinned;
                        {
                            let mut g = snap_bg.lock().unwrap_or_else(|e| e.into_inner());
                            *g = session.clone();
                        }
                        if tx.send(CoreEvent::MediaUpdated(session)).is_err() {
                            break;
                        }
                        wake();
                    }

                    let wait = if is_playing {
                        Duration::from_millis(1000)
                    } else {
                        Duration::from_millis(2000)
                    };
                    let guard = park_bg.0.lock().unwrap_or_else(|e| e.into_inner());
                    if !stop_bg.load(Ordering::Relaxed) {
                        let _ = park_bg.1.wait_timeout(guard, wait);
                    }
                }
            })?;

        Ok(Self {
            pinned_source,
            playing_flag,
            spectrum_park,
            last_snapshot,
            stop,
            park,
            thread: Some(thread),
        })
    }

    pub fn current_snapshot(&self) -> Option<MediaSessionSnapshot> {
        self.last_snapshot
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn get_sources(&self) -> Vec<MediaSourceInfo> {
        let pin = self
            .pinned_source
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let _com = ComGuard::new();
        get_available_media_sources(pin.as_deref())
    }

    pub fn pin_source(&self, source_id: Option<String>) -> Option<MediaSessionSnapshot> {
        {
            let mut g = self.pinned_source.lock().unwrap_or_else(|e| e.into_inner());
            *g = source_id.clone();
        }
        let _com = ComGuard::new();
        let updated = get_current_media_session(source_id.as_deref());
        self.park.1.notify_all();
        updated
    }

    pub fn send_command(&self, command: &str) -> Result<Option<MediaSessionSnapshot>, String> {
        let pin = self
            .pinned_source
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let _com = ComGuard::new()?;
        send_media_command(command, pin.as_deref())?;
        std::thread::sleep(Duration::from_millis(80));
        let updated = get_current_media_session(pin.as_deref());
        let is_playing = updated
            .as_ref()
            .map(|s| s.playback_status == "playing")
            .unwrap_or(false);
        self.playing_flag.store(is_playing, Ordering::Release);
        self.spectrum_park.1.notify_all();
        self.park.1.notify_all();
        Ok(updated)
    }

    pub fn seek(&self, position_secs: f64) -> Result<Option<MediaSessionSnapshot>, String> {
        let pin = self
            .pinned_source
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let _com = ComGuard::new()?;
        seek_media(position_secs, pin.as_deref())?;
        std::thread::sleep(Duration::from_millis(50));
        let updated = get_current_media_session(pin.as_deref());
        self.park.1.notify_all();
        Ok(updated)
    }
}

impl Drop for MediaService {
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
    fn format_source_title_resolves_friendly_names() {
        assert_eq!(format_source_title("spotify.exe"), "Spotify");
        assert_eq!(
            format_source_title("FACEBOOK.INSTAGRAMBETA_8XX8RVFYW5NNT!APP2"),
            "Instagram"
        );
        assert_eq!(format_source_title("chrome.exe"), "Google Chrome");
        assert_eq!(format_source_title("customplayer.exe"), "customplayer");
    }
}
