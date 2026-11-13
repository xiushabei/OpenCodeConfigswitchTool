use serde::{Deserialize, Serialize};

pub const SETTINGS_FILE: &str = "switcher.settings.json";
pub const DATA_DIR: &str = "data";
pub const PRESETS_FILE: &str = "data/presets.json";
pub const WEBVIEW_DIR: &str = "data/webview";
pub const BACKUPS_DIR: &str = "backups";
pub const EXPORTS_DIR: &str = "exports";
pub const PRESET_FORMAT_VERSION: u32 = 1;
pub const EXPORT_FORMAT_VERSION: u32 = 1;
pub const RESTART_MESSAGE: &str = "已应用到目标文件。请重启 OpenCode 使配置生效。";
pub const RESTORE_MESSAGE: &str = "已恢复启动时的目标文件。请重启 OpenCode 使配置生效。";

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Opencode,
    Omo,
}

impl Target {
    pub const ALL: [Self; 2] = [Self::Opencode, Self::Omo];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Opencode => "opencode",
            Self::Omo => "omo",
        }
    }

    /// User-facing label used in startup warnings and restore messages.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Opencode => "OpenCode",
            Self::Omo => "OMO",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct TargetPaths {
    pub opencode: String,
    pub omo: String,
}

impl TargetPaths {
    #[must_use]
    pub fn get(&self, target: Target) -> &str {
        match target {
            Target::Opencode => &self.opencode,
            Target::Omo => &self.omo,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub targets: TargetPaths,
    pub theme: Theme,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub target: Target,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TargetState {
    pub target: Target,
    pub path: String,
    pub content: Option<String>,
    pub error: Option<String>,
    pub active_preset_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub settings: Settings,
    pub presets: Vec<Preset>,
    pub targets: Vec<TargetState>,
    pub root: String,
    /// Startup import or baseline warnings. Empty when startup found nothing to report.
    #[serde(default)]
    pub startup_warnings: Vec<String>,
    /// True only when a complete launch baseline was captured and can be restored.
    #[serde(default)]
    pub can_restore_startup: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ApplyResult {
    pub snapshot: Snapshot,
    pub backup: Option<String>,
    pub message: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PresetsFile {
    pub format_version: u32,
    pub presets: Vec<Preset>,
    /// `None` for storage written before the first-import marker existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_import_completed: Option<bool>,
}

impl PresetsFile {
    /// Files written by older versions have no marker; they already hold user data and must
    /// never trigger another automatic import.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.first_import_completed.unwrap_or(true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportEnvelope {
    pub format_version: u32,
    pub preset: Preset,
}
