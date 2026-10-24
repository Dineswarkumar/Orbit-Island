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
    pub exists: bool,
    pub is_managed_copy: bool,
}

pub struct DatabaseManager {
    conn: Arc<Mutex<Connection>>,
    shelf_storage_dir: PathBuf,
}

impl DatabaseManager {
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let shelf_storage_dir = db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("shelf_storage");
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
            shelf_storage_dir,
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

    /// Clears imported schedule events (optionally filtered by source like "academic", "office", "ics"),
    /// while preserving manual local tasks (`source = 'local'`).
    pub fn clear_imported_reminders(&self, source_filter: Option<&str>) -> Result<usize, String> {
        let conn = self.conn.lock().unwrap();
        let deleted = if let Some(src) = source_filter.filter(|s| !s.trim().is_empty()) {
            conn.execute("DELETE FROM reminders WHERE source = ?1", params![src.trim()])
                .map_err(|e| format!("Failed to clear imported reminders: {e}"))?
        } else {
            conn.execute("DELETE FROM reminders WHERE source != 'local'", [])
                .map_err(|e| format!("Failed to clear imported reminders: {e}"))?
        };
        Ok(deleted)
    }

    /// Imports calendar events from an `.ics`, `.csv`, or `.json` file into SQLite `reminders`.
    /// Automatically deduplicates by `(LOWER(title), due_timestamp_ms)` and classifies categories.
    pub fn import_calendar_file(&self, raw_path: &str, source_tag: &str) -> Result<usize, String> {
        let path = Path::new(raw_path.trim());
        if !path.exists() {
            return Err(format!("Schedule file does not exist: {}", raw_path));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read schedule file: {e}"))?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let clean_source = if source_tag.trim().is_empty() {
            "ics"
        } else {
            source_tag.trim()
        };

        let parsed_events = match ext.as_str() {
            "ics" | "ical" => parse_ics_events(&content),
            "csv" => parse_csv_events(&content),
            "json" => parse_json_events(&content),
            _ => {
                if content.contains("BEGIN:VCALENDAR") || content.contains("BEGIN:VEVENT") {
                    parse_ics_events(&content)
                } else {
                    parse_csv_events(&content)
                }
            }
        };

        if parsed_events.is_empty() {
            return Err("No valid events found in the selected file. Supported formats: .ics (iCalendar), .csv (Title, Date, Time, Category), or .json.".to_string());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let conn = self.conn.lock().unwrap();
        let mut inserted_count = 0usize;

        for (idx, (title, due_ms, raw_cat)) in parsed_events.into_iter().enumerate() {
            let clean_title = title.trim();
            if clean_title.is_empty() || due_ms == 0 {
                continue;
            }

            // Deduplicate: skip if an event with the same title and due_timestamp_ms already exists
            let exists: Option<i32> = conn
                .query_row(
                    "SELECT 1 FROM reminders WHERE LOWER(title) = LOWER(?1) AND ABS(due_timestamp_ms - ?2) < 60000 LIMIT 1",
                    params![clean_title, due_ms as i64],
                    |row| row.get(0),
                )
                .ok();
            if exists.is_some() {
                continue;
            }

            let category = classify_event_category(clean_title, &raw_cat, clean_source);
            let id = format!("imp_{}_{}_{}", clean_source, now, idx);

            if conn
                .execute(
                    "INSERT INTO reminders (id, title, due_timestamp_ms, is_completed, source, category, created_at)
                     VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6)",
                    params![
                        id,
                        clean_title,
                        due_ms as i64,
                        clean_source,
                        category,
                        now as i64
                    ],
                )
                .is_ok()
            {
                inserted_count += 1;
            }
        }

        Ok(inserted_count)
    }

    // File Shelf Operations (Section 5.8: max 10 items, holds references or optional ≤50MB managed copies)
    pub fn get_shelf_items(&self) -> Result<Vec<ShelfItem>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, file_path, display_name, file_size, extension, added_at
                 FROM file_shelf
                 ORDER BY added_at DESC",
            )
            .map_err(|e| e.to_string())?;

        let storage_prefix = self.shelf_storage_dir.to_string_lossy().to_string();
        let rows = stmt
            .query_map([], |row| {
                let fp: String = row.get(1)?;
                let exists = Path::new(&fp).exists();
                let is_managed_copy = fp.starts_with(&storage_prefix) || fp.contains("shelf_storage");
                Ok(ShelfItem {
                    id: row.get(0)?,
                    file_path: fp,
                    display_name: row.get(2)?,
                    file_size: row.get::<_, i64>(3)? as u64,
                    extension: row.get(4)?,
                    added_at: row.get::<_, i64>(5)? as u64,
                    exists,
                    is_managed_copy,
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
        self.add_shelf_item_with_mode(raw_path, "reference")
    }

    pub fn add_shelf_item_with_mode(&self, raw_path: &str, mode: &str) -> Result<ShelfItem, String> {
        let path = Path::new(raw_path.trim());
        if !path.exists() {
            return Err(format!("File does not exist: {}", raw_path));
        }

        let metadata = fs::metadata(path)
            .map_err(|e| format!("Could not read file metadata: {e}"))?;

        const MAX_COPY_BYTES: u64 = 50 * 1024 * 1024;
        if mode == "copy" && metadata.len() > MAX_COPY_BYTES {
            return Err(
                "File exceeds 50 MB limit for Managed Copy mode. Switch Shelf mode to Reference or pick a smaller file."
                    .to_string(),
            );
        }

        let canonical = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        let norm_path = canonical.to_string_lossy().to_string();

        let conn = self.conn.lock().unwrap();

        // Check if already in shelf (duplicate check by normalized path)
        let storage_prefix = self.shelf_storage_dir.to_string_lossy().to_string();
        let existing: Option<ShelfItem> = conn
            .query_row(
                "SELECT id, file_path, display_name, file_size, extension, added_at
                 FROM file_shelf WHERE file_path = ?1",
                params![norm_path],
                |row| {
                    let fp: String = row.get(1)?;
                    let exists = Path::new(&fp).exists();
                    let is_managed_copy =
                        fp.starts_with(&storage_prefix) || fp.contains("shelf_storage");
                    Ok(ShelfItem {
                        id: row.get(0)?,
                        file_path: fp,
                        display_name: row.get(2)?,
                        file_size: row.get::<_, i64>(3)? as u64,
                        extension: row.get(4)?,
                        added_at: row.get::<_, i64>(5)? as u64,
                        exists,
                        is_managed_copy,
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

        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = format!("shelf_{now_nanos}");
        let now = (now_nanos / 1_000_000) as u64;

        let (stored_path, is_managed_copy) = if mode == "copy" {
            fs::create_dir_all(&self.shelf_storage_dir)
                .map_err(|e| format!("Failed to create shelf storage directory: {e}"))?;
            let safe_name: String = display_name
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
                .collect();
            let dest_path = self.shelf_storage_dir.join(format!("{now_nanos}_{safe_name}"));
            fs::copy(&canonical, &dest_path)
                .map_err(|e| format!("Failed to copy file to managed shelf storage: {e}"))?;
            let dest_canonical = dunce::canonicalize(&dest_path).unwrap_or(dest_path);
            (dest_canonical.to_string_lossy().to_string(), true)
        } else {
            (norm_path, false)
        };

        let item = ShelfItem {
            id: id.clone(),
            file_path: stored_path,
            display_name,
            file_size: metadata.len(),
            extension,
            added_at: now,
            exists: true,
            is_managed_copy,
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
        // Removing reference from shelf NEVER deletes original user files outside shelf_storage.
        // If the item is a managed copy inside shelf_storage, clean up that managed copy.
        let conn = self.conn.lock().unwrap();
        let existing_path: Option<String> = conn
            .query_row(
                "SELECT file_path FROM file_shelf WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok();

        conn.execute("DELETE FROM file_shelf WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to remove shelf item: {e}"))?;

        if let Some(fp) = existing_path {
            let storage_prefix = self.shelf_storage_dir.to_string_lossy().to_string();
            if fp.starts_with(&storage_prefix) || fp.contains("shelf_storage") {
                let _ = fs::remove_file(&fp);
            }
        }
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
        if self.shelf_storage_dir.exists() {
            let _ = fs::remove_dir_all(&self.shelf_storage_dir);
        }
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
        if self.shelf_storage_dir.exists() {
            let _ = fs::remove_dir_all(&self.shelf_storage_dir);
        }
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

fn classify_event_category(title: &str, explicit_cat: &str, source: &str) -> String {
    let exp = explicit_cat.trim().to_lowercase();
    if matches!(
        exp.as_str(),
        "exam" | "deadline" | "meeting" | "class" | "holiday" | "event" | "task" | "reminder"
    ) {
        return exp;
    }

    let lower = format!("{} {}", title.to_lowercase(), exp);
    if lower.contains("exam")
        || lower.contains("midterm")
        || lower.contains("final")
        || lower.contains("quiz")
        || lower.contains("test")
        || lower.contains("viva")
        || lower.contains("assessment")
    {
        "exam".to_string()
    } else if lower.contains("due")
        || lower.contains("deadline")
        || lower.contains("submission")
        || lower.contains("assignment")
        || lower.contains("homework")
        || lower.contains("deliverable")
        || lower.contains("milestone")
    {
        "deadline".to_string()
    } else if lower.contains("holiday")
        || lower.contains("vacation")
        || lower.contains("festival")
        || lower.contains("break")
        || lower.contains("day off")
        || lower.contains("closed")
    {
        "holiday".to_string()
    } else if lower.contains("meeting")
        || lower.contains("standup")
        || lower.contains("sync")
        || lower.contains("1:1")
        || lower.contains("scrum")
        || lower.contains("sprint")
        || lower.contains("interview")
        || lower.contains("review")
        || lower.contains("client")
    {
        "meeting".to_string()
    } else if lower.contains("lecture")
        || lower.contains("class")
        || lower.contains("lab")
        || lower.contains("seminar")
        || lower.contains("workshop")
        || lower.contains("tutorial")
        || source == "academic"
    {
        "class".to_string()
    } else if source == "office" {
        "meeting".to_string()
    } else {
        "event".to_string()
    }
}

/// Converts `(year, month 1..=12, day 1..=31, hour 0..=23, min 0..=59, sec 0..=59)` to epoch milliseconds.
/// Uses the civil-from-days algorithm (Howard Hinnant) for exact UTC/local timestamp calculation.
fn ymd_hms_to_epoch_ms(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32, is_utc: bool) -> u64 {
    if !(1970..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return 0;
    }
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m = month;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = (era as i64) * 146097 + (doe as i64) - 719468;
    if days < 0 {
        return 0;
    }
    let total_secs = days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64);
    // If local time (not ending with 'Z'), adjust by the system's local UTC offset so JavaScript `new Date(ms)`
    // lands on the exact local calendar date and hour!
    let adjusted_secs = if is_utc {
        total_secs
    } else {
        total_secs - local_utc_offset_secs()
    };
    if adjusted_secs <= 0 {
        0
    } else {
        (adjusted_secs as u64) * 1000
    }
}

fn local_utc_offset_secs() -> i64 {
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        struct SYSTEMTIME {
            w_year: u16,
            w_month: u16,
            w_day_of_week: u16,
            w_day: u16,
            w_hour: u16,
            w_minute: u16,
            w_second: u16,
            w_milliseconds: u16,
        }
        #[repr(C)]
        struct TIME_ZONE_INFORMATION {
            bias: i32,
            standard_name: [u16; 32],
            standard_date: SYSTEMTIME,
            standard_bias: i32,
            daylight_name: [u16; 32],
            daylight_date: SYSTEMTIME,
            daylight_bias: i32,
        }
        unsafe extern "system" {
            fn GetTimeZoneInformation(lp_time_zone_information: *mut TIME_ZONE_INFORMATION) -> u32;
        }
        unsafe {
            let mut tzi: TIME_ZONE_INFORMATION = std::mem::zeroed();
            let res = GetTimeZoneInformation(&mut tzi);
            let active_bias = if res == 2 {
                tzi.bias + tzi.daylight_bias
            } else {
                tzi.bias + tzi.standard_bias
            };
            // Windows Bias is defined as: UTC = local time + bias (in minutes)
            // Therefore local_offset_from_utc = -active_bias * 60
            (-active_bias as i64) * 60
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        0
    }
}

fn parse_flexible_datetime(raw: &str) -> u64 {
    let s = raw.trim().trim_matches('"').trim_matches('\'');
    if s.is_empty() {
        return 0;
    }

    // 1. iCalendar format: YYYYMMDD or YYYYMMDDTHHMMSS or YYYYMMDDTHHMMSSZ
    let is_utc = s.ends_with('Z') || s.ends_with('z');
    let clean_ical = s.trim_end_matches(['Z', 'z']);
    if clean_ical.len() >= 8 && clean_ical.chars().take(8).all(|c| c.is_ascii_digit()) {
        let year = clean_ical[0..4].parse::<i32>().unwrap_or(0);
        let month = clean_ical[4..6].parse::<u32>().unwrap_or(0);
        let day = clean_ical[6..8].parse::<u32>().unwrap_or(0);
        let mut hour = 9u32; // Default all-day events to 09:00 local time so they never shift across midnight
        let mut min = 0u32;
        let mut sec = 0u32;
        if let Some(t_idx) = clean_ical.find(['T', 't']) {
            let time_part = &clean_ical[t_idx + 1..];
            if time_part.len() >= 4 {
                hour = time_part[0..2].parse::<u32>().unwrap_or(9);
                min = time_part[2..4].parse::<u32>().unwrap_or(0);
            }
            if time_part.len() >= 6 {
                sec = time_part[4..6].parse::<u32>().unwrap_or(0);
            }
            return ymd_hms_to_epoch_ms(year, month, day, hour, min, sec, is_utc);
        }
        return ymd_hms_to_epoch_ms(year, month, day, hour, min, sec, false);
    }

    // 2. ISO / CSV date: YYYY-MM-DD [HH:MM] or DD/MM/YYYY or MM/DD/YYYY
    let parts: Vec<&str> = s.split([' ', 'T', 't']).filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return 0;
    }
    let date_str = parts[0];
    let mut hour = 9u32;
    let mut min = 0u32;
    if parts.len() >= 2 {
        let time_str = parts[1].trim_end_matches(['Z', 'z']);
        let hm: Vec<&str> = time_str.split(':').collect();
        if hm.len() >= 2 {
            hour = hm[0].parse::<u32>().unwrap_or(9);
            min = hm[1].chars().take(2).collect::<String>().parse::<u32>().unwrap_or(0);
            if parts.len() >= 3 {
                let ampm = parts[2].to_uppercase();
                if ampm == "PM" && hour < 12 {
                    hour += 12;
                } else if ampm == "AM" && hour == 12 {
                    hour = 0;
                }
            }
        }
    }

    if date_str.contains('-') {
        let d: Vec<&str> = date_str.split('-').collect();
        if d.len() == 3 {
            let p0 = d[0].parse::<i32>().unwrap_or(0);
            if p0 >= 1970 {
                let m = d[1].parse::<u32>().unwrap_or(0);
                let day = d[2].parse::<u32>().unwrap_or(0);
                return ymd_hms_to_epoch_ms(p0, m, day, hour, min, 0, false);
            } else {
                // DD-MM-YYYY
                let day = d[0].parse::<u32>().unwrap_or(0);
                let m = d[1].parse::<u32>().unwrap_or(0);
                let y = d[2].parse::<i32>().unwrap_or(0);
                return ymd_hms_to_epoch_ms(y, m, day, hour, min, 0, false);
            }
        }
    } else if date_str.contains('/') {
        let d: Vec<&str> = date_str.split('/').collect();
        if d.len() == 3 {
            let p0 = d[0].parse::<u32>().unwrap_or(0);
            let p1 = d[1].parse::<u32>().unwrap_or(0);
            let p2 = d[2].parse::<i32>().unwrap_or(0);
            if p0 >= 1970 {
                return ymd_hms_to_epoch_ms(p0 as i32, p1, p2 as u32, hour, min, 0, false);
            } else if p2 >= 1970 {
                // Support both DD/MM/YYYY and MM/DD/YYYY intelligently
                let (day, month) = if p0 > 12 { (p0, p1) } else { (p0, p1) };
                return ymd_hms_to_epoch_ms(p2, month, day, hour, min, 0, false);
            }
        }
    }

    0
}

fn parse_ics_events(content: &str) -> Vec<(String, u64, String)> {
    // Unfold RFC 5545 folded lines (CRLF followed by space or tab)
    let unfolded = content.replace("\r\n ", "").replace("\n ", "").replace("\r\n\t", "").replace("\n\t", "");
    let mut events = Vec::new();
    let mut in_event = false;
    let mut summary = String::new();
    let mut dtstart_ms = 0u64;
    let mut categories = String::new();

    for raw_line in unfolded.lines() {
        let line = raw_line.trim();
        if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
            in_event = true;
            summary.clear();
            dtstart_ms = 0;
            categories.clear();
        } else if line.eq_ignore_ascii_case("END:VEVENT") {
            if in_event && !summary.trim().is_empty() && dtstart_ms > 0 {
                events.push((summary.trim().to_string(), dtstart_ms, categories.trim().to_string()));
            }
            in_event = false;
        } else if in_event {
            if let Some((prop, val)) = line.split_once(':') {
                let prop_upper = prop.to_uppercase();
                let base_prop = prop_upper.split(';').next().unwrap_or("");
                let clean_val = val
                    .replace("\\n", " ")
                    .replace("\\N", " ")
                    .replace("\\,", ",")
                    .replace("\\;", ";")
                    .trim()
                    .to_string();
                match base_prop {
                    "SUMMARY" => summary = clean_val,
                    "DTSTART" => {
                        let parsed = parse_flexible_datetime(&clean_val);
                        if parsed > 0 {
                            dtstart_ms = parsed;
                        }
                    }
                    "CATEGORIES" => categories = clean_val,
                    _ => {}
                }
            }
        }
    }

    events
}

fn parse_csv_events(content: &str) -> Vec<(String, u64, String)> {
    let mut events = Vec::new();
    let mut title_idx = 0usize;
    let mut date_idx = 1usize;
    let mut time_idx: Option<usize> = None;
    let mut cat_idx: Option<usize> = None;

    for (line_num, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split(',').map(|c| c.trim().trim_matches('"')).collect();
        if cols.len() < 2 {
            continue;
        }

        // Check if first row is a header row
        if line_num == 0 {
            let lower_cols: Vec<String> = cols.iter().map(|c| c.to_lowercase()).collect();
            let has_header = lower_cols.iter().any(|c| {
                c.contains("title")
                    || c.contains("event")
                    || c.contains("subject")
                    || c.contains("summary")
                    || c.contains("date")
                    || c.contains("start")
            });
            if has_header {
                for (i, col) in lower_cols.iter().enumerate() {
                    if col.contains("title") || col.contains("event") || col.contains("subject") || col.contains("summary") || col.contains("name") {
                        title_idx = i;
                    } else if col.contains("date") || col.contains("start") || col.contains("day") {
                        date_idx = i;
                    } else if col.contains("time") || col.contains("hour") {
                        time_idx = Some(i);
                    } else if col.contains("cat") || col.contains("type") || col.contains("tag") {
                        cat_idx = Some(i);
                    }
                }
                continue;
            }
        }

        let title = cols.get(title_idx).copied().unwrap_or("").trim();
        let date_part = cols.get(date_idx).copied().unwrap_or("").trim();
        let combined_dt = if let Some(t_i) = time_idx {
            let t_val = cols.get(t_i).copied().unwrap_or("").trim();
            if t_val.is_empty() {
                date_part.to_string()
            } else {
                format!("{} {}", date_part, t_val)
            }
        } else {
            date_part.to_string()
        };

        let mut ms = parse_flexible_datetime(&combined_dt);
        let mut final_title = title.to_string();
        // Also support swapped columns (Date, Title) if col[0] is a valid date
        if ms == 0 && title_idx == 0 && date_idx == 1 {
            let swapped_ms = parse_flexible_datetime(title);
            if swapped_ms > 0 {
                ms = swapped_ms;
                final_title = date_part.to_string();
            }
        }

        let category = cat_idx
            .and_then(|i| cols.get(i).copied())
            .unwrap_or("")
            .to_string();

        if !final_title.is_empty() && ms > 0 {
            events.push((final_title, ms, category));
        }
    }

    events
}

fn parse_json_events(content: &str) -> Vec<(String, u64, String)> {
    let mut events = Vec::new();
    if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(content) {
        for item in arr {
            let title = item
                .get("title")
                .or_else(|| item.get("summary"))
                .or_else(|| item.get("event"))
                .or_else(|| item.get("subject"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let due_ms = if let Some(ms) = item.get("dueTimestampMs").and_then(|v| v.as_u64()) {
                ms
            } else if let Some(dt_str) = item
                .get("date")
                .or_else(|| item.get("start"))
                .or_else(|| item.get("datetime"))
                .and_then(|v| v.as_str())
            {
                parse_flexible_datetime(dt_str)
            } else {
                0
            };
            let cat = item
                .get("category")
                .or_else(|| item.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if !title.is_empty() && due_ms > 0 {
                events.push((title, due_ms, cat));
            }
        }
    }
    events
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

    #[test]
    fn file_shelf_managed_copy_and_missing_indicator() {
        let temp_dir = std::env::temp_dir().join(format!(
            "orbit_test_copy_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = temp_dir.join("copy_test.db");
        let db = DatabaseManager::new(db_path).expect("Failed to create test db");

        let ref_file = temp_dir.join("ref_doc.txt");
        fs::write(&ref_file, "reference data").unwrap();
        let copy_file = temp_dir.join("copy_doc.txt");
        fs::write(&copy_file, "managed copy data").unwrap();

        let ref_item = db
            .add_shelf_item_with_mode(&ref_file.to_string_lossy(), "reference")
            .unwrap();
        assert!(ref_item.exists);
        assert!(!ref_item.is_managed_copy);

        let copy_item = db
            .add_shelf_item_with_mode(&copy_file.to_string_lossy(), "copy")
            .unwrap();
        assert!(copy_item.exists);
        assert!(copy_item.is_managed_copy);
        assert!(copy_item.file_path.contains("shelf_storage"));

        // If the original ref_file is deleted, get_shelf_items reports exists=false for ref_item,
        // while copy_item still exists even if copy_file is deleted!
        let _ = fs::remove_file(&ref_file);
        let _ = fs::remove_file(&copy_file);

        let list = db.get_shelf_items().unwrap();
        let fetched_ref = list.iter().find(|i| i.id == ref_item.id).unwrap();
        let fetched_copy = list.iter().find(|i| i.id == copy_item.id).unwrap();
        assert!(!fetched_ref.exists);
        assert!(fetched_copy.exists);

        // Removing the managed copy deletes the file inside shelf_storage
        let managed_path = PathBuf::from(&fetched_copy.file_path);
        assert!(managed_path.exists());
        db.remove_shelf_item(&fetched_copy.id).unwrap();
        assert!(!managed_path.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_ics_import_parses_vevents_and_deduplicates() {
        let temp_dir = std::env::temp_dir().join(format!(
            "orbit_test_ics_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = temp_dir.join("ics_test.db");
        let db = DatabaseManager::new(db_path).expect("Failed to create test db");

        let ics_content = r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//University//Academic Calendar//EN
BEGIN:VEVENT
UID:exam-cs301-2026
DTSTART:20261115T093000
SUMMARY:Data Structures Final Exam
CATEGORIES:Academics,Exams
DESCRIPTION:Room 402 Main Campus
END:VEVENT
BEGIN:VEVENT
UID:assign-cs302-2026
DTSTART:20261120T235900
SUMMARY:Compiler Design Assignment 2 Submission Due
CATEGORIES:Assignment
END:VEVENT
END:VCALENDAR"#;

        let ics_file = temp_dir.join("calendar.ics");
        fs::write(&ics_file, ics_content).unwrap();

        // 1. First import: imports 2 events
        let count = db
            .import_calendar_file(&ics_file.to_string_lossy(), "academic")
            .unwrap();
        assert_eq!(count, 2);

        let list = db.get_reminders().unwrap();
        assert_eq!(list.len(), 2);

        let exam_item = list.iter().find(|i| i.title.contains("Final Exam")).unwrap();
        assert_eq!(exam_item.category, "exam");
        assert_eq!(exam_item.source, "academic");

        let deadline_item = list
            .iter()
            .find(|i| i.title.contains("Submission Due"))
            .unwrap();
        assert_eq!(deadline_item.category, "deadline");

        // 2. Re-import: deduplication prevents re-inserting existing items
        let count_again = db
            .import_calendar_file(&ics_file.to_string_lossy(), "academic")
            .unwrap();
        assert_eq!(count_again, 0);
        assert_eq!(db.get_reminders().unwrap().len(), 2);

        // 3. Add manual local task
        db.add_reminder("Buy groceries", 1700000000000, "task", "local")
            .unwrap();
        assert_eq!(db.get_reminders().unwrap().len(), 3);

        // 4. Clear imported reminders: removes academic items, preserves manual local task!
        let cleared = db.clear_imported_reminders(None).unwrap();
        assert_eq!(cleared, 2);
        let preserved = db.get_reminders().unwrap();
        assert_eq!(preserved.len(), 1);
        assert_eq!(preserved[0].title, "Buy groceries");
        assert_eq!(preserved[0].source, "local");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_csv_import_parses_academic_and_office_dates() {
        let temp_dir = std::env::temp_dir().join(format!(
            "orbit_test_csv_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let db_path = temp_dir.join("csv_test.db");
        let db = DatabaseManager::new(db_path).expect("Failed to create test db");

        let csv_content = r#"Title,Date,Time,Category
Sprint Planning Sync,2026-10-28,14:00,Meeting
Operating Systems Viva,2026-11-02,10:30,Exam
Company Thanksgiving Day Off,2026-11-26,09:00,Holiday
Machine Learning Lab,2026-10-29,11:00,Class
"#;
        let csv_file = temp_dir.join("schedule.csv");
        fs::write(&csv_file, csv_content).unwrap();

        let count = db
            .import_calendar_file(&csv_file.to_string_lossy(), "office")
            .unwrap();
        assert_eq!(count, 4);

        let list = db.get_reminders().unwrap();
        assert_eq!(list.len(), 4);

        let meeting = list.iter().find(|i| i.title.contains("Sprint Planning")).unwrap();
        assert_eq!(meeting.category, "meeting");

        let viva = list.iter().find(|i| i.title.contains("Viva")).unwrap();
        assert_eq!(viva.category, "exam");

        let holiday = list.iter().find(|i| i.title.contains("Thanksgiving")).unwrap();
        assert_eq!(holiday.category, "holiday");

        let lab = list.iter().find(|i| i.title.contains("Machine Learning")).unwrap();
        assert_eq!(lab.category, "class");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
