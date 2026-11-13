use std::path::Path;

use serde_json::Value;

use crate::error::AppError;
use crate::model::{Preset, Settings};
use crate::paths;

/// Directories and files owned by the application. Targets must never overwrite them.
const RESERVED_DIRS: [&str; 9] = [
    "data",
    "backups",
    "exports",
    "src-tauri",
    "src",
    "dist",
    "node_modules",
    ".git",
    "target",
];

const RESERVED_FILES: [&str; 8] = [
    "switcher.settings.json",
    "package.json",
    "index.html",
    "vite.config.ts",
    "tsconfig.json",
    "CONTRACT.md",
    "README.md",
    "Cargo.toml",
];

/// Validates that both configured targets resolve to distinct, writable application-external files.
///
/// # Errors
///
/// Returns [`AppError::TargetPathInvalid`] or [`AppError::PathCollision`] when a target path is
/// rejected or would collide with application-owned files.
pub fn validate_settings(root: &Path, settings: &Settings) -> Result<(), AppError> {
    let opencode = paths::resolve_target(root, &settings.targets.opencode)?;
    let omo = paths::resolve_target(root, &settings.targets.omo)?;

    if paths::same_path(&opencode, &omo) {
        return Err(AppError::PathCollision {
            reason: "两个目标不能指向同一个文件。".to_string(),
        });
    }

    for (label, path) in [("目标 opencode", &opencode), ("目标 omo", &omo)] {
        ensure_not_owned(root, path, label)?;
        if path.is_dir() {
            return Err(AppError::TargetPathInvalid {
                reason: format!("{label} 指向一个目录：{}", path.display()),
            });
        }
    }
    Ok(())
}

fn ensure_not_owned(root: &Path, target: &Path, label: &str) -> Result<(), AppError> {
    let root = paths::normalize(root);
    if paths::same_path(target, &root) {
        return Err(AppError::PathCollision {
            reason: format!("{label} 不能指向项目根目录。"),
        });
    }
    for dir in RESERVED_DIRS {
        if paths::is_within(target, &root.join(dir)) {
            return Err(AppError::PathCollision {
                reason: format!("{label} 不能指向应用保留目录 {dir}/。"),
            });
        }
    }
    for file in RESERVED_FILES {
        if paths::same_path(target, &root.join(file)) {
            return Err(AppError::PathCollision {
                reason: format!("{label} 不能覆盖应用文件 {file}。"),
            });
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if paths::same_path(target, &exe) {
            return Err(AppError::PathCollision {
                reason: format!("{label} 不能指向应用自身。"),
            });
        }
    }
    Ok(())
}

/// Validates a preset before it is stored or applied. Content must be a strict JSON object.
///
/// # Errors
///
/// Returns [`AppError::PresetInvalid`] when the name or content is empty or the content is not a
/// JSON object.
pub fn validate_preset_input(preset: &Preset) -> Result<(), AppError> {
    if preset.name.trim().is_empty() {
        return Err(AppError::PresetInvalid {
            reason: "名称不能为空。".to_string(),
        });
    }
    if preset.content.trim().is_empty() {
        return Err(AppError::PresetInvalid {
            reason: "内容不能为空。".to_string(),
        });
    }
    match serde_json::from_str::<Value>(&preset.content) {
        Ok(Value::Object(_)) => Ok(()),
        Ok(_) => Err(AppError::PresetInvalid {
            reason: "内容必须是 JSON 对象。".to_string(),
        }),
        Err(err) => Err(AppError::PresetInvalid {
            reason: format!("内容不是有效 JSON：{err}"),
        }),
    }
}
