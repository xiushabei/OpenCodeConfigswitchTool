use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::jsonc;

/// Reads a dropped `.json`/`.jsonc` file as UTF-8 and returns strict JSON object text.
///
/// Strict JSON objects are returned byte-for-byte. JSONC input is normalized to strict JSON so
/// the preset storage contract stays unchanged.
///
/// # Errors
///
/// Returns [`AppError::ImportInvalid`] for unsupported extensions, non-UTF-8 content, or content
/// that is not a JSON/JSONC object, and [`AppError::Io`] when the file cannot be read.
pub fn read_import_text(path: &str) -> Result<String, AppError> {
    let path = Path::new(path);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(extension.as_deref(), Some("json" | "jsonc")) {
        return Err(AppError::ImportInvalid {
            reason: "只支持 .json 或 .jsonc 文件。".to_string(),
        });
    }
    if !path.is_file() {
        return Err(AppError::ImportInvalid {
            reason: format!("导入路径不是文件：{}", path.display()),
        });
    }

    let bytes = fs::read(path).map_err(|err| AppError::io("读取导入文件", path, err))?;
    let text = String::from_utf8(bytes).map_err(|_| AppError::ImportInvalid {
        reason: "导入文件不是有效的 UTF-8 文本。".to_string(),
    })?;
    normalize_content(&text)
}

/// Normalizes JSON/JSONC object text to strict JSON, returning strict input byte-for-byte.
///
/// # Errors
///
/// Returns [`AppError::ImportInvalid`] when the content is not a JSON/JSONC object.
pub fn normalize_content(text: &str) -> Result<String, AppError> {
    if jsonc::is_strict_object(text) {
        return Ok(text.to_string());
    }

    let value = jsonc::parse_object(text).map_err(|reason| AppError::ImportInvalid { reason })?;
    jsonc::normalize_object(&value).map_err(|reason| AppError::ImportInvalid { reason })
}
