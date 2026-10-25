use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaTrack {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub thumbnail_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaTimeline {
    pub position_secs: f64,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaControls {
    pub can_play: bool,
    pub can_pause: bool,
    pub can_skip_next: bool,
    pub can_skip_previous: bool,
    pub can_seek: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSourceInfo {
    pub source_id: String,
    pub source_title: String,
    pub is_playing: bool,
    pub is_pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSessionSnapshot {
    pub source_id: String,
    pub source_title: String,
    pub playback_status: String, // "playing", "paused", "stopped"
    pub track: MediaTrack,
    pub timeline: Option<MediaTimeline>,
    pub controls: MediaControls,
    pub is_pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSpectrumState {
    pub peak: f32,
    pub compact_bars: [f32; 4],
    pub wave_bars: [f32; 20],
}

impl Default for AudioSpectrumState {
    fn default() -> Self {
        Self {
            peak: 0.0,
            compact_bars: [0.0; 4],
            wave_bars: [0.0; 20],
        }
    }
}

fn format_source_title(id: &str) -> String {
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
    } else if lower.contains("zunemusic") || lower.contains("microsoft.zunemusic") || lower.contains("mediaplayer") {
        "Media Player".to_string()
    } else if lower.contains("vlc") {
        "VLC".to_string()
    } else if lower.contains("apple") || lower.contains("itunes") {
        "Apple Music".to_string()
    } else {
        let candidate = id.strip_suffix(".exe").unwrap_or(id);
        // Strip Windows AppX/AUMID packaging boilerplate: Publisher.AppName_hash!App
        let clean_id = if let Some(idx) = candidate.find('!') {
            &candidate[..idx]
        } else {
            candidate
        };
        let clean_id = if let Some(idx) = clean_id.find('_') {
            &clean_id[..idx]
        } else {
            clean_id
        };
        let clean_id = if let Some(idx) = clean_id.rfind('.') {
            &clean_id[idx + 1..]
        } else {
            clean_id
        };
        let res = clean_id.trim();
        if res.is_empty() {
            "Media Source".to_string()
        } else {
            res.to_string()
        }
    }
}

use std::sync::Mutex;
use std::time::Duration;

pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    pub fn new() -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        unsafe {
            use windows_sys::Win32::Foundation::{RPC_E_CHANGED_MODE, S_FALSE, S_OK};
            use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
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
            unsafe {
                windows_sys::Win32::System::Com::CoUninitialize();
            }
        }
    }
}

pub async fn run_blocking_media_task<F, T>(timeout: Duration, f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    let join_handle = tokio::task::spawn_blocking(move || {
        let _com = ComGuard::new()?;
        f()
    });

    match tokio::time::timeout(timeout, join_handle).await {
        Ok(Ok(task_result)) => task_result,
        Ok(Err(join_err)) => Err(format!("Worker thread panicked or failed: {join_err}")),
        Err(_elapsed) => Err(format!("Media operation timed out after {}ms", timeout.as_millis())),
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

    fn load_thumbnail_from_stream(props: &GlobalSystemMediaTransportControlsSessionMediaProperties) -> Option<String> {
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

    pub fn get_current_media_session(pinned_source: Option<&str>) -> Option<MediaSessionSnapshot> {
        let manager_async = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?;
        let manager = manager_async.join().ok()?;

        let sessions = manager.GetSessions().ok()?;
        let mut all_sessions: Vec<GlobalSystemMediaTransportControlsSession> = Vec::new();
        for i in 0..sessions.Size().unwrap_or(0) {
            if let Ok(sess) = sessions.GetAt(i) {
                all_sessions.push(sess);
            }
        }

        if all_sessions.is_empty() {
            return None;
        }

        // Priority logic:
        // 1. Pinned source if present and active
        // 2. Playing session
        // 3. Current active session reported by Windows
        // 4. Any paused session
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
        };

        let target_session = target_session.or_else(|| {
            // Find a playing session
            all_sessions
                .iter()
                .find(|s| {
                    s.GetPlaybackInfo()
                        .and_then(|info| info.PlaybackStatus())
                        .map(|st| st == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
                        .unwrap_or(false)
                })
                .cloned()
        }).or_else(|| {
            // Or default to manager's current session
            manager.GetCurrentSession().ok()
        }).or_else(|| {
            // Or take first available
            all_sessions.first().cloned()
        })?;

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
        }.to_string();

        let ctrl_flags = playback_info.Controls().ok();
        let controls = MediaControls {
            can_play: ctrl_flags.as_ref().and_then(|c| c.IsPlayEnabled().ok()).unwrap_or(false),
            can_pause: ctrl_flags.as_ref().and_then(|c| c.IsPauseEnabled().ok()).unwrap_or(false),
            can_skip_next: ctrl_flags.as_ref().and_then(|c| c.IsNextEnabled().ok()).unwrap_or(false),
            can_skip_previous: ctrl_flags.as_ref().and_then(|c| c.IsPreviousEnabled().ok()).unwrap_or(false),
            can_seek: ctrl_flags.as_ref().and_then(|c| c.IsPlaybackPositionEnabled().ok()).unwrap_or(false),
        };

        let props = target_session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
        let title = props.Title().map(|s| s.to_string_lossy()).unwrap_or_default();
        let artist = props.Artist().map(|s| s.to_string_lossy()).unwrap_or_default();
        let album = props.AlbumTitle().map(|s| s.to_string_lossy()).unwrap_or_default();

        let cache_key = format!("{source_id}:{title}:{artist}:{album}");
        let thumbnail_base64 = {
            let mut cache = THUMBNAIL_CACHE.lock().unwrap();
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

            // Maximum realistic media duration: 48 hours in 100ns units (48 * 3600 * 10_000_000)
            const MAX_DURATION_TICKS: i64 = 48 * 3600 * 10_000_000;
            if duration_ticks > 0 && duration_ticks <= MAX_DURATION_TICKS {
                let duration_secs = duration_ticks as f64 / 10_000_000.0;
                let mut pos_ticks = pos - start;

                // For playing sessions, extrapolate position using LastUpdatedTime
                if playback_status == "playing" {
                    if let Ok(last_updated_dt) = tl.LastUpdatedTime() {
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
                                // Ignore stale intervals greater than 1 hour to prevent runaway jumps
                                if elapsed < 3600 * 10_000_000 {
                                    pos_ticks += elapsed;
                                }
                            }
                        }
                    }
                }

                let clamped_pos_ticks = pos_ticks.clamp(0, duration_ticks);
                let position_secs = clamped_pos_ticks as f64 / 10_000_000.0;
                Some(MediaTimeline {
                    position_secs,
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
            track: MediaTrack {
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
        let manager_async = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
            Ok(op) => op,
            Err(_) => return result,
        };
        let manager = match manager_async.join() {
            Ok(m) => m,
            Err(_) => return result,
        };
        let sessions = match manager.GetSessions() {
            Ok(s) => s,
            Err(_) => return result,
        };

        for i in 0..sessions.Size().unwrap_or(0) {
            if let Ok(sess) = sessions.GetAt(i) {
                if let Ok(id_hstring) = sess.SourceAppUserModelId() {
                    let source_id = id_hstring.to_string_lossy();
                    if source_id.is_empty() || result.iter().any(|r: &MediaSourceInfo| r.source_id == source_id) {
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
        }
        result
    }

    pub fn send_media_command(command: &str, pinned_source: Option<&str>) -> Result<(), String> {
        let manager_async = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .map_err(|e| e.to_string())?;
        let manager = manager_async.join().map_err(|e| e.to_string())?;

        let sessions = manager.GetSessions().map_err(|e| e.to_string())?;
        let mut target = None;

        if let Some(pinned) = pinned_source {
            for i in 0..sessions.Size().unwrap_or(0) {
                if let Ok(sess) = sessions.GetAt(i) {
                    if let Ok(id) = sess.SourceAppUserModelId() {
                        if id.to_string_lossy().eq_ignore_ascii_case(pinned) {
                            target = Some(sess);
                            break;
                        }
                    }
                }
            }
        }

        let target = target.or_else(|| manager.GetCurrentSession().ok()).ok_or_else(|| {
            "No active media session found".to_string()
        })?;

        match command {
            "play" => {
                let op = target.TryPlayAsync().map_err(|e| e.to_string())?;
                op.join().map_err(|e| e.to_string())?;
            }
            "pause" => {
                let op = target.TryPauseAsync().map_err(|e| e.to_string())?;
                op.join().map_err(|e| e.to_string())?;
            }
            "toggle" => {
                let op = target.TryTogglePlayPauseAsync().map_err(|e| e.to_string())?;
                op.join().map_err(|e| e.to_string())?;
            }
            "next" => {
                let op = target.TrySkipNextAsync().map_err(|e| e.to_string())?;
                op.join().map_err(|e| e.to_string())?;
            }
            "previous" => {
                let op = target.TrySkipPreviousAsync().map_err(|e| e.to_string())?;
                op.join().map_err(|e| e.to_string())?;
            }
            _ => return Err(format!("Unsupported media command: {command}")),
        }
        Ok(())
    }

    pub fn seek_media(position_secs: f64, pinned_source: Option<&str>) -> Result<(), String> {
        let manager_async = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .map_err(|e| e.to_string())?;
        let manager = manager_async.join().map_err(|e| e.to_string())?;

        let sessions = manager.GetSessions().map_err(|e| e.to_string())?;
        let mut target = None;

        if let Some(pinned) = pinned_source {
            for i in 0..sessions.Size().unwrap_or(0) {
                if let Ok(sess) = sessions.GetAt(i) {
                    if let Ok(id) = sess.SourceAppUserModelId() {
                        if id.to_string_lossy().eq_ignore_ascii_case(pinned) {
                            target = Some(sess);
                            break;
                        }
                    }
                }
            }
        }

        let target = target.or_else(|| manager.GetCurrentSession().ok()).ok_or_else(|| {
            "No active media session found".to_string()
        })?;

        let ticks = (position_secs * 10_000_000.0) as i64;
        let op = target.TryChangePlaybackPositionAsync(ticks).map_err(|e| e.to_string())?;
        op.join().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn sample_audio_spectrum_meter(
        prev_peak: &mut f32,
        smooth_low: &mut f32,
        phase: &mut f32,
    ) -> AudioSpectrumState {
        use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
        use windows::Win32::Media::Audio::{
            eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
        };
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

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
                let mut ch = vec![0.0f32; count as usize];
                if meter.GetChannelsPeakValues(&mut ch).is_ok() {
                    (peak, ch[0].clamp(0.0, 1.0), ch[1].clamp(0.0, 1.0))
                } else {
                    (peak, peak, peak)
                }
            } else {
                (peak, peak, peak)
            }
        };

        // When audio output is silent / muted / 0 frequency, keep all bars strictly at 0.0 (no fake rising!)
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
            (bass * (0.78 + 0.22 * (p).sin().abs())).clamp(0.0, 1.0),
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

    pub fn get_current_media_session(_pinned_source: Option<&str>) -> Option<MediaSessionSnapshot> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_source_title_resolves_friendly_names_and_cleans_packages() {
        assert_eq!(format_source_title("spotify.exe"), "Spotify");
        assert_eq!(
            format_source_title("FACEBOOK.INSTAGRAMBETA_8XX8RVFYW5NNT!APP2"),
            "Instagram"
        );
        assert_eq!(format_source_title("WhatsApp.exe"), "WhatsApp");
        assert_eq!(format_source_title("Telegram.exe"), "Telegram");
        assert_eq!(format_source_title("chrome.exe"), "Google Chrome");
        assert_eq!(format_source_title("msedge.exe"), "Microsoft Edge");
        assert_eq!(format_source_title("brave.exe"), "Brave");
        assert_eq!(format_source_title("discord.exe"), "Discord");
        assert_eq!(format_source_title("netflix"), "Netflix");
        assert_eq!(format_source_title("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), "Media Player");
        assert_eq!(format_source_title("SomeCompany.AudioPlayer_12345!App"), "AudioPlayer");
        assert_eq!(format_source_title("customplayer.exe"), "customplayer");
    }
}

