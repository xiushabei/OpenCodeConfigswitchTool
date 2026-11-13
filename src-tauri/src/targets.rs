use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::model::{Preset, Settings, Target, TargetState};
use crate::{jsonc, paths};

/// Reads the current state of one configured target file.
#[must_use]
pub fn read_target_state(root: &Path, settings: &Settings, target: Target) -> TargetState {
    let raw = settings.targets.get(target);
    let path = match paths::resolve_target(root, raw) {
        Ok(path) => path,
        Err(err) => return error_state(target, raw, err.to_string()),
    };
    let display = path.display().to_string();

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return TargetState {
                target,
                path: display,
                content: None,
                error: None,
                active_preset_ids: Vec::new(),
            }
        }
        Err(err) => return error_state(target, &display, format!("读取目标文件失败：{err}")),
    };

    match String::from_utf8(bytes) {
        Ok(content) => TargetState {
            target,
            path: display,
            content: Some(content),
            error: None,
            active_preset_ids: Vec::new(),
        },
        Err(_) => error_state(
            target,
            &display,
            "目标文件不是有效的 UTF-8 文本。".to_string(),
        ),
    }
}

fn error_state(target: Target, path: &str, error: String) -> TargetState {
    TargetState {
        target,
        path: path.to_string(),
        content: None,
        error: Some(error),
        active_preset_ids: Vec::new(),
    }
}

/// Marks presets whose content is structurally equivalent to the target file.
///
/// JSONC comments, trailing commas, CRLF, and object key order do not prevent a match. Invalid
/// target text never marks a preset active, even when the raw text is identical.
pub fn mark_active(states: &mut [TargetState], presets: &[Preset]) {
    for state in states.iter_mut() {
        let target = state.target;
        let active: Vec<String> = state.content.as_deref().map_or_else(Vec::new, |content| {
            presets
                .iter()
                .filter(|preset| preset.target == target)
                .filter(|preset| jsonc::objects_equal(&preset.content, content))
                .map(|preset| preset.id.clone())
                .collect()
        });
        state.active_preset_ids = active;
    }
}
