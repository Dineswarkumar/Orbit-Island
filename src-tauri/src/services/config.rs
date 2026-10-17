use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Manager};

static TEMP_FILE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub theme: String,
    pub position: String,
    pub density: String,
    pub time_format: String,
    pub always_on_top: bool,
    pub launch_at_startup: bool,
    pub show_hardware_monitor: bool,
    pub transparency_mode: String,
    pub notifications_enabled: bool,
    pub privacy_mode: bool,
    pub notification_preview_duration_sec: u32,
    pub show_privacy_indicators: bool,
    pub blocked_apps: Vec<String>,
    pub surface_opacity: u32,
}

impl AppConfig {
    fn validate(&self) -> Result<(), String> {
        let valid = [
            self.theme.as_str(),
            self.position.as_str(),
            self.density.as_str(),
            self.time_format.as_str(),
            self.transparency_mode.as_str(),
        ];
        if ![
            "obsidian", "dark", "dynamic", "light", "black", "espresso", "evergreen", "nord",
            "custom",
        ]
        .contains(&valid[0])
        {
            return Err("Unsupported theme".to_string());
        }
        if !["top-center", "top-left", "top-right"].contains(&valid[1]) {
            return Err("Unsupported island position".to_string());
        }
        if !["compact", "balanced", "detailed"].contains(&valid[2]) {
            return Err("Unsupported density".to_string());
        }
        if !["12h", "24h"].contains(&valid[3]) {
            return Err("Unsupported time format".to_string());
        }
        if !["glass", "frosted", "smoked", "solid"].contains(&valid[4]) {
            return Err("Unsupported transparency mode".to_string());
        }
        if self.notification_preview_duration_sec < 2 || self.notification_preview_duration_sec > 30 {
            return Err("Invalid notification preview duration".to_string());
        }
        if ![20, 50, 80, 100].contains(&self.surface_opacity) {
            return Err("Unsupported surface opacity preset. Must be 20, 50, 80, or 100".to_string());
        }
        Ok(())
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "obsidian".to_string(),
            position: "top-center".to_string(),
            density: "balanced".to_string(),
            time_format: "12h".to_string(),
            always_on_top: true,
            launch_at_startup: false,
            show_hardware_monitor: true,
            transparency_mode: "glass".to_string(),
            notifications_enabled: true,
            privacy_mode: false,
            notification_preview_duration_sec: 4,
            show_privacy_indicators: true,
            blocked_apps: Vec::new(),
            surface_opacity: 80,
        }
    }
}

pub fn get_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

    if !app_dir.exists() {
        fs::create_dir_all(&app_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    Ok(app_dir.join("config.json"))
}

pub fn load_config(app: &AppHandle) -> AppConfig {
    match get_config_path(app) {
        Ok(path) => {
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                        Ok(config) if config.validate().is_ok() => config,
                        Ok(_) => {
                            log::warn!("Ignoring invalid ORBIT config values; using defaults");
                            AppConfig::default()
                        }
                        Err(error) => {
                            log::warn!("Ignoring unreadable ORBIT config: {error}");
                            AppConfig::default()
                        }
                    },
                    Err(error) => {
                        log::warn!("Could not read ORBIT config: {error}");
                        AppConfig::default()
                    }
                }
            } else {
                let default_cfg = AppConfig::default();
                let _ = save_config(app, &default_cfg);
                default_cfg
            }
        }
        Err(_) => AppConfig::default(),
    }
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    config.validate()?;
    let path = get_config_path(app)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    sync_startup_registry(config.launch_at_startup);

    atomic_write(&path, content.as_bytes())
}

#[cfg(target_os = "windows")]
pub fn sync_startup_registry(enable: bool) {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, RegCloseKey, RegDeleteValueW, RegOpenKeyExW,
        RegSetValueExW, REG_SZ,
    };

    let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "ORBIT\0".encode_utf16().collect();

    unsafe {
        let mut hkey = 0 as HKEY;
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if status != ERROR_SUCCESS {
            log::warn!("Failed to open Windows Run registry key: status {status}");
            return;
        }

        if enable {
            if let Ok(current_exe) = std::env::current_exe() {
                let exe_str = format!("\"{}\"", current_exe.to_string_lossy());
                let exe_wide: Vec<u16> = exe_str.encode_utf16().chain(std::iter::once(0)).collect();
                let byte_len = (exe_wide.len() * std::mem::size_of::<u16>()) as u32;
                let set_status = RegSetValueExW(
                    hkey,
                    value_name.as_ptr(),
                    0,
                    REG_SZ,
                    exe_wide.as_ptr() as *const u8,
                    byte_len,
                );
                if set_status != ERROR_SUCCESS {
                    log::warn!("Failed to set ORBIT startup registry value: {set_status}");
                }
            }
        } else {
            let _ = RegDeleteValueW(hkey, value_name.as_ptr());
        }

        RegCloseKey(hkey);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn sync_startup_registry(_enable: bool) {}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    let id = TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.json");
    let temporary_path = path.with_file_name(format!("{file_name}.{}.{}.tmp", std::process::id(), id));

    let write_result = (|| -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary_path)
            .map_err(|e| format!("Failed to create temporary config: {e}"))?;

        file.write_all(content)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Failed to write temporary config: {e}"))?;

        // Explicitly drop file handle to release Windows file locks
        drop(file);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            let temp_wide: Vec<u16> = temporary_path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let target_wide: Vec<u16> = path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            if path.exists() {
                let success = unsafe {
                    windows_sys::Win32::Storage::FileSystem::ReplaceFileW(
                        target_wide.as_ptr(),
                        temp_wide.as_ptr(),
                        std::ptr::null(),
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                if success != 0 {
                    return Ok(());
                }

                // If ReplaceFileW failed because target disappeared, fall back to MoveFileExW
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() == Some(2) {
                    let move_res = unsafe {
                        windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                            temp_wide.as_ptr(),
                            target_wide.as_ptr(),
                            windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                                | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
                        )
                    };
                    if move_res != 0 {
                        return Ok(());
                    }
                } else {
                    // Try MoveFileExW as fallback
                    let move_res = unsafe {
                        windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                            temp_wide.as_ptr(),
                            target_wide.as_ptr(),
                            windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                                | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
                        )
                    };
                    if move_res != 0 {
                        return Ok(());
                    }
                }

                return Err(format!("Failed to replace config file: {err}"));
            }

            // Destination does not exist: Move temp to target
            let move_res = unsafe {
                windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                    temp_wide.as_ptr(),
                    target_wide.as_ptr(),
                    windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                        | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
                )
            };
            if move_res != 0 {
                Ok(())
            } else {
                Err(format!(
                    "Failed to move config file to destination: {}",
                    std::io::Error::last_os_error()
                ))
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            fs::rename(&temporary_path, path).map_err(|e| format!("Failed to replace config file: {e}"))
        }
    })();

    // Always clean up the temporary file if anything failed
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }

    write_result
}

#[cfg(test)]
mod tests {
    use super::{atomic_write, AppConfig};
    use std::fs;

    #[test]
    fn defaults_are_valid_and_unknown_theme_is_rejected() {
        assert!(AppConfig::default().validate().is_ok());
        let mut config = AppConfig::default();
        config.theme = "unknown".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn first_write_when_no_file_exists() {
        let path = std::env::temp_dir().join(format!("orbit-config-first-{}.json", std::process::id()));
        let _ = fs::remove_file(&path);
        assert!(!path.exists());

        atomic_write(&path, b"initial content").unwrap();
        assert!(path.exists());
        assert_eq!(fs::read(&path).unwrap(), b"initial content");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn replacing_existing_file_on_windows() {
        let path = std::env::temp_dir().join(format!("orbit-config-replace-{}.json", std::process::id()));
        let _ = fs::remove_file(&path);

        atomic_write(&path, b"first version").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first version");

        atomic_write(&path, b"second version").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second version");

        atomic_write(&path, b"third version").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"third version");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn preserving_prior_file_if_replacement_fails() {
        let path = std::env::temp_dir().join(format!("orbit-config-preserve-{}.json", std::process::id()));
        let _ = fs::remove_file(&path);

        atomic_write(&path, b"safe original").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"safe original");

        // Attempting to write into an invalid path where directory does not exist fails
        let invalid_dir = path.join("sub").join("nonexistent_path.json");
        let result = atomic_write(&invalid_dir, b"failed write");
        assert!(result.is_err());

        // The original file is completely intact and preserved
        assert_eq!(fs::read(&path).unwrap(), b"safe original");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn surface_opacity_presets_validated_and_backward_compatible() {
        // Backward-compatible: missing surfaceOpacity in old JSON defaults to 80
        let old_json = r#"{"theme":"obsidian","position":"top-center"}"#;
        let config: AppConfig = serde_json::from_str(old_json).unwrap();
        assert_eq!(config.surface_opacity, 80);
        assert!(config.validate().is_ok());

        // Valid presets: 20, 50, 80, 100
        for valid_preset in [20, 50, 80, 100] {
            let mut cfg = config.clone();
            cfg.surface_opacity = valid_preset;
            assert!(cfg.validate().is_ok());
        }

        // Invalid presets rejected
        for invalid_preset in [0, 10, 45, 75, 90, 120] {
            let mut cfg = config.clone();
            cfg.surface_opacity = invalid_preset;
            assert!(cfg.validate().is_err());
        }
    }
}
