//! Central state store with versioned snapshots.

use crate::config::Config;
use crate::events::{ClockStatus, CoreEvent, SystemSnapshot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppSnapshot {
    pub version: u64,
    pub config: Config,
    pub system: SystemSnapshot,
    pub clock: ClockStatus,
    pub hidden: bool,
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
            },
        }
    }

    pub fn snapshot(&self) -> &AppSnapshot {
        &self.snapshot
    }

    pub fn config(&self) -> &Config {
        &self.snapshot.config
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
            date: "Thu 20".into(),
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
    }
}
