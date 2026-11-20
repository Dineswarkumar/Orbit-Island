//! Typed events, snapshots, and channels shared across core, services, and app.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryStatus {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClockStatus {
    pub time: String,
    pub date: String,
    pub millis_to_next_minute: u64,
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SystemSnapshot {
    pub monitor: Option<MonitorInfo>,
    pub light_theme: bool,
    pub animations: bool,
    pub battery: Option<BatteryStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreEvent {
    SystemChanged(SystemSnapshot),
    ClockTick(ClockStatus),
    SetHidden(bool),
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
