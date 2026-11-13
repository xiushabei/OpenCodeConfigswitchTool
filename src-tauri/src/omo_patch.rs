use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::AppError;

const MODEL_FIELD: &str = "model";
const BLOCKED_KEYS: [&str; 3] = ["__proto__", "prototype", "constructor"];

/// The only OMO sections that hold documented model assignments.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Section {
    Agents,
    Categories,
}

impl Section {
    const fn key(self) -> &'static str {
        match self {
            Self::Agents => "agents",
            Self::Categories => "categories",
        }
    }
}

/// Edits exactly one `section.key.model` assignment in OMO JSON text.
///
/// `Some(model)` sets the override, `None` removes only the `model` field. Unknown fields, key
/// order, strings, and numeric values (including integers beyond 2^53) survive because the whole
/// document is parsed once and re-serialized. Nothing is written to disk.
///
/// # Errors
///
/// Returns [`AppError::OmoPatchInvalid`] when the content is not a JSON object, the section or
/// entry has the wrong type, or the key/model is empty or prototype-sensitive.
pub fn patch(
    content: &str,
    section: Section,
    key: &str,
    model: Option<&str>,
) -> Result<String, AppError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(invalid("条目键不能为空。"));
    }
    if BLOCKED_KEYS
        .iter()
        .any(|blocked| key.eq_ignore_ascii_case(blocked))
    {
        return Err(invalid(format!("条目键不能使用原型敏感名称：{key}")));
    }
    if key.contains('\0') {
        return Err(invalid("条目键不能包含 NUL 字符。"));
    }
    let model = model.map(str::trim);
    if model.is_some_and(str::is_empty) {
        return Err(invalid("模型 ID 不能为空；清除请使用 null。"));
    }

    let mut root: Value = serde_json::from_str(content)
        .map_err(|err| invalid(format!("内容不是有效 JSON：{err}")))?;
    let Value::Object(root_map) = &mut root else {
        return Err(invalid("配置根必须是 JSON 对象。"));
    };
    let section_name = section.key();
    let section_value = root_map
        .entry(section_name)
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(entries) = section_value else {
        return Err(invalid(format!("{section_name} 必须是 JSON 对象。")));
    };
    let entry = entries
        .entry(key)
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(entry_map) = entry else {
        return Err(invalid(format!("{section_name}.{key} 必须是 JSON 对象。")));
    };
    match model {
        Some(model) => {
            entry_map.insert(MODEL_FIELD.to_string(), Value::String(model.to_string()));
        }
        None => {
            entry_map.remove(MODEL_FIELD);
        }
    }
    serde_json::to_string_pretty(&root).map_err(|err| AppError::serialize(&err))
}

fn invalid(reason: impl Into<String>) -> AppError {
    AppError::OmoPatchInvalid {
        reason: reason.into(),
    }
}
