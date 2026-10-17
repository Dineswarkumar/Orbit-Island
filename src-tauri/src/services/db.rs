use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReminderItem {
    pub id: String,
    pub title: String,
    pub due_timestamp_ms: u64,
    pub is_completed: bool,
    pub source: String,   // "local", "google", "outlook"
    pub category: String, // "task", "event", "reminder", "deadline"
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub id: String,
    pub file_path: String,
    pub display_name: String,
    pub file_size: u64,
    pub extension: String,
    pub added_at: u64,
}

pub struct DatabaseManager {
    conn: Arc<Mutex<Connection>>,
}

impl DatabaseManager {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create db directory: {e}"))?;
        }

        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open SQLite database: {e}"))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS reminders (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                due_timestamp_ms INTEGER NOT NULL,
                is_completed INTEGER NOT NULL DEFAULT 0,
                source TEXT NOT NULL DEFAULT 'local',
                category TEXT NOT NULL DEFAULT 'task',
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS file_shelf (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                extension TEXT NOT NULL,
                added_at INTEGER NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Failed to initialize SQLite schema: {e}"))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn for_app(app: &AppHandle) -> Result<Self, String> {
        let app_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
        let db_path = app_dir.join("orbit.db");
        Self::new(db_path)
    }

    // Reminders / Productivity Hub Operations
    pub fn get_reminders(&self) -> Result<Vec<ReminderItem>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, due_timestamp_ms, is_completed, source, category, created_at
                 FROM reminders
                 ORDER BY is_completed ASC, due_timestamp_ms ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ReminderItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    due_timestamp_ms: row.get::<_, i64>(2)? as u64,
                    is_completed: row.get::<_, i32>(3)? != 0,
                    source: row.get(4)?,
                    category: row.get(5)?,
                    created_at: row.get::<_, i64>(6)? as u64,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for r in rows {
            if let Ok(item) = r {
                items.push(item);
            }
        }
        Ok(items)
    }

    pub fn add_reminder(
        &self,
        title: &str,
        due_timestamp_ms: u64,
        category: &str,
        source: &str,
    ) -> Result<ReminderItem, String> {
        let clean_title = title.trim();
        if clean_title.is_empty() {
            return Err("Reminder title cannot be empty".to_string());
        }

        let id = format!("rem_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let item = ReminderItem {
            id: id.clone(),
            title: clean_title.to_string(),
            due_timestamp_ms,
            is_completed: false,
            source: if source.is_empty() { "local".to_string() } else { source.to_string() },
            category: if category.is_empty() { "task".to_string() } else { category.to_string() },
            created_at: now,
        };

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO reminders (id, title, due_timestamp_ms, is_completed, source, category, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                item.id,
                item.title,
                item.due_timestamp_ms as i64,
                if item.is_completed { 1 } else { 0 },
                item.source,
                item.category,
                item.created_at as i64,
            ],
        )
        .map_err(|e| format!("Failed to insert reminder: {e}"))?;

        Ok(item)
    }

    pub fn toggle_reminder(&self, id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().unwrap();
        let current_state: Option<i32> = conn
            .query_row(
                "SELECT is_completed FROM reminders WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok();

        let current = current_state.ok_or_else(|| "Reminder not found".to_string())?;
        let next = if current == 0 { 1 } else { 0 };

        conn.execute(
            "UPDATE reminders SET is_completed = ?1 WHERE id = ?2",
            params![next, id],
        )
        .map_err(|e| format!("Failed to update reminder: {e}"))?;

        Ok(next == 1)
    }

    pub fn delete_reminder(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM reminders WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete reminder: {e}"))?;
        Ok(())
    }

    // File Shelf Operations (Section 5.8: max 10 items, holds references without copying originals)
    pub fn get_shelf_items(&self) -> Result<Vec<ShelfItem>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, file_path, display_name, file_size, extension, added_at
                 FROM file_shelf
                 ORDER BY added_at DESC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ShelfItem {
                    id: row.get(0)?,
                    file_path: row.get(1)?,
                    display_name: row.get(2)?,
                    file_size: row.get::<_, i64>(3)? as u64,
                    extension: row.get(4)?,
                    added_at: row.get::<_, i64>(5)? as u64,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for r in rows {
            if let Ok(item) = r {
                items.push(item);
            }
        }
        Ok(items)
    }

    pub fn add_shelf_item(&self, raw_path: &str) -> Result<ShelfItem, String> {
        let path = Path::new(raw_path.trim());
        if !path.exists() {
            return Err(format!("File does not exist: {}", raw_path));
        }

        let metadata = fs::metadata(path)
            .map_err(|e| format!("Could not read file metadata: {e}"))?;

        let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let norm_path = canonical.to_string_lossy().to_string();

        let conn = self.conn.lock().unwrap();

        // Check if already in shelf (duplicate check by normalized path)
        let existing: Option<ShelfItem> = conn
            .query_row(
                "SELECT id, file_path, display_name, file_size, extension, added_at
                 FROM file_shelf WHERE file_path = ?1",
                params![norm_path],
                |row| {
                    Ok(ShelfItem {
                        id: row.get(0)?,
                        file_path: row.get(1)?,
                        display_name: row.get(2)?,
                        file_size: row.get::<_, i64>(3)? as u64,
                        extension: row.get(4)?,
                        added_at: row.get::<_, i64>(5)? as u64,
                    })
                },
            )
            .ok();

        if let Some(item) = existing {
            return Ok(item);
        }

        // Capacity check: strictly maximum 10 items
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM file_shelf", [], |row| row.get(0))
            .unwrap_or(0);

        if count >= 10 {
            return Err("File Shelf is full (maximum 10 items). Remove an item before adding another.".to_string());
        }

        let display_name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown File".to_string());

        let extension = path
            .extension()
            .map(|s| s.to_string_lossy().to_string().to_lowercase())
            .unwrap_or_default();

        let id = format!("shelf_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let item = ShelfItem {
            id: id.clone(),
            file_path: norm_path.clone(),
            display_name,
            file_size: metadata.len(),
            extension,
            added_at: now,
        };

        conn.execute(
            "INSERT INTO file_shelf (id, file_path, display_name, file_size, extension, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                item.id,
                item.file_path,
                item.display_name,
                item.file_size as i64,
                item.extension,
                item.added_at as i64,
            ],
        )
        .map_err(|e| format!("Failed to insert shelf item: {e}"))?;

        Ok(item)
    }

    pub fn remove_shelf_item(&self, id: &str) -> Result<(), String> {
        // Removing reference from shelf NEVER deletes the original user file
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM file_shelf WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to remove shelf item: {e}"))?;
        Ok(())
    }

    pub fn open_shelf_file(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM file_shelf WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| "Shelf item not found".to_string())?;

        let path = Path::new(&file_path);
        if !path.exists() {
            return Err("File not found at its original location. It may have been moved or deleted.".to_string());
        }

        #[cfg(target_os = "windows")]
        unsafe {
            use windows_sys::Win32::UI::Shell::ShellExecuteW;
            use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
            let wide_path: Vec<u16> = file_path.encode_utf16().chain(std::iter::once(0)).collect();
            let open_op: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();

            let res = ShellExecuteW(
                std::ptr::null_mut(),
                open_op.as_ptr(),
                wide_path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL as i32,
            );

            if (res as isize) <= 32 {
                return Err(format!("ShellExecuteW failed to open file (code: {res:?})"));
            }
        }

        Ok(())
    }

    pub fn show_shelf_file_in_folder(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let file_path: String = conn
            .query_row(
                "SELECT file_path FROM file_shelf WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| "Shelf item not found".to_string())?;

        let path = Path::new(&file_path);
        if !path.exists() {
            return Err("File not found at its original location.".to_string());
        }

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let _ = std::process::Command::new("explorer")
                .arg(format!("/select,{}", file_path))
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }

        Ok(())
    }

    pub fn clear_shelf(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM file_shelf", [])
            .map_err(|e| format!("Failed to clear file shelf: {e}"))?;
        Ok(())
    }

    pub fn clear_local_data(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            DELETE FROM reminders;
            DELETE FROM file_shelf;
            ",
        )
        .map_err(|e| format!("Failed to clear local database tables: {e}"))?;
        Ok(())
    }
}

// Module dunce provides Windows path normalisation fallback
mod dunce {
    use std::path::{Path, PathBuf};
    pub fn canonicalize<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
        std::fs::canonicalize(path).map(|p| {
            let s = p.to_string_lossy();
            if s.starts_with(r"\\?\") {
                PathBuf::from(&s[4..])
            } else {
                p
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminders_crud_and_persistence() {
        let temp_dir = std::env::temp_dir().join(format!("orbit_test_db_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let db_path = temp_dir.join("test.db");
        let db = DatabaseManager::new(db_path.clone()).expect("Failed to create test db");

        // 1. Initial list empty
        let initial = db.get_reminders().unwrap();
        assert!(initial.is_empty());

        // 2. Add reminder
        let item1 = db.add_reminder("Submit report", 1700000000000, "task", "local").unwrap();
        assert_eq!(item1.title, "Submit report");
        assert!(!item1.is_completed);

        // 3. Add second reminder
        let item2 = db.add_reminder("Design sync", 1699999999000, "event", "local").unwrap();

        // 4. Get list (ordered by due date)
        let list = db.get_reminders().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, item2.id); // earlier due date first

        // 5. Toggle completed
        let is_completed = db.toggle_reminder(&item2.id).unwrap();
        assert!(is_completed);

        // 6. Delete reminder
        db.delete_reminder(&item1.id).unwrap();
        let remaining = db.get_reminders().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, item2.id);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn file_shelf_bounds_and_preservation() {
        let temp_dir = std::env::temp_dir().join(format!("orbit_test_shelf_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let db_path = temp_dir.join("shelf.db");
        let db = DatabaseManager::new(db_path).expect("Failed to create test db");

        // Create 11 temporary test files
        let mut test_files = Vec::new();
        for i in 1..=11 {
            let file_path = temp_dir.join(format!("test_file_{}.txt", i));
            fs::write(&file_path, format!("Content {}", i)).unwrap();
            test_files.push(file_path);
        }

        // Add 10 items (must all succeed)
        for i in 0..10 {
            let res = db.add_shelf_item(&test_files[i].to_string_lossy());
            assert!(res.is_ok(), "Item {} must be added successfully", i + 1);
        }

        let shelf = db.get_shelf_items().unwrap();
        assert_eq!(shelf.len(), 10, "Shelf must hold exactly 10 items");

        // 11th item must be rejected per PRD Section 5.8
        let rejected = db.add_shelf_item(&test_files[10].to_string_lossy());
        assert!(rejected.is_err(), "11th item must be rejected");
        assert!(
            rejected.unwrap_err().contains("File Shelf is full"),
            "Error message must clearly state full shelf"
        );

        // Duplicate addition of already held item returns existing without incrementing count
        let duplicate = db.add_shelf_item(&test_files[0].to_string_lossy()).unwrap();
        assert_eq!(duplicate.file_path, dunce::canonicalize(&test_files[0]).unwrap().to_string_lossy().to_string());
        assert_eq!(db.get_shelf_items().unwrap().len(), 10);

        // Remove item from shelf: MUST NOT delete original file
        let item_to_remove = &shelf[0];
        let original_file_path = PathBuf::from(&item_to_remove.file_path);
        assert!(original_file_path.exists(), "Original file must exist before shelf removal");

        db.remove_shelf_item(&item_to_remove.id).unwrap();
        assert_eq!(db.get_shelf_items().unwrap().len(), 9);
        assert!(
            original_file_path.exists(),
            "Original file MUST NOT be deleted when reference is removed from shelf"
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn clear_shelf_and_local_data() {
        let temp_dir = std::env::temp_dir().join(format!("orbit_test_clear_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let db_path = temp_dir.join("clear_test.db");
        let db = DatabaseManager::new(db_path).expect("Failed to create test db");

        let file_path = temp_dir.join("test.txt");
        fs::write(&file_path, "test").unwrap();

        db.add_reminder("Sample", 1700000000000, "task", "local").unwrap();
        db.add_shelf_item(&file_path.to_string_lossy()).unwrap();

        assert_eq!(db.get_reminders().unwrap().len(), 1);
        assert_eq!(db.get_shelf_items().unwrap().len(), 1);

        db.clear_shelf().unwrap();
        assert_eq!(db.get_shelf_items().unwrap().len(), 0);
        assert_eq!(db.get_reminders().unwrap().len(), 1);

        db.clear_local_data().unwrap();
        assert_eq!(db.get_reminders().unwrap().len(), 0);
        assert_eq!(db.get_shelf_items().unwrap().len(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
