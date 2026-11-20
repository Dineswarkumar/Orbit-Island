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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: ThemeMode,
    pub placement: Placement,
    pub on_top: bool,
    pub hover_delay_ms: u32,
    pub collapse_grace_ms: u32,
    pub launch_at_startup: bool,
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
    /// Loads the config from the default path, writing defaults on first run.
    /// A malformed file is renamed to `config.toml.bad` and defaults are used.
    pub fn load() -> Result<Self> {
        let path = path()?;
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => match toml::from_str::<Config>(&text) {
                Ok(cfg) => Ok(cfg),
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
        let cfg = Config::default();
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        let back: Config = toml::from_str(&text).expect("parse");
        assert_eq!(cfg, back);
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
