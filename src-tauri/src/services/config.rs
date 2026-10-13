use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub theme: String,
    pub position: String,
    pub density: String,
    pub time_format: String,
    pub always_on_top: bool,
    pub launch_at_startup: bool,
    pub show_hardware_monitor: bool,
    pub transparency_mode: String,
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
                    Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                    Err(_) => AppConfig::default(),
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
    let path = get_config_path(app)?;
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write config to file: {}", e))?;

    Ok(())
}
