use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::model::{ApplyResult, RESTART_MESSAGE};
use crate::{backup, paths, service, storage, validate};

/// Applies a stored preset after validation. Never modifies anything when validation fails.
///
/// # Errors
///
/// Returns [`AppError`] when settings or preset storage cannot be read, the preset is unknown or
/// invalid, the target path is rejected, or the backup or target write fails.
pub fn apply_preset(root: &Path, id: &str) -> Result<ApplyResult, AppError> {
    let settings = storage::read_settings(root)?;
    let presets = storage::read_presets(root)?;
    let preset = presets
        .iter()
        .find(|preset| preset.id == id)
        .ok_or_else(|| AppError::PresetNotFound { id: id.to_string() })?;

    validate::validate_preset_input(preset)?;
    let target_path = paths::resolve_target(root, settings.targets.get(preset.target))?;
    if target_path.is_dir() {
        return Err(AppError::TargetPathInvalid {
            reason: format!("目标路径是目录：{}", target_path.display()),
        });
    }
    let parent = target_path
        .parent()
        .ok_or_else(|| AppError::TargetPathInvalid {
            reason: format!("目标路径没有父目录：{}", target_path.display()),
        })?;
    if !parent.is_dir() {
        fs::create_dir_all(parent).map_err(|err| AppError::io("创建目标目录", parent, err))?;
    }

    let backup_path = if target_path.is_file() {
        let original = fs::read(&target_path)
            .map_err(|err| AppError::io("读取目标文件", &target_path, err))?;
        Some(backup::write_backup(root, preset.target, &original)?)
    } else {
        None
    };

    storage::atomic_write(&target_path, preset.content.as_bytes())?;

    Ok(ApplyResult {
        snapshot: service::snapshot(root)?,
        backup: backup_path,
        message: RESTART_MESSAGE.to_string(),
    })
}
