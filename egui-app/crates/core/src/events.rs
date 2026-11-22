//! Typed events, snapshots, and channels shared across core, services, and app.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Callback used by background services to wake the UI thread when state changes.
pub type WakeFn = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryStatus {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClockStatus {
    pub time: String,
    pub time_digits: String,
    pub ampm: String,
    pub date: String,
    pub full_date: String,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub weekday: u32,
    pub millis_to_next_minute: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusTimerState {
    pub label: String,
    pub total_secs: u64,
    pub deadline_unix_ms: Option<u64>,
    pub paused_remaining_secs: Option<u64>,
}

impl FocusTimerState {
    pub fn is_running(&self) -> bool {
        self.deadline_unix_ms.is_some() && self.paused_remaining_secs.is_none()
    }

    pub fn is_active(&self) -> bool {
        self.deadline_unix_ms.is_some() || self.paused_remaining_secs.unwrap_or(0) > 0
    }

    pub fn remaining_secs(&self, now_unix_ms: u64) -> u64 {
        if let Some(paused) = self.paused_remaining_secs {
            return paused;
        }
        if let Some(deadline) = self.deadline_unix_ms {
            let diff_ms = deadline.saturating_sub(now_unix_ms);
            return diff_ms.div_ceil(1000);
        }
        0
    }

    pub fn progress(&self, now_unix_ms: u64) -> f32 {
        if self.total_secs == 0 {
            return 0.0;
        }
        let rem = self.remaining_secs(now_unix_ms).min(self.total_secs);
        1.0 - (rem as f32 / self.total_secs as f32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl MonitorRect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    pub id: isize,
    pub bounds: MonitorRect,
    pub work: MonitorRect,
    pub dpi: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemSnapshot {
    pub monitor: Option<MonitorInfo>,
    pub light_theme: bool,
    pub animations: bool,
    pub transparency_enabled: bool,
    pub battery: Option<BatteryStatus>,
    pub wifi_connected: bool,
    pub bluetooth_enabled: bool,
}

impl Default for SystemSnapshot {
    fn default() -> Self {
        Self {
            monitor: None,
            light_theme: false,
            animations: true,
            transparency_enabled: true,
            battery: None,
            wifi_connected: false,
            bluetooth_enabled: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareMetrics {
    pub cpu_usage: f32,
    pub cpu_cores: u32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub ram_percentage: f32,
    pub disk_free_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_percentage: f32,
    pub battery_percentage: Option<u8>,
    pub battery_charging: Option<bool>,
    pub ac_power_connected: Option<bool>,
    pub wifi_connected: bool,
    pub bluetooth_enabled: bool,
    pub hotspot_active: bool,
    pub network_upload_speed_bps: u64,
    pub network_download_speed_bps: u64,
    pub gpu_name: String,
    pub gpu_usage: Option<f32>,
    pub screen_recording_active: bool,
    pub active_download_count: u32,
    pub last_completed_download: Option<String>,
    pub battery_saver_active: bool,
    pub os_transparency_enabled: bool,
}

impl Default for HardwareMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            cpu_cores: 1,
            ram_used_bytes: 0,
            ram_total_bytes: 0,
            ram_percentage: 0.0,
            disk_free_bytes: 0,
            disk_total_bytes: 0,
            disk_percentage: 0.0,
            battery_percentage: None,
            battery_charging: None,
            ac_power_connected: None,
            wifi_connected: false,
            bluetooth_enabled: false,
            hotspot_active: false,
            network_upload_speed_bps: 0,
            network_download_speed_bps: 0,
            gpu_name: "Graphics".to_string(),
            gpu_usage: Some(0.0),
            screen_recording_active: false,
            active_download_count: 0,
            last_completed_download: None,
            battery_saver_active: false,
            os_transparency_enabled: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCleanResult {
    pub freed_bytes: u64,
    pub freed_mb: f32,
    pub ram_used_bytes: u64,
    pub ram_percentage: f32,
    pub trimmed_processes: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaTrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub thumbnail_base64: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaTimelineInfo {
    pub position_secs: f64,
    pub duration_secs: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaControlsInfo {
    pub can_play: bool,
    pub can_pause: bool,
    pub can_skip_next: bool,
    pub can_skip_previous: bool,
    pub can_seek: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaSourceInfo {
    pub source_id: String,
    pub source_title: String,
    pub is_playing: bool,
    pub is_pinned: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaSessionSnapshot {
    pub source_id: String,
    pub source_title: String,
    pub playback_status: String,
    pub track: MediaTrackInfo,
    pub timeline: Option<MediaTimelineInfo>,
    pub controls: MediaControlsInfo,
    pub is_pinned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyState {
    pub camera_active: bool,
    pub camera_app: Option<String>,
    pub microphone_active: bool,
    pub microphone_app: Option<String>,
    #[serde(default)]
    pub location_active: bool,
    #[serde(default)]
    pub location_app: Option<String>,
    pub is_reliable: bool,
}

impl Default for PrivacyState {
    fn default() -> Self {
        Self {
            camera_active: false,
            camera_app: None,
            microphone_active: false,
            microphone_app: None,
            location_active: false,
            location_app: None,
            is_reliable: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub is_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItem {
    pub id: u32,
    pub app_name: String,
    pub app_id: String,
    pub title: String,
    pub body: String,
    pub timestamp_ms: u64,
    pub is_critical: bool,
    pub count: u32,
    pub actions: Vec<NotificationAction>,
    pub app_icon: Option<String>,
    #[serde(default)]
    pub underlying_ids: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSnapshot {
    pub access_status: String,
    pub active_notification: Option<NotificationItem>,
    pub notifications: Vec<NotificationItem>,
    pub overflow_count: usize,
    pub total_count: usize,
}

impl Default for NotificationSnapshot {
    fn default() -> Self {
        Self {
            access_status: "unspecified".to_string(),
            active_notification: None,
            notifications: Vec::new(),
            overflow_count: 0,
            total_count: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherGlance {
    pub temp_c: i32,
    pub temp_f: i32,
    pub condition_code: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderItem {
    pub id: String,
    pub title: String,
    pub due_timestamp_ms: u64,
    pub is_completed: bool,
    pub source: String,
    pub category: String,
    pub created_at: u64,
}

fn default_shelf_kind() -> String {
    "file".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub id: String,
    pub file_path: String,
    pub display_name: String,
    pub file_size: u64,
    pub extension: String,
    pub added_at: u64,
    pub exists: bool,
    pub is_managed_copy: bool,
    #[serde(default = "default_shelf_kind")]
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropFileResult {
    pub path: String,
    pub display_name: String,
    pub error: Option<String>,
    pub shelf_item: Option<ShelfItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoreEvent {
    SystemChanged(SystemSnapshot),
    ClockTick(ClockStatus),
    SetHidden(bool),
    FullscreenChanged(bool),
    HardwareUpdated(HardwareMetrics),
    MediaUpdated(Option<MediaSessionSnapshot>),
    PrivacyUpdated(PrivacyState),
    SpectrumUpdated(AudioSpectrumState),
    NotificationsUpdated(NotificationSnapshot),
    WeatherUpdated(WeatherGlance),
    RemindersUpdated(Vec<ReminderItem>),
    ShelfUpdated(Vec<ShelfItem>),
    FocusTimerUpdated(FocusTimerState),
    QuitRequested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiCommand {
    Quit,
}

pub fn event_channel() -> (
    crossbeam_channel::Sender<CoreEvent>,
    crossbeam_channel::Receiver<CoreEvent>,
) {
    crossbeam_channel::unbounded()
}
