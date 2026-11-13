use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

/// Errors surfaced to the UI. Messages are user-facing and complete sentences.
#[derive(Debug)]
pub enum AppError {
    RootNotFound,
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    SettingsInvalid {
        reason: String,
    },
    PresetStorageInvalid {
        reason: String,
    },
    PresetNotFound {
        id: String,
    },
    PresetInvalid {
        reason: String,
    },
    TargetPathInvalid {
        reason: String,
    },
    PathCollision {
        reason: String,
    },
    LockPoisoned,
    Serialize {
        reason: String,
    },
    ImportInvalid {
        reason: String,
    },
    StartupBaselineMissing,
    StartupRestoreBlocked {
        reason: String,
    },
    RestorePartial {
        reason: String,
    },
    RestoreRefreshFailed {
        reason: String,
    },
}

impl AppError {
    pub fn io(action: &'static str, path: impl AsRef<Path>, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.as_ref().to_path_buf(),
            source,
        }
    }

    /// Wraps a JSON serialization failure.
    #[must_use]
    pub fn serialize(error: &serde_json::Error) -> Self {
        Self::Serialize {
            reason: error.to_string(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootNotFound => write!(
                f,
                "未找到 switcher.settings.json：已从可执行文件与当前目录向上查找，请确认项目结构完整。"
            ),
            Self::Io {
                action,
                path,
                source,
            } => write!(f, "{action}失败（{}）：{source}", path.display()),
            Self::SettingsInvalid { reason } => write!(f, "设置无效：{reason}"),
            Self::PresetStorageInvalid { reason } => {
                write!(f, "预设存储无效（data/presets.json）：{reason}")
            }
            Self::PresetNotFound { id } => write!(f, "预设不存在：{id}"),
            Self::PresetInvalid { reason } => write!(f, "预设无效：{reason}"),
            Self::TargetPathInvalid { reason } => write!(f, "目标路径无效：{reason}"),
            Self::PathCollision { reason } => write!(f, "路径冲突：{reason}"),
            Self::LockPoisoned => write!(f, "内部状态锁已损坏，请重启应用。"),
            Self::Serialize { reason } => write!(f, "序列化失败：{reason}"),
            Self::ImportInvalid { reason } => write!(f, "导入文件无效：{reason}"),
            Self::StartupBaselineMissing => {
                write!(f, "本次启动没有可用的启动状态基线，无法恢复。")
            }
            Self::StartupRestoreBlocked { reason } => write!(f, "无法恢复启动状态：{reason}"),
            Self::RestorePartial { reason } => write!(f, "恢复启动状态未完全成功：{reason}"),
            Self::RestoreRefreshFailed { reason } => {
                write!(f, "目标文件已恢复，但刷新数据失败：{reason}")
            }
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::RootNotFound
            | Self::SettingsInvalid { .. }
            | Self::PresetStorageInvalid { .. }
            | Self::PresetNotFound { .. }
            | Self::PresetInvalid { .. }
            | Self::TargetPathInvalid { .. }
            | Self::PathCollision { .. }
            | Self::LockPoisoned
            | Self::Serialize { .. }
            | Self::ImportInvalid { .. }
            | Self::StartupBaselineMissing
            | Self::StartupRestoreBlocked { .. }
            | Self::RestorePartial { .. }
            | Self::RestoreRefreshFailed { .. } => None,
        }
    }
}
