use super::{RZeroAdaptiveSettings, RZeroCheckpointPolicy, RZeroClientRestoreSettings};
use crate::anchor::RZeroAnchorSettings;
use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RZeroSettings {
    #[serde(rename = "rzerochashEnabled", default = "default_true")]
    pub rzerochash_enabled: bool,
    #[serde(rename = "checkpointPolicy", default)]
    pub checkpoint_policy: RZeroCheckpointPolicy,
    #[serde(default)]
    pub client_restore: RZeroClientRestoreSettings,
    #[serde(default)]
    pub anchor: RZeroAnchorSettings,
    #[serde(default)]
    pub adaptive: RZeroAdaptiveSettings,
    #[serde(rename = "advancedUi", default)]
    pub advanced_ui: bool,
    #[serde(default)]
    pub logs: bool,
}

impl Default for RZeroSettings {
    fn default() -> Self {
        Self {
            rzerochash_enabled: true,
            checkpoint_policy: RZeroCheckpointPolicy::default(),
            client_restore: RZeroClientRestoreSettings::default(),
            anchor: RZeroAnchorSettings::default(),
            adaptive: RZeroAdaptiveSettings::default(),
            advanced_ui: false,
            logs: false,
        }
    }
}

impl RZeroSettings {
    pub fn sanitize(self) -> Self {
        Self {
            rzerochash_enabled: self.rzerochash_enabled,
            checkpoint_policy: self.checkpoint_policy.sanitize(),
            client_restore: self.client_restore.sanitize(),
            anchor: self.anchor.sanitize(),
            adaptive: self.adaptive.sanitize(),
            advanced_ui: self.advanced_ui,
            logs: self.logs,
        }
    }

    pub fn with_rzerochash_enabled(mut self, enabled: bool) -> Self {
        self.rzerochash_enabled = enabled;
        self
    }

    pub fn with_checkpoint_policy(mut self, policy: RZeroCheckpointPolicy) -> Self {
        self.checkpoint_policy = policy.sanitize();
        self
    }

    pub fn with_client_restore(mut self, client_restore: RZeroClientRestoreSettings) -> Self {
        self.client_restore = client_restore.sanitize();
        self
    }

    pub fn with_anchor(mut self, anchor: RZeroAnchorSettings) -> Self {
        self.anchor = anchor.sanitize();
        self
    }

    pub fn with_adaptive(mut self, adaptive: RZeroAdaptiveSettings) -> Self {
        self.adaptive = adaptive.sanitize();
        self
    }

    pub fn with_advanced_ui(mut self, advanced_ui: bool) -> Self {
        self.advanced_ui = advanced_ui;
        self
    }

    pub fn with_logs(mut self, logs: bool) -> Self {
        self.logs = logs;
        self
    }
}