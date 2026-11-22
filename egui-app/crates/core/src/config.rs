// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Adapted from nook/crates/app/src/config.rs for ORBIT.

//! User configuration in `%APPDATA%\ORBIT\config.toml`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Dark,
    Light,
    #[default]
    Auto,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExpandTrigger {
    #[default]
    Hover,
    Click,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlassMode {
    #[default]
    Normal,
    Realistic,
    Blurred,
    Opaque,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorTheme {
    #[default]
    Obsidian,
    OledBlack,
    LinearIndigo,
    VisionSpatial,
    CatppuccinMocha,
    Cyberpunk,
    SunsetVelvet,
    EmeraldAurora,
    CeramicPearl,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionPreset {
    #[default]
    LiquidSpring,
    Snappy,
    Bouncy,
    Smooth,
    Reduced,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeFormat {
    #[default]
    Hour12,
    Hour24,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactRightSlot {
    #[default]
    Weather,
    Battery,
    OrbitLogo,
    WindowsLogo,
    AppleLogo,
    LinuxLogo,
    SmartAuto,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsAppearance {
    #[default]
    Dark,
    System,
    Light,
    Orbit,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeMode,
    pub placement: Placement,
    pub on_top: bool,
    pub hover_delay_ms: u32,
    pub collapse_grace_ms: u32,
    pub launch_at_startup: bool,
    pub pause_in_fullscreen: bool,
    pub show_privacy_indicators: bool,
    pub notifications_enabled: bool,
    pub privacy_mode: bool,
    pub preview_privacy: bool,
    pub blocked_apps: Vec<String>,
    pub shelf_storage_mode: String,
    pub weather_enabled: bool,
    pub weather_unit: WeatherUnit,
    pub expand_trigger: ExpandTrigger,
    pub notification_preview_secs: u32,
    pub glass_mode: GlassMode,
    pub hide_from_screen_capture: bool,
    pub color_theme: ColorTheme,
    pub motion_preset: MotionPreset,
    pub time_format: TimeFormat,
    pub compact_right_slot: CompactRightSlot,
    pub utility_alerts_enabled: bool,
    pub settings_appearance: SettingsAppearance,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Auto,
            placement: Placement::Center,
            on_top: true,
            hover_delay_ms: 120,
            collapse_grace_ms: 250,
            launch_at_startup: false,
            pause_in_fullscreen: true,
            show_privacy_indicators: true,
            notifications_enabled: true,
            privacy_mode: false,
            preview_privacy: false,
            blocked_apps: Vec::new(),
            shelf_storage_mode: "reference".to_string(),
            weather_enabled: true,
            weather_unit: WeatherUnit::Celsius,
            expand_trigger: ExpandTrigger::Hover,
            notification_preview_secs: 5,
            glass_mode: GlassMode::Normal,
            hide_from_screen_capture: false,
            color_theme: ColorTheme::Obsidian,
            motion_preset: MotionPreset::LiquidSpring,
            time_format: TimeFormat::Hour12,
            compact_right_slot: CompactRightSlot::Weather,
            utility_alerts_enabled: true,
            settings_appearance: SettingsAppearance::Dark,
        }
    }
}

pub fn dir() -> Result<PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Ok(PathBuf::from(appdata).join("ORBIT"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("orbit"));
    }
    let home = std::env::var_os("HOME").context("APPDATA / HOME is not set")?;
    Ok(PathBuf::from(home).join(".config").join("orbit"))
}

pub fn path() -> Result<PathBuf> {
    Ok(dir()?.join("config.toml"))
}

impl Config {
    /// Clamps numeric bounds and canonicalizes string enums after deserializing.
    pub fn normalized(mut self) -> Self {
        self.hover_delay_ms = self.hover_delay_ms.min(1000);
        self.collapse_grace_ms = self.collapse_grace_ms.clamp(50, 1500);
        self.notification_preview_secs = self.notification_preview_secs.clamp(2, 15);
        if self.shelf_storage_mode != "copy" && self.shelf_storage_mode != "reference" {
            self.shelf_storage_mode = "reference".to_string();
        }
        self
    }

    /// Loads the config from the default path, writing defaults on first run.
    /// A malformed file is renamed to `config.toml.bad` and defaults are used.
    pub fn load() -> Result<Self> {
        let path = path()?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => match toml::from_str::<Config>(&text) {
                Ok(cfg) => Ok(cfg.normalized()),
                Err(e) => {
                    tracing::error!("config parse error in {}, using defaults: {e}", path.display());
                    let bad_path = path.with_extension("toml.bad");
                    let _ = std::fs::remove_file(&bad_path);
                    let _ = std::fs::rename(path, &bad_path);
                    let cfg = Config::default();
                    cfg.save_to(path)?;
                    Ok(cfg)
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let cfg = Config::default();
                cfg.save_to(path)?;
                Ok(cfg)
            }
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = path()?;
        self.save_to(&path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("creating {}", dir.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("orbit-core-{name}-{nanos}"))
    }

    #[test]
    fn round_trips_through_toml() {
        let cfg = Config {
            theme: ThemeMode::Light,
            placement: Placement::Right,
            on_top: false,
            hover_delay_ms: 200,
            collapse_grace_ms: 400,
            launch_at_startup: true,
            pause_in_fullscreen: false,
            show_privacy_indicators: false,
            notifications_enabled: false,
            privacy_mode: true,
            preview_privacy: true,
            blocked_apps: vec!["discord.exe".into(), "slack.exe".into()],
            shelf_storage_mode: "copy".into(),
            weather_enabled: false,
            weather_unit: WeatherUnit::Fahrenheit,
            expand_trigger: ExpandTrigger::Click,
            notification_preview_secs: 9,
            glass_mode: GlassMode::Blurred,
            hide_from_screen_capture: true,
            color_theme: ColorTheme::Cyberpunk,
            motion_preset: MotionPreset::Bouncy,
            time_format: TimeFormat::Hour24,
            compact_right_slot: CompactRightSlot::OrbitLogo,
            utility_alerts_enabled: false,
            settings_appearance: SettingsAppearance::Orbit,
        };
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        let back: Config = toml::from_str(&text).expect("parse");
        assert_eq!(cfg, back);
    }

    #[test]
    fn normalized_clamps_bounds_and_resets_bad_storage_mode() {
        let raw = Config {
            hover_delay_ms: 5000,
            collapse_grace_ms: 10,
            notification_preview_secs: 99,
            shelf_storage_mode: "unknown".into(),
            ..Config::default()
        }
        .normalized();
        assert_eq!(raw.hover_delay_ms, 1000);
        assert_eq!(raw.collapse_grace_ms, 50);
        assert_eq!(raw.notification_preview_secs, 15);
        assert_eq!(raw.shelf_storage_mode, "reference");
    }

    #[test]
    fn partial_gate1_config_loads_with_defaults() {
        let gate1_toml = r#"
theme = "auto"
placement = "center"
on_top = true
hover_delay_ms = 120
collapse_grace_ms = 250
launch_at_startup = false
"#;
        let cfg: Config = toml::from_str(gate1_toml).expect("parse gate1 config");
        assert!(cfg.pause_in_fullscreen);
        assert!(cfg.show_privacy_indicators);
        assert!(cfg.notifications_enabled);
        assert_eq!(cfg.shelf_storage_mode, "reference");
        assert!(cfg.weather_enabled);
        assert_eq!(cfg.weather_unit, WeatherUnit::Celsius);
        assert_eq!(cfg.expand_trigger, ExpandTrigger::Hover);
        assert_eq!(cfg.notification_preview_secs, 5);
        assert_eq!(cfg.glass_mode, GlassMode::Normal);
        assert!(!cfg.hide_from_screen_capture);
        assert_eq!(cfg.color_theme, ColorTheme::Obsidian);
        assert_eq!(cfg.motion_preset, MotionPreset::LiquidSpring);
        assert_eq!(cfg.time_format, TimeFormat::Hour12);
        assert_eq!(cfg.compact_right_slot, CompactRightSlot::Weather);
        assert!(cfg.utility_alerts_enabled);
        assert_eq!(cfg.settings_appearance, SettingsAppearance::Dark);
    }

    #[test]
    fn bad_config_is_renamed_and_defaults_written() {
        let dir = temp_dir("bad-cfg");
        let path = dir.join("config.toml");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "theme = [not valid toml").unwrap();

        let loaded = Config::load_from(&path).expect("fallback to defaults");
        assert_eq!(loaded, Config::default());
        assert!(dir.join("config.toml.bad").exists());
        assert!(path.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
