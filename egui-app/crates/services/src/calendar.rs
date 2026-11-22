//! Calendar & reminder service wrapping `DatabaseManager` and publishing `CoreEvent`.

use crate::db::DatabaseManager;
use crossbeam_channel::Sender;
use orbit_core::events::{CoreEvent, ReminderItem, WakeFn};
use std::sync::Arc;

pub struct CalendarService {
    db: Arc<DatabaseManager>,
    tx: Sender<CoreEvent>,
    wake: WakeFn,
}

impl CalendarService {
    pub fn new(db: Arc<DatabaseManager>, tx: Sender<CoreEvent>, wake: WakeFn) -> Self {
        Self { db, tx, wake }
    }

    fn publish_latest(&self) {
        if let Ok(items) = self.db.get_reminders() {
            let _ = self.tx.send(CoreEvent::RemindersUpdated(items));
            (self.wake)();
        }
    }

    pub fn get_reminders(&self) -> Result<Vec<ReminderItem>, String> {
        self.db.get_reminders()
    }

    pub fn add_reminder(
        &self,
        title: &str,
        due_timestamp_ms: u64,
        category: &str,
        source: &str,
    ) -> Result<ReminderItem, String> {
        let item = self.db.add_reminder(title, due_timestamp_ms, category, source)?;
        self.publish_latest();
        Ok(item)
    }

    pub fn toggle_reminder(&self, id: &str) -> Result<bool, String> {
        let res = self.db.toggle_reminder(id)?;
        self.publish_latest();
        Ok(res)
    }

    pub fn delete_reminder(&self, id: &str) -> Result<(), String> {
        self.db.delete_reminder(id)?;
        self.publish_latest();
        Ok(())
    }

    pub fn import_calendar_file(&self, raw_path: &str, source_tag: &str) -> Result<usize, String> {
        let count = self.db.import_calendar_file(raw_path, source_tag)?;
        self.publish_latest();
        Ok(count)
    }

    pub fn clear_imported_reminders(&self, source_filter: Option<&str>) -> Result<usize, String> {
        let deleted = self.db.clear_imported_reminders(source_filter)?;
        self.publish_latest();
        Ok(deleted)
    }
}
