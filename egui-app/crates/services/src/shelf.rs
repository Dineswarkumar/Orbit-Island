//! File shelf service wrapping `DatabaseManager` with blocked-extension enforcement.

use crate::db::{self, DatabaseManager};
use crate::windows_share;
use crossbeam_channel::Sender;
use orbit_core::config::Config;
use orbit_core::events::{CoreEvent, DropFileResult, ShelfItem, WakeFn};
use std::path::Path;
use std::sync::{Arc, RwLock};

pub use db::{BLOCKED_EXTENSIONS, check_allowed_extension};

pub struct ShelfService {
    db: Arc<DatabaseManager>,
    config: Arc<RwLock<Config>>,
    tx: Sender<CoreEvent>,
    wake: WakeFn,
}

impl ShelfService {
    pub fn new(
        db: Arc<DatabaseManager>,
        config: Arc<RwLock<Config>>,
        tx: Sender<CoreEvent>,
        wake: WakeFn,
    ) -> Self {
        Self { db, config, tx, wake }
    }

    fn publish_latest(&self) {
        if let Ok(items) = self.db.get_shelf_items() {
            let _ = self.tx.send(CoreEvent::ShelfUpdated(items));
            (self.wake)();
        }
    }

    fn storage_mode(&self) -> String {
        self.config
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .shelf_storage_mode
            .clone()
    }

    pub fn get_items(&self) -> Result<Vec<ShelfItem>, String> {
        self.db.get_shelf_items()
    }

    pub fn add_item(&self, file_path: &str) -> Result<ShelfItem, String> {
        let mode = self.storage_mode();
        let item = self.db.add_shelf_item_with_mode(file_path, &mode)?;
        self.publish_latest();
        Ok(item)
    }

    pub fn add_entry(&self, raw: &str) -> Result<ShelfItem, String> {
        let mode = self.storage_mode();
        let item = self.db.add_shelf_entry(raw, &mode)?;
        self.publish_latest();
        Ok(item)
    }

    pub fn handle_file_drop(
        &self,
        paths: &[String],
        action: &str,
    ) -> Vec<DropFileResult> {
        let mode = self.storage_mode();
        let res = self.db.handle_dropped_files(paths, action, &mode);
        self.publish_latest();
        res
    }

    pub fn open_file(&self, id: &str) -> Result<(), String> {
        self.db.open_shelf_file(id)
    }

    pub fn open_with_file(&self, id: &str) -> Result<(), String> {
        self.db.open_with_shelf_file(id)
    }

    pub fn reveal_in_folder(&self, id: &str) -> Result<(), String> {
        self.db.show_shelf_file_in_folder(id)
    }

    pub fn remove_item(&self, id: &str) -> Result<(), String> {
        self.db.remove_shelf_item(id)?;
        self.publish_latest();
        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        self.db.clear_shelf()?;
        self.publish_latest();
        Ok(())
    }

    pub fn share_files(&self, hwnd: isize, paths: &[String]) -> Result<(), String> {
        let validated = paths
            .iter()
            .map(|raw| {
                let p = db::validate_local_safe_path(raw)?;
                if !p.is_file() {
                    return Err("Only regular files can be shared.".to_string());
                }
                Ok(p.to_string_lossy().into_owned())
            })
            .collect::<Result<Vec<_>, String>>()?;
        windows_share::share_files_for_window(hwnd, validated)
    }

    pub fn is_extension_allowed(path: &Path) -> Result<(), String> {
        check_allowed_extension(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("orbit_shelf_{name}_{nanos}"))
    }

    #[test]
    fn dangerous_extensions_including_js_and_lnk_blocked_for_open_and_open_with() {
        let dir = temp_dir("blocked_exts");
        fs::create_dir_all(&dir).unwrap();
        let db = Arc::new(DatabaseManager::new(dir.join("shelf.db")).unwrap());
        let cfg = Arc::new(RwLock::new(Config::default()));
        let (tx, _rx) = crossbeam_channel::unbounded();
        let wake: WakeFn = Arc::new(|| {});
        let shelf = ShelfService::new(db, cfg, tx, wake);

        let all_blocked = [
            "exe", "bat", "cmd", "msi", "vbs", "ps1", "com", "scr", "pif", "reg", "hta", "cpl",
            "jar", "wsf", "vbe", "jse", "js", "lnk", "url", "wsh", "inf", "msc", "appref-ms",
            "application", "gadget", "sct", "scf",
        ];

        for ext in all_blocked {
            let f = dir.join(format!("sample.{ext}"));
            fs::write(&f, b"dummy").unwrap();
            let item = shelf.add_item(&f.to_string_lossy()).unwrap();

            let open_err = shelf.open_file(&item.id).unwrap_err();
            assert!(
                open_err.contains("blocked for security"),
                "open failed to block .{ext}: {open_err}"
            );

            let open_with_err = shelf.open_with_file(&item.id).unwrap_err();
            assert!(
                open_with_err.contains("blocked for security"),
                "open_with failed to block .{ext}: {open_with_err}"
            );

            let drop_open_with = shelf.handle_file_drop(&[f.to_string_lossy().into_owned()], "open_with");
            assert!(
                drop_open_with[0]
                    .error
                    .as_deref()
                    .unwrap_or("")
                    .contains("blocked for security"),
                "drop open_with failed to block .{ext}"
            );
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn drop_js_file_onto_shelf_is_refused_with_message() {
        let dir = temp_dir("drop_js");
        fs::create_dir_all(&dir).unwrap();
        let db = Arc::new(DatabaseManager::new(dir.join("shelf.db")).unwrap());
        let cfg = Arc::new(RwLock::new(Config::default()));
        let (tx, _rx) = crossbeam_channel::unbounded();
        let shelf = ShelfService::new(db, cfg, tx, Arc::new(|| {}));

        let js = dir.join("evil.js");
        fs::write(&js, b"alert(1);").unwrap();
        let res = shelf.handle_file_drop(&[js.to_string_lossy().into_owned()], "shelf");
        assert!(res[0].shelf_item.is_none());
        let err = res[0].error.as_deref().unwrap_or("");
        assert!(err.contains("blocked for security"), "got: {err}");
        assert!(shelf.get_items().unwrap().is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn drop_three_files_at_once_adds_all_three() {
        let dir = temp_dir("drop_three");
        fs::create_dir_all(&dir).unwrap();
        let db = Arc::new(DatabaseManager::new(dir.join("shelf.db")).unwrap());
        let cfg = Arc::new(RwLock::new(Config::default()));
        let (tx, _rx) = crossbeam_channel::unbounded();
        let shelf = ShelfService::new(db, cfg, tx, Arc::new(|| {}));

        let mut paths = Vec::new();
        for name in ["a.txt", "b.pdf", "c.png"] {
            let p = dir.join(name);
            fs::write(&p, b"hello").unwrap();
            paths.push(p.to_string_lossy().into_owned());
        }
        let res = shelf.handle_file_drop(&paths, "shelf");
        assert_eq!(res.len(), 3);
        assert!(res.iter().all(|r| r.error.is_none() && r.shelf_item.is_some()));
        assert_eq!(shelf.get_items().unwrap().len(), 3);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_entry_detects_file_link_and_text() {
        let dir = temp_dir("detect_kind");
        fs::create_dir_all(&dir).unwrap();
        let db = Arc::new(DatabaseManager::new(dir.join("shelf.db")).unwrap());
        let cfg = Arc::new(RwLock::new(Config::default()));
        let (tx, _rx) = crossbeam_channel::unbounded();
        let shelf = ShelfService::new(db, cfg, tx, Arc::new(|| {}));

        let f = dir.join("note.txt");
        fs::write(&f, b"abc").unwrap();
        let it_file = shelf.add_entry(&f.to_string_lossy()).unwrap();
        assert_eq!(it_file.kind, "file");

        let it_link = shelf.add_entry("github.com/project").unwrap();
        assert_eq!(it_link.kind, "link");
        assert_eq!(it_link.file_path, "https://github.com/project");
        assert_eq!(it_link.display_name, "github.com/project");

        let it_text = shelf.add_entry("Quick sprint note: check tests").unwrap();
        assert_eq!(it_text.kind, "text");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_missing_file_returns_clear_error_without_panicking() {
        let dir = temp_dir("missing_file");
        fs::create_dir_all(&dir).unwrap();
        let db = Arc::new(DatabaseManager::new(dir.join("shelf.db")).unwrap());
        let cfg = Arc::new(RwLock::new(Config::default()));
        let (tx, _rx) = crossbeam_channel::unbounded();
        let shelf = ShelfService::new(db, cfg, tx, Arc::new(|| {}));

        let f = dir.join("gone.txt");
        fs::write(&f, b"temp").unwrap();
        let item = shelf.add_item(&f.to_string_lossy()).unwrap();
        fs::remove_file(&f).unwrap();

        let listed = shelf.get_items().unwrap();
        assert_eq!(listed.len(), 1);
        assert!(!listed[0].exists);

        let err = shelf.open_file(&item.id).unwrap_err();
        assert!(err.contains("not found"), "got: {err}");

        let _ = fs::remove_dir_all(&dir);
    }
}
