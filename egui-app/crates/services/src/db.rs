//! SQLite storage for reminders, calendar imports, and the file shelf (no Tauri types).

use orbit_core::events::{DropFileResult, ReminderItem, ShelfItem};
use rusqlite::{Connection, params};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Extensions blocked from execution (`Open` and `Open with`).
pub const BLOCKED_EXTENSIONS: &[&str] = &[
    "exe",
    "bat",
    "cmd",
    "msi",
    "vbs",
    "ps1",
    "com",
    "scr",
    "pif",
    "reg",
    "hta",
    "cpl",
    "jar",
    "wsf",
    "vbe",
    "jse",
    "js",
    "lnk",
    "url",
    "wsh",
    "inf",
    "msc",
    "appref-ms",
    "application",
    "gadget",
    "sct",
    "scf",
];

pub fn check_allowed_extension(path: &Path) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if BLOCKED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(format!(
            "Opening executable or script files (.{ext}) from the shelf is blocked for security reasons."
        ));
    }
    Ok(())
}

pub struct DatabaseManager {
    conn: Arc<Mutex<Connection>>,
    shelf_storage_dir: PathBuf,
}

const SCHEMA_SQL: &str = "
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
    added_at INTEGER NOT NULL,
    is_managed_copy INTEGER NOT NULL DEFAULT 0,
    item_kind TEXT NOT NULL DEFAULT 'file'
);
";

#[cfg(target_os = "windows")]
fn dispatch_shell_file(path: &Path, action: &str) -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let path_str = path.to_string_lossy();
    if action == "reveal" {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let explorer = orbit_platform::system::system_root_binary("explorer.exe")?;
        return std::process::Command::new(explorer)
            .arg(format!("/select,{path_str}"))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Could not open File Explorer: {error}"));
    }

    let verb = match action {
        "open" => "open",
        "open_with" => "openas",
        "print" => "print",
        _ => return Err("Unsupported file action.".to_string()),
    };
    let wide_path: Vec<u16> = path_str.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_verb: Vec<u16> = verb.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: ShellExecuteW on validated local file path.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            wide_verb.as_ptr(),
            wide_path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL as i32,
        )
    };
    if (result as isize) <= 32 {
        Err(format!(
            "Windows could not run the {action} action (code: {result:?})."
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn dispatch_shell_file(_path: &Path, _action: &str) -> Result<(), String> {
    Err("This file action is supported on Windows only.".to_string())
}

fn init_connection_pragmas_and_schema(conn: &Connection, is_file: bool) -> Result<(), String> {
    if is_file {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(|e| format!("Failed to configure SQLite WAL mode: {e}"))?;

        let check: String = conn
            .query_row("PRAGMA quick_check(1);", [], |row| row.get(0))
            .map_err(|e| format!("PRAGMA quick_check failed: {e}"))?;
        if check != "ok" {
            return Err(format!("SQLite integrity quick_check reported: {check}"));
        }
    }

    conn.execute_batch(SCHEMA_SQL)
        .map_err(|e| format!("Failed to initialize SQLite schema: {e}"))?;

    let _ = conn.execute(
        "ALTER TABLE file_shelf ADD COLUMN is_managed_copy INTEGER NOT NULL DEFAULT 0",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE file_shelf ADD COLUMN item_kind TEXT NOT NULL DEFAULT 'file'",
        [],
    );
    Ok(())
}

fn quarantine_corrupt_db(db_path: &Path) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let corrupt_name = format!(
        "{}.corrupt.{}",
        db_path.file_name().unwrap_or_default().to_string_lossy(),
        timestamp
    );
    let corrupt_path = db_path.with_file_name(corrupt_name);
    tracing::warn!(
        "Quarantining corrupt database file {:?} to {:?}",
        db_path,
        corrupt_path
    );
    let _ = fs::rename(db_path, &corrupt_path);

    let file_stem = db_path.file_name().unwrap_or_default().to_string_lossy();
    let wal_path = db_path.with_file_name(format!("{file_stem}-wal"));
    if wal_path.exists() {
        let _ = fs::remove_file(&wal_path);
    }
    let shm_path = db_path.with_file_name(format!("{file_stem}-shm"));
    if shm_path.exists() {
        let _ = fs::remove_file(&shm_path);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShelfInputKind {
    File(String),
    Link(String),
    Text(String),
}

/// Detects whether a user `+` input is a local file path, web link, or plain text snippet.
pub fn detect_shelf_input_kind(raw: &str) -> Result<ShelfInputKind, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Input cannot be empty.".to_string());
    }
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| trimmed.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .unwrap_or(trimmed)
        .trim();

    if !unquoted.is_empty() && Path::new(unquoted).is_file() {
        return Ok(ShelfInputKind::File(unquoted.to_string()));
    }

    if !trimmed.contains(char::is_whitespace) {
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("https://") || lower.starts_with("http://") {
            let after_scheme = trimmed.split_once("://").map(|x| x.1).unwrap_or("");
            if after_scheme.is_empty() {
                return Err("Invalid URL.".to_string());
            }
            return Ok(ShelfInputKind::Link(trimmed.to_string()));
        }
        // Check bare domain URLs like github.com/project
        if trimmed.contains('.') && !trimmed.contains('\\') && !trimmed.contains(":\\") {
            let host = trimmed.split('/').next().unwrap_or("");
            if let Some((left, right)) = host.split_once('.')
                && !left.is_empty()
                && right.len() >= 2
                && left.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                && right.split('.').all(|seg| {
                    !seg.is_empty()
                        && seg
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':')
                })
            {
                return Ok(ShelfInputKind::Link(format!("https://{trimmed}")));
            }
        }
    }

    if trimmed.len() > 50_000 {
        return Err("Text snippet exceeds 50,000 character limit.".to_string());
    }
    Ok(ShelfInputKind::Text(trimmed.to_string()))
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

        let primary_result = Connection::open(&db_path)
            .map_err(|e| format!("Open error: {e}"))
            .and_then(|conn| {
                init_connection_pragmas_and_schema(&conn, true)?;
                Ok(conn)
            });

        match primary_result {
            Ok(conn) => Ok(Self {
                conn: Arc::new(Mutex::new(conn)),
                shelf_storage_dir,
            }),
            Err(err) => {
                tracing::warn!(
                    "Primary SQLite database initialization failed at {:?}: {err}. Quarantining...",
                    db_path
                );
                quarantine_corrupt_db(&db_path);
                let recovery_result = Connection::open(&db_path)
                    .map_err(|e| format!("Re-open error: {e}"))
                    .and_then(|conn| {
                        init_connection_pragmas_and_schema(&conn, true)?;
                        Ok(conn)
                    });
                match recovery_result {
                    Ok(conn) => Ok(Self {
                        conn: Arc::new(Mutex::new(conn)),
                        shelf_storage_dir,
                    }),
                    Err(second_err) => {
                        tracing::error!(
                            "Recreation of SQLite database failed at {:?}: {second_err}. Falling back to in-memory database.",
                            db_path
                        );
                        Self::in_memory_with_shelf_dir(shelf_storage_dir)
                    }
                }
            }
        }
    }

    pub fn in_memory() -> Result<Self, String> {
        Self::in_memory_with_shelf_dir(PathBuf::from("shelf_storage"))
    }

    pub fn in_memory_with_shelf_dir(shelf_storage_dir: PathBuf) -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to create in-memory database: {e}"))?;
        init_connection_pragmas_and_schema(&conn, false)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            shelf_storage_dir,
        })
    }

    pub fn default_app_db() -> Result<Self, String> {
        let dir = orbit_core::config::dir().map_err(|e| format!("Could not resolve data dir: {e}"))?;
        Self::new(dir.join("orbit.db"))
    }

    pub fn get_reminders(&self) -> Result<Vec<ReminderItem>, String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
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
        Ok(rows.filter_map(|r| r.ok()).collect())
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
        static REMINDER_COUNTER: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(1);
        let seq = REMINDER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let item = ReminderItem {
            id: format!("rem_{now_nanos}_{seq}"),
            title: clean_title.to_string(),
            due_timestamp_ms,
            is_completed: false,
            source: if source.is_empty() {
                "local".to_string()
            } else {
                source.to_string()
            },
            category: if category.is_empty() {
                "task".to_string()
            } else {
                category.to_string()
            },
            created_at: (now_nanos / 1_000_000) as u64,
        };
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
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
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let current: i32 = conn
            .query_row(
                "SELECT is_completed FROM reminders WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| "Reminder not found".to_string())?;
        let next = if current == 0 { 1 } else { 0 };
        conn.execute(
            "UPDATE reminders SET is_completed = ?1 WHERE id = ?2",
            params![next, id],
        )
        .map_err(|e| format!("Failed to update reminder: {e}"))?;
        Ok(next == 1)
    }

    pub fn delete_reminder(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        conn.execute("DELETE FROM reminders WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete reminder: {e}"))?;
        Ok(())
    }

    pub fn clear_imported_reminders(&self, source_filter: Option<&str>) -> Result<usize, String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(src) = source_filter.filter(|s| !s.trim().is_empty()) {
            conn.execute("DELETE FROM reminders WHERE source = ?1", params![src.trim()])
        } else {
            conn.execute("DELETE FROM reminders WHERE source != 'local'", [])
        }
        .map_err(|e| format!("Failed to clear imported reminders: {e}"))
    }

    pub fn import_calendar_file(&self, raw_path: &str, source_tag: &str) -> Result<usize, String> {
        let canonical = validate_local_safe_path(raw_path)?;
        let path = canonical.as_path();
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read schedule file: {e}"))?;
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
            return Err("No valid events found in the selected file.".to_string());
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let mut inserted_count = 0usize;
        for (idx, (title, due_ms, raw_cat)) in parsed_events.into_iter().enumerate() {
            let clean_title = title.trim();
            if clean_title.is_empty() || due_ms == 0 {
                continue;
            }
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
            let id = format!("imp_{clean_source}_{now}_{idx}");
            if conn
                .execute(
                    "INSERT INTO reminders (id, title, due_timestamp_ms, is_completed, source, category, created_at)
                     VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6)",
                    params![id, clean_title, due_ms as i64, clean_source, category, now as i64],
                )
                .is_ok()
            {
                inserted_count += 1;
            }
        }
        Ok(inserted_count)
    }

    pub fn get_shelf_items(&self) -> Result<Vec<ShelfItem>, String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, file_path, display_name, file_size, extension, added_at, is_managed_copy, item_kind
                 FROM file_shelf
                 ORDER BY added_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let fp: String = row.get(1)?;
                let kind = row
                    .get::<_, String>(7)
                    .unwrap_or_else(|_| "file".to_string());
                let exists = if kind == "file" {
                    Path::new(&fp).exists()
                } else {
                    true
                };
                let is_managed_copy: bool = row.get::<_, i32>(6).unwrap_or(0) != 0;
                Ok(ShelfItem {
                    id: row.get(0)?,
                    file_path: fp,
                    display_name: row.get(2)?,
                    file_size: row.get::<_, i64>(3)? as u64,
                    extension: row.get(4)?,
                    added_at: row.get::<_, i64>(5)? as u64,
                    exists,
                    is_managed_copy,
                    kind,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn add_shelf_item(&self, raw_path: &str) -> Result<ShelfItem, String> {
        self.add_shelf_item_with_mode(raw_path, "reference")
    }

    pub fn add_shelf_item_with_mode(&self, raw_path: &str, mode: &str) -> Result<ShelfItem, String> {
        let canonical = validate_local_safe_path(raw_path)?;
        let path = canonical.as_path();
        let metadata =
            fs::metadata(path).map_err(|e| format!("Could not read file metadata: {e}"))?;
        if !metadata.is_file() {
            return Err("Only regular files may be added to the shelf.".to_string());
        }
        let norm_path = canonical.to_string_lossy().to_string();
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());

        let existing: Option<ShelfItem> = conn
            .query_row(
                "SELECT id, file_path, display_name, file_size, extension, added_at, is_managed_copy, item_kind
                 FROM file_shelf WHERE file_path = ?1",
                params![norm_path],
                |row| {
                    let fp: String = row.get(1)?;
                    let kind = row
                        .get::<_, String>(7)
                        .unwrap_or_else(|_| "file".to_string());
                    let exists = if kind == "file" {
                        Path::new(&fp).exists()
                    } else {
                        true
                    };
                    let is_managed_copy = row.get::<_, i32>(6).unwrap_or(0) != 0;
                    Ok(ShelfItem {
                        id: row.get(0)?,
                        file_path: fp,
                        display_name: row.get(2)?,
                        file_size: row.get::<_, i64>(3)? as u64,
                        extension: row.get(4)?,
                        added_at: row.get::<_, i64>(5)? as u64,
                        exists,
                        is_managed_copy,
                        kind,
                    })
                },
            )
            .ok();
        if let Some(item) = existing {
            return Ok(item);
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
                .map(|c| {
                    if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            let dest_path = self.shelf_storage_dir.join(format!("{now_nanos}_{safe_name}"));
            let temp_path = self.shelf_storage_dir.join(format!(".{now_nanos}.partial"));
            if let Err(error) = fs::copy(&canonical, &temp_path) {
                let _ = fs::remove_file(&temp_path);
                return Err(format!("Failed to copy file to managed shelf storage: {error}"));
            }
            if let Err(error) = fs::rename(&temp_path, &dest_path) {
                let _ = fs::remove_file(&temp_path);
                return Err(format!(
                    "Failed to finish copying file into Shelf storage: {error}"
                ));
            }
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
            kind: "file".to_string(),
        };

        conn.execute(
            "INSERT INTO file_shelf (id, file_path, display_name, file_size, extension, added_at, is_managed_copy, item_kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'file')",
            params![
                item.id,
                item.file_path,
                item.display_name,
                item.file_size as i64,
                item.extension,
                item.added_at as i64,
                if item.is_managed_copy { 1 } else { 0 },
            ],
        )
        .map_err(|e| {
            if item.is_managed_copy {
                let _ = fs::remove_file(&item.file_path);
            }
            format!("Failed to insert shelf item: {e}")
        })?;
        Ok(item)
    }

    pub fn add_shelf_entry(&self, raw: &str, mode: &str) -> Result<ShelfItem, String> {
        match detect_shelf_input_kind(raw)? {
            ShelfInputKind::File(path) => self.add_shelf_item_with_mode(&path, mode),
            ShelfInputKind::Link(url) => {
                let display = url
                    .strip_prefix("https://")
                    .or_else(|| url.strip_prefix("http://"))
                    .unwrap_or(&url)
                    .to_string();
                self.insert_virtual_shelf_item(&url, &display, 0, "link", "link")
            }
            ShelfInputKind::Text(snippet) => {
                let single_line: String = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
                let display: String = single_line.chars().take(64).collect();
                self.insert_virtual_shelf_item(
                    &snippet,
                    &display,
                    snippet.len() as u64,
                    "txt",
                    "text",
                )
            }
        }
    }

    fn insert_virtual_shelf_item(
        &self,
        payload: &str,
        display_name: &str,
        file_size: u64,
        extension: &str,
        kind: &str,
    ) -> Result<ShelfItem, String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let existing: Option<ShelfItem> = conn
            .query_row(
                "SELECT id, file_path, display_name, file_size, extension, added_at, is_managed_copy, item_kind
                 FROM file_shelf WHERE file_path = ?1",
                params![payload],
                |row| {
                    let item_kind = row
                        .get::<_, String>(7)
                        .unwrap_or_else(|_| kind.to_string());
                    Ok(ShelfItem {
                        id: row.get(0)?,
                        file_path: row.get(1)?,
                        display_name: row.get(2)?,
                        file_size: row.get::<_, i64>(3)? as u64,
                        extension: row.get(4)?,
                        added_at: row.get::<_, i64>(5)? as u64,
                        exists: true,
                        is_managed_copy: false,
                        kind: item_kind,
                    })
                },
            )
            .ok();
        if let Some(item) = existing {
            return Ok(item);
        }

        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let item = ShelfItem {
            id: format!("shelf_{now_nanos}"),
            file_path: payload.to_string(),
            display_name: display_name.to_string(),
            file_size,
            extension: extension.to_string(),
            added_at: (now_nanos / 1_000_000) as u64,
            exists: true,
            is_managed_copy: false,
            kind: kind.to_string(),
        };
        conn.execute(
            "INSERT INTO file_shelf (id, file_path, display_name, file_size, extension, added_at, is_managed_copy, item_kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
            params![
                item.id,
                item.file_path,
                item.display_name,
                item.file_size as i64,
                item.extension,
                item.added_at as i64,
                item.kind,
            ],
        )
        .map_err(|e| format!("Failed to insert shelf entry: {e}"))?;
        Ok(item)
    }

    pub fn handle_dropped_files(
        &self,
        paths: &[String],
        action: &str,
        storage_mode: &str,
    ) -> Vec<DropFileResult> {
        paths
            .iter()
            .map(|raw_path| {
                let display_name = Path::new(raw_path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| raw_path.clone());
                match self.handle_dropped_file(raw_path, action, storage_mode) {
                    Ok(shelf_item) => DropFileResult {
                        path: raw_path.clone(),
                        display_name,
                        error: None,
                        shelf_item,
                    },
                    Err(error) => DropFileResult {
                        path: raw_path.clone(),
                        display_name,
                        error: Some(error),
                        shelf_item: None,
                    },
                }
            })
            .collect()
    }

    fn handle_dropped_file(
        &self,
        raw_path: &str,
        action: &str,
        storage_mode: &str,
    ) -> Result<Option<ShelfItem>, String> {
        let canonical = validate_local_safe_path(raw_path)?;
        if !canonical.is_file() {
            return Err("Only regular files can be dropped here.".to_string());
        }
        check_allowed_extension(&canonical)?;
        match action {
            "shelf" => self
                .add_shelf_item_with_mode(&canonical.to_string_lossy(), storage_mode)
                .map(Some),
            "open" | "open_with" | "print" | "reveal" => {
                dispatch_shell_file(&canonical, action)?;
                Ok(None)
            }
            _ => Err("Unsupported drop target.".to_string()),
        }
    }

    pub fn remove_shelf_item(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        let existing: Option<(String, bool)> = conn
            .query_row(
                "SELECT file_path, is_managed_copy FROM file_shelf WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get::<_, i32>(1).unwrap_or(0) != 0)),
            )
            .ok();
        conn.execute("DELETE FROM file_shelf WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to remove shelf item: {e}"))?;
        if let Some((fp, is_managed_copy)) = existing
            && is_managed_copy
            && let (Ok(target_canon), Ok(storage_canon)) = (
                dunce::canonicalize(Path::new(&fp)),
                dunce::canonicalize(&self.shelf_storage_dir),
            )
            && target_canon.starts_with(&storage_canon)
            && target_canon != storage_canon
        {
            let _ = fs::remove_file(&target_canon);
        }
        Ok(())
    }

    fn resolve_shelf_entry(&self, id: &str) -> Result<(String, String), String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        conn.query_row(
            "SELECT file_path, COALESCE(item_kind, 'file') FROM file_shelf WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "Shelf item not found".to_string())
    }

    fn resolve_shelf_file_path(&self, id: &str) -> Result<PathBuf, String> {
        let (file_path, kind) = self.resolve_shelf_entry(id)?;
        if kind != "file" {
            return Err("Only local files have a path on disk.".to_string());
        }
        let path = PathBuf::from(&file_path);
        if !path.exists() {
            return Err(
                "File not found at its original location. It may have been moved or deleted."
                    .to_string(),
            );
        }
        Ok(path)
    }

    pub fn open_shelf_file(&self, id: &str) -> Result<(), String> {
        let (target, kind) = self.resolve_shelf_entry(id)?;
        if kind == "link" {
            return orbit_platform::system::open_external_url(&target);
        }
        if kind == "text" {
            return Ok(());
        }
        let path = PathBuf::from(&target);
        if !path.exists() {
            return Err(
                "File not found at its original location. It may have been moved or deleted."
                    .to_string(),
            );
        }
        check_allowed_extension(&path)?;
        dispatch_shell_file(&path, "open")
    }

    pub fn open_with_shelf_file(&self, id: &str) -> Result<(), String> {
        let path = self.resolve_shelf_file_path(id)?;
        check_allowed_extension(&path)?;
        dispatch_shell_file(&path, "open_with")
    }

    pub fn show_shelf_file_in_folder(&self, id: &str) -> Result<(), String> {
        let path = self.resolve_shelf_file_path(id)?;
        dispatch_shell_file(&path, "reveal")
    }

    pub fn clear_shelf(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        conn.execute("DELETE FROM file_shelf", [])
            .map_err(|e| format!("Failed to clear file shelf: {e}"))?;
        if self.shelf_storage_dir.exists() {
            let _ = fs::remove_dir_all(&self.shelf_storage_dir);
        }
        Ok(())
    }

    pub fn clear_local_data(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap_or_else(|p| p.into_inner());
        conn.execute_batch(
            "DELETE FROM reminders;
             DELETE FROM file_shelf;",
        )
        .map_err(|e| format!("Failed to clear local database tables: {e}"))?;
        if self.shelf_storage_dir.exists() {
            let _ = fs::remove_dir_all(&self.shelf_storage_dir);
        }
        Ok(())
    }
}

pub fn validate_local_safe_path(raw_path: &str) -> Result<PathBuf, String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    if trimmed.contains('\0') {
        return Err("Path contains null character".to_string());
    }
    if trimmed.starts_with(r"\\") || trimmed.starts_with("//") {
        return Err(format!("UNC network paths are not allowed: {trimmed}"));
    }
    let norm_slashes = trimmed.replace('/', "\\");
    if norm_slashes.starts_with(r"\\.\") || norm_slashes.starts_with(r"\\?\") {
        return Err(format!("Device namespace paths are not allowed: {trimmed}"));
    }
    if trimmed.contains("..") {
        return Err("Directory traversal ('..') is not permitted".to_string());
    }
    for component in Path::new(trimmed).components() {
        if let std::path::Component::ParentDir = component {
            return Err("Directory traversal ('..') is not permitted".to_string());
        }
    }
    const DOS_DEVICES: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let path_obj = Path::new(trimmed);
    for comp in path_obj.components() {
        let os_str = comp.as_os_str().to_string_lossy();
        let stem = Path::new(&*os_str)
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or(os_str.clone());
        let stem_upper = stem.trim().to_ascii_uppercase();
        if DOS_DEVICES.contains(&stem_upper.as_str()) {
            return Err(format!(
                "Reserved DOS device name is not allowed: {stem_upper}"
            ));
        }
    }
    if !path_obj.exists() {
        return Err(format!("File does not exist: {trimmed}"));
    }
    let canonical = dunce::canonicalize(path_obj)
        .map_err(|e| format!("Failed to canonicalize path: {e}"))?;
    let canon_str = canonical.to_string_lossy();
    if canon_str.starts_with(r"\\") && !canon_str.starts_with(r"\\?\") {
        return Err(format!(
            "Path resolved to forbidden UNC location: {canon_str}"
        ));
    }
    Ok(canonical)
}

mod dunce {
    use std::path::{Path, PathBuf};
    pub fn canonicalize<P: AsRef<Path>>(path: P) -> std::io::Result<PathBuf> {
        std::fs::canonicalize(path).map(|p| {
            let s = p.to_string_lossy();
            if let Some(stripped) = s.strip_prefix(r"\\?\") {
                PathBuf::from(stripped)
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

fn ymd_hms_to_epoch_ms(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: u32,
    is_utc: bool,
) -> u64 {
    if !(1970..=2100).contains(&year) || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return 0;
    }
    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = (era as i64) * 146097 + (doe as i64) - 719468;
    if days < 0 {
        return 0;
    }
    let total_secs = days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64);
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
        struct SystemTime {
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
        struct TimeZoneInformation {
            bias: i32,
            standard_name: [u16; 32],
            standard_date: SystemTime,
            standard_bias: i32,
            daylight_name: [u16; 32],
            daylight_date: SystemTime,
            daylight_bias: i32,
        }
        unsafe extern "system" {
            fn GetTimeZoneInformation(lp_time_zone_information: *mut TimeZoneInformation) -> u32;
        }
        // SAFETY: valid out pointer.
        unsafe {
            let mut tzi: TimeZoneInformation = std::mem::zeroed();
            let res = GetTimeZoneInformation(&mut tzi);
            let active_bias = if res == 2 {
                tzi.bias + tzi.daylight_bias
            } else {
                tzi.bias + tzi.standard_bias
            };
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
    let is_utc = s.ends_with('Z') || s.ends_with('z');
    let clean_ical = s.trim_end_matches(['Z', 'z']);
    if clean_ical.len() >= 8 && clean_ical.chars().take(8).all(|c| c.is_ascii_digit()) {
        let year = clean_ical[0..4].parse::<i32>().unwrap_or(0);
        let month = clean_ical[4..6].parse::<u32>().unwrap_or(0);
        let day = clean_ical[6..8].parse::<u32>().unwrap_or(0);
        let mut hour = 9u32;
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
                return ymd_hms_to_epoch_ms(p2, p1, p0, hour, min, 0, false);
            }
        }
    }
    0
}

fn parse_ics_events(content: &str) -> Vec<(String, u64, String)> {
    let unfolded = content
        .replace("\r\n ", "")
        .replace("\n ", "")
        .replace("\r\n\t", "")
        .replace("\n\t", "");
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
                events.push((
                    summary.trim().to_string(),
                    dtstart_ms,
                    categories.trim().to_string(),
                ));
            }
            in_event = false;
        } else if in_event && let Some((prop, val)) = line.split_once(':') {
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
        let cols: Vec<&str> = line
            .split(',')
            .map(|c| c.trim().trim_matches('"'))
            .collect();
        if cols.len() < 2 {
            continue;
        }
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
                    if col.contains("title")
                        || col.contains("event")
                        || col.contains("subject")
                        || col.contains("summary")
                        || col.contains("name")
                    {
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
                format!("{date_part} {t_val}")
            }
        } else {
            date_part.to_string()
        };
        let mut ms = parse_flexible_datetime(&combined_dt);
        let mut final_title = title.to_string();
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
    fn corrupt_db_quarantines_and_recovers() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("orbit_db_corrupt_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("orbit.db");
        fs::write(&db_path, b"not a valid sqlite file").unwrap();

        let db = DatabaseManager::new(db_path).expect("should recover");
        db.add_reminder("Recovered", 1700000000000, "task", "local")
            .unwrap();
        assert_eq!(db.get_reminders().unwrap().len(), 1);
        let quarantined = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().contains("orbit.db.corrupt."));
        assert!(quarantined);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_local_safe_path_rejects_unsafe_inputs() {
        assert!(validate_local_safe_path(r"\\127.0.0.1\c$\x.txt").is_err());
        assert!(validate_local_safe_path(r"\\.\COM1").is_err());
        assert!(validate_local_safe_path(r"..\secret.txt").is_err());
        assert!(validate_local_safe_path("CON.txt").is_err());
    }
}
