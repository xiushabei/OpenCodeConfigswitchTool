use std::fs;
use std::io;
use std::path::Path;

use crate::error::AppError;
use crate::model::{ExportEnvelope, EXPORTS_DIR, EXPORT_FORMAT_VERSION};
use crate::storage;

/// Writes the preset into `exports/` and returns its project-relative path.
///
/// # Errors
///
/// Returns [`AppError`] when the preset storage cannot be read, the preset is unknown, the JSON
/// envelope cannot be serialized, or no exclusive export file can be created.
pub fn export_preset(root: &Path, id: &str) -> Result<String, AppError> {
    let presets = storage::read_presets(root)?;
    let preset = presets
        .iter()
        .find(|preset| preset.id == id)
        .ok_or_else(|| AppError::PresetNotFound { id: id.to_string() })?;
    let envelope = ExportEnvelope {
        format_version: EXPORT_FORMAT_VERSION,
        preset: preset.clone(),
    };
    let mut json =
        serde_json::to_string_pretty(&envelope).map_err(|err| AppError::serialize(&err))?;
    json.push('\n');

    let dir = root.join(EXPORTS_DIR);
    fs::create_dir_all(&dir).map_err(|err| AppError::io("创建导出目录", &dir, err))?;
    let stem = sanitize_file_stem(&preset.id);
    let stamp = storage::timestamp_millis();

    for attempt in 0..100 {
        let name = if attempt == 0 {
            format!("{stem}-{stamp}.json")
        } else {
            format!("{stem}-{stamp}-{attempt}.json")
        };
        let path = dir.join(&name);
        match storage::try_create_file(&path, json.as_bytes()) {
            Ok(()) => return Ok(format!("{EXPORTS_DIR}/{name}")),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(AppError::io("写入导出文件", &path, err)),
        }
    }
    Err(AppError::io(
        "写入导出文件",
        &dir,
        io::Error::new(io::ErrorKind::AlreadyExists, "无法生成唯一的导出文件名"),
    ))
}

fn sanitize_file_stem(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.trim_matches('-').is_empty() {
        "preset".to_string()
    } else {
        cleaned
    }
}
