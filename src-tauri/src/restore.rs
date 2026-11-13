use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::error::AppError;
use crate::model::{ApplyResult, Target, RESTORE_MESSAGE};
use crate::service;
use crate::startup::{BaselineFile, BaselineState, StartupBaseline};
use crate::{backup, storage};

struct RestorePlan<'a> {
    file: &'a BaselineFile,
    current: Option<Vec<u8>>,
    backup: Option<String>,
    action: Action,
}

#[derive(Clone, Copy)]
enum Action {
    Restored,
    Removed,
    Unchanged,
}

impl Action {
    fn verb(self) -> &'static str {
        match self {
            Self::Restored => "已恢复",
            Self::Removed => "已删除",
            Self::Unchanged => "未变化",
        }
    }
}

/// Restores both launch targets exactly, backing up any current bytes before writing or removing.
///
/// # Errors
///
/// Returns [`AppError::StartupRestoreBlocked`] when the baseline is incomplete or unreadable,
/// [`AppError::Io`] when preflight reads fail, [`AppError::RestorePartial`] when a target fails
/// after another one changed, and [`AppError::RestoreRefreshFailed`] when the snapshot cannot be
/// rebuilt after a successful restore.
pub(crate) fn restore_startup(
    root: &Path,
    baseline: &StartupBaseline,
) -> Result<ApplyResult, AppError> {
    let mut plans = preflight(baseline)?;
    for plan in &mut plans {
        prepare(plan, root)?;
    }
    apply_all(&mut plans)?;

    let backups: Vec<String> = plans
        .iter()
        .filter_map(|plan| plan.backup.clone())
        .collect();
    let snapshot = service::snapshot(root).map_err(|err| AppError::RestoreRefreshFailed {
        reason: err.to_string(),
    })?;
    Ok(ApplyResult {
        snapshot,
        backup: backups.first().cloned(),
        message: restore_message(&plans, &backups),
    })
}

fn preflight(baseline: &StartupBaseline) -> Result<Vec<RestorePlan<'_>>, AppError> {
    if baseline.files().len() != Target::ALL.len() {
        return Err(AppError::StartupRestoreBlocked {
            reason: "启动基线没有同时包含两个目标。".to_string(),
        });
    }

    let mut plans = Vec::with_capacity(baseline.files().len());
    for file in baseline.files() {
        if let BaselineState::Unreadable(reason) = &file.state {
            return Err(AppError::StartupRestoreBlocked {
                reason: format!("{} 目标在启动时读取失败：{reason}", file.target.label()),
            });
        }
        if file.path.is_dir() {
            return Err(AppError::TargetPathInvalid {
                reason: format!("目标路径是目录：{}", file.path.display()),
            });
        }
        let current = match fs::read(&file.path) {
            Ok(bytes) => Some(bytes),
            Err(err) if err.kind() == ErrorKind::NotFound => None,
            Err(err) => return Err(AppError::io("读取当前目标文件", &file.path, err)),
        };
        plans.push(RestorePlan {
            file,
            current,
            backup: None,
            action: Action::Unchanged,
        });
    }
    Ok(plans)
}

fn prepare(plan: &mut RestorePlan<'_>, root: &Path) -> Result<(), AppError> {
    if matches!(&plan.file.state, BaselineState::Bytes(_)) {
        let parent = plan
            .file
            .path
            .parent()
            .ok_or_else(|| AppError::TargetPathInvalid {
                reason: format!("目标路径没有父目录：{}", plan.file.path.display()),
            })?;
        if !parent.is_dir() {
            fs::create_dir_all(parent).map_err(|err| AppError::io("创建目标目录", parent, err))?;
        }
    }
    if let Some(current) = &plan.current {
        plan.backup = Some(backup::write_backup(root, plan.file.target, current)?);
    }
    Ok(())
}

fn apply_all(plans: &mut [RestorePlan<'_>]) -> Result<(), AppError> {
    for index in 0..plans.len() {
        let outcome = apply_one(&plans[index]);
        match outcome {
            Ok(action) => plans[index].action = action,
            Err(err) => return Err(partial_error(plans, index, &err)),
        }
    }
    Ok(())
}

fn apply_one(plan: &RestorePlan<'_>) -> Result<Action, AppError> {
    match &plan.file.state {
        BaselineState::Bytes(bytes) => {
            storage::atomic_write(&plan.file.path, bytes)?;
            Ok(Action::Restored)
        }
        BaselineState::Missing => {
            if plan.current.is_none() {
                return Ok(Action::Unchanged);
            }
            match fs::remove_file(&plan.file.path) {
                Ok(()) => Ok(Action::Removed),
                Err(err) if err.kind() == ErrorKind::NotFound => Ok(Action::Removed),
                Err(err) => Err(AppError::io("删除目标文件", &plan.file.path, err)),
            }
        }
        BaselineState::Unreadable(_) => Err(AppError::StartupRestoreBlocked {
            reason: format!("{} 目标基线不可用。", plan.file.target.label()),
        }),
    }
}

fn partial_error(plans: &[RestorePlan<'_>], failed: usize, error: &AppError) -> AppError {
    let changed: Vec<String> = plans[..failed]
        .iter()
        .map(|plan| format!("{} {}", plan.file.target.label(), plan.action.verb()))
        .collect();
    let unprocessed: Vec<String> = plans[failed + 1..]
        .iter()
        .map(|plan| plan.file.target.label().to_string())
        .collect();
    let backups: Vec<String> = plans
        .iter()
        .filter_map(|plan| plan.backup.clone())
        .collect();
    AppError::RestorePartial {
        reason: format!(
            "已变更：{}；失败：{}（{error}）；未处理：{}；备份：{}。",
            join_or_none(&changed),
            plans[failed].file.target.label(),
            join_or_none(&unprocessed),
            join_or_none(&backups)
        ),
    }
}

fn join_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "无".to_string()
    } else {
        items.join("、")
    }
}

fn restore_message(plans: &[RestorePlan<'_>], backups: &[String]) -> String {
    let mut message = RESTORE_MESSAGE.to_string();
    let summary: Vec<String> = plans
        .iter()
        .map(|plan| format!("{} {}", plan.file.target.label(), plan.action.verb()))
        .collect();
    message.push('（');
    message.push_str(&summary.join("，"));
    message.push('）');
    if !backups.is_empty() {
        message.push_str(" 当前内容已备份至：");
        message.push_str(&backups.join("、"));
        message.push('。');
    }
    message
}

#[cfg(test)]
mod tests;
