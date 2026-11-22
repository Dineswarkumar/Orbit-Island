//! Central state store with versioned snapshots.

use crate::config::Config;
use crate::events::{
    AudioSpectrumState, ClockStatus, CoreEvent, FocusTimerState, HardwareMetrics,
    MediaSessionSnapshot, NotificationSnapshot, PrivacyState, ReminderItem, ShelfItem,
    SystemSnapshot, WeatherGlance,
};

#[derive(Clone, Debug, PartialEq)]
pub struct AppSnapshot {
    pub version: u64,
    pub config: Config,
    pub system: SystemSnapshot,
    pub clock: ClockStatus,
    pub hidden: bool,
    pub fullscreen_suppressed: bool,
    pub hardware: HardwareMetrics,
    pub media: Option<MediaSessionSnapshot>,
    pub privacy: PrivacyState,
    pub spectrum: AudioSpectrumState,
    pub notifications: NotificationSnapshot,
    pub weather: Option<WeatherGlance>,
    pub reminders: Vec<ReminderItem>,
    pub shelf: Vec<ShelfItem>,
    pub focus_timer: FocusTimerState,
}

pub struct AppStore {
    snapshot: AppSnapshot,
}

impl AppStore {
    pub fn new(config: Config, system: SystemSnapshot, clock: ClockStatus, hidden: bool) -> Self {
        Self {
            snapshot: AppSnapshot {
                version: 1,
                config,
                system,
                clock,
                hidden,
                fullscreen_suppressed: false,
                hardware: HardwareMetrics::default(),
                media: None,
                privacy: PrivacyState::default(),
                spectrum: AudioSpectrumState::default(),
                notifications: NotificationSnapshot::default(),
                weather: None,
                reminders: Vec::new(),
                shelf: Vec::new(),
                focus_timer: FocusTimerState::default(),
            },
        }
    }

    pub fn snapshot(&self) -> &AppSnapshot {
        &self.snapshot
    }

    pub fn config(&self) -> &Config {
        &self.snapshot.config
    }

    pub fn set_config(&mut self, config: Config) -> bool {
        if self.snapshot.config != config {
            self.snapshot.config = config;
            self.snapshot.version = self.snapshot.version.saturating_add(1);
            true
        } else {
            false
        }
    }

    pub fn apply(&mut self, event: CoreEvent) -> bool {
        let changed = match event {
            CoreEvent::SystemChanged(sys) => {
                if self.snapshot.system != sys {
                    self.snapshot.system = sys;
                    true
                } else {
                    false
                }
            }
            CoreEvent::ClockTick(clock) => {
                if self.snapshot.clock != clock {
                    self.snapshot.clock = clock;
                    true
                } else {
                    false
                }
            }
            CoreEvent::SetHidden(hidden) => {
                if self.snapshot.hidden != hidden {
                    self.snapshot.hidden = hidden;
                    true
                } else {
                    false
                }
            }
            CoreEvent::FullscreenChanged(fs) => {
                if self.snapshot.fullscreen_suppressed != fs {
                    self.snapshot.fullscreen_suppressed = fs;
                    true
                } else {
                    false
                }
            }
            CoreEvent::HardwareUpdated(hw) => {
                if self.snapshot.hardware != hw {
                    self.snapshot.hardware = hw;
                    true
                } else {
                    false
                }
            }
            CoreEvent::MediaUpdated(media) => {
                if self.snapshot.media != media {
                    self.snapshot.media = media;
                    true
                } else {
                    false
                }
            }
            CoreEvent::PrivacyUpdated(privacy) => {
                if self.snapshot.privacy != privacy {
                    self.snapshot.privacy = privacy;
                    true
                } else {
                    false
                }
            }
            CoreEvent::SpectrumUpdated(spectrum) => {
                if self.snapshot.spectrum != spectrum {
                    self.snapshot.spectrum = spectrum;
                    true
                } else {
                    false
                }
            }
            CoreEvent::NotificationsUpdated(notifications) => {
                if self.snapshot.notifications != notifications {
                    self.snapshot.notifications = notifications;
                    true
                } else {
                    false
                }
            }
            CoreEvent::WeatherUpdated(weather) => {
                let next = Some(weather);
                if self.snapshot.weather != next {
                    self.snapshot.weather = next;
                    true
                } else {
                    false
                }
            }
            CoreEvent::RemindersUpdated(reminders) => {
                if self.snapshot.reminders != reminders {
                    self.snapshot.reminders = reminders;
                    true
                } else {
                    false
                }
            }
            CoreEvent::ShelfUpdated(shelf) => {
                if self.snapshot.shelf != shelf {
                    self.snapshot.shelf = shelf;
                    true
                } else {
                    false
                }
            }
            CoreEvent::FocusTimerUpdated(timer) => {
                if self.snapshot.focus_timer != timer {
                    self.snapshot.focus_timer = timer;
                    true
                } else {
                    false
                }
            }
            CoreEvent::QuitRequested => false,
        };
        if changed {
            self.snapshot.version = self.snapshot.version.saturating_add(1);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::BatteryStatus;

    #[test]
    fn version_increments_only_on_actual_change() {
        let sys = SystemSnapshot {
            battery: Some(BatteryStatus {
                percent: 80,
                charging: false,
            }),
            ..Default::default()
        };
        let clock = ClockStatus {
            time: "12:00 PM".into(),
            time_digits: "12:00".into(),
            ampm: "PM".into(),
            date: "Thu 20".into(),
            full_date: "Thursday, 20 November".into(),
            year: 2026,
            month: 11,
            day: 20,
            weekday: 4,
            millis_to_next_minute: 60_000,
        };
        let mut store = AppStore::new(Config::default(), sys, clock, false);
        assert_eq!(store.snapshot().version, 1);

        // Identical system snapshot does not bump version.
        assert!(!store.apply(CoreEvent::SystemChanged(sys)));
        assert_eq!(store.snapshot().version, 1);

        // Changed battery bumps version.
        let sys2 = SystemSnapshot {
            battery: Some(BatteryStatus {
                percent: 81,
                charging: true,
            }),
            ..Default::default()
        };
        assert!(store.apply(CoreEvent::SystemChanged(sys2)));
        assert_eq!(store.snapshot().version, 2);

        // Fullscreen suppression toggles version only on change.
        assert!(store.apply(CoreEvent::FullscreenChanged(true)));
        assert_eq!(store.snapshot().version, 3);
        assert!(!store.apply(CoreEvent::FullscreenChanged(true)));
        assert_eq!(store.snapshot().version, 3);
    }
}
