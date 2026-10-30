pub mod adaptive;
pub mod client_restore;
pub mod policy;
pub mod settings;

pub use adaptive::RZeroAdaptiveSettings;
pub use client_restore::RZeroClientRestoreSettings;
pub use policy::RZeroCheckpointPolicy;
pub use settings::RZeroSettings;

use crate::{log_error, runtime::RZeroRuntime};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct RZeroConfig;

impl RZeroConfig {
    pub fn config_path() -> PathBuf {
        Path::new("config").join("rzero.json")
    }

    pub fn load() {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str::<RZeroSettings>(&text).ok())
            {
                Some(loaded) => {
                    RZeroRuntime::with_mut(|rt| rt.set_settings(loaded.sanitize()));
                }
                None => {
                    log_error("Failed to load config/rzero.json, falling back to defaults");
                    RZeroRuntime::with_mut(|rt| rt.set_settings(RZeroSettings::default()));
                }
            }
        } else {
            RZeroRuntime::with_mut(|rt| rt.set_settings(RZeroSettings::default()));
            Self::save();
        }
    }

    pub fn save() {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let settings = RZeroRuntime::with(|rt| rt.settings.clone());
        match serde_json::to_string_pretty(&settings) {
            Ok(json) => {
                if let Err(err) = fs::write(&path, json) {
                    log_error(&format!("Failed to write config/rzero.json: {err}"));
                }
            }
            Err(err) => {
                log_error(&format!("Failed to serialize RZeroSettings: {err}"));
            }
        }
    }
}