use serde_json::Value;

use super::json_text::{format_value, summarize_tool};
use super::types::PartView;

/// Parsed display data for one message row. `truncated` marks malformed input only.
pub struct MessageContent {
    pub role: String,
    pub parts: Vec<PartView>,
    pub truncated: bool,
}

/// v1 message rows only carry the role; parts come from the `part` table.
pub fn parse_v1_message(data: &str) -> MessageContent {
    match serde_json::from_str::<Value>(data) {
        Ok(value) => MessageContent {
            role: value
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            parts: Vec::new(),
            truncated: false,
        },
        Err(_) => MessageContent {
            role: "unknown".to_string(),
            parts: Vec::new(),
            truncated: true,
        },
    }
}

/// v2 rows may carry inline `content`/`parts`/`text`; `session_part` rows are merged by the caller.
pub fn parse_v2_message(data: &str, type_column: &str) -> MessageContent {
    let Ok(value) = serde_json::from_str::<Value>(data) else {
        return MessageContent {
            role: fallback_role(type_column),
            parts: vec![malformed_part(None, data)],
            truncated: true,
        };
    };
    let role = value
        .get("type")
        .and_then(Value::as_str)
        .or_else(|| value.get("role").and_then(Value::as_str))
        .map_or_else(|| fallback_role(type_column), str::to_string);
    let mut parts = Vec::new();
    if let Some(items) = inline_items(&value) {
        for item in ordered_items(items) {
            parts.push(part_from_value(item, None));
        }
    } else if let Some(text) = value.get("text").and_then(Value::as_str) {
        parts.push(text_part(text));
    }
    MessageContent {
        role,
        parts,
        truncated: false,
    }
}

/// Builds one display part from a parsed JSON part row, keeping its full content.
pub fn part_from_value(value: &Value, kind_hint: Option<&str>) -> PartView {
    let kind = value
        .get("type")
        .and_then(Value::as_str)
        .or(kind_hint)
        .unwrap_or("unknown")
        .to_string();
    let text = match kind.as_str() {
        "text" | "reasoning" => value
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        "tool" => summarize_tool(value),
        _ => match value.get("text").and_then(Value::as_str) {
            Some(text) => text.to_string(),
            None => format_value(value),
        },
    };
    PartView {
        kind,
        text,
        truncated: false,
    }
}

/// Builds a part from raw JSON data. Malformed data stays visible as raw, flagged text.
pub fn part_from_data(data: &str, kind_hint: Option<&str>) -> PartView {
    match serde_json::from_str::<Value>(data) {
        Ok(value) => part_from_value(&value, kind_hint),
        Err(_) => malformed_part(kind_hint, data),
    }
}

pub fn text_part(text: &str) -> PartView {
    PartView {
        kind: "text".to_string(),
        text: text.to_string(),
        truncated: false,
    }
}

fn malformed_part(kind_hint: Option<&str>, raw: &str) -> PartView {
    let text = if raw.is_empty() {
        "[消息片段无法解析：原始内容为空]".to_string()
    } else {
        format!("[消息片段不是有效 JSON，以下为原始内容]\n{raw}")
    };
    PartView {
        kind: kind_hint.unwrap_or("text").to_string(),
        text,
        truncated: true,
    }
}

fn fallback_role(type_column: &str) -> String {
    if type_column.is_empty() {
        "unknown".to_string()
    } else {
        type_column.to_string()
    }
}

fn inline_items(value: &Value) -> Option<&Vec<Value>> {
    value
        .get("content")
        .and_then(Value::as_array)
        .or_else(|| value.get("parts").and_then(Value::as_array))
}

fn ordered_items(items: &[Value]) -> Vec<&Value> {
    let mut entries: Vec<(i64, usize, &Value)> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let ordinal = item
                .get("ordinal")
                .and_then(Value::as_i64)
                .unwrap_or(i64::try_from(index).unwrap_or(i64::MAX));
            (ordinal, index, item)
        })
        .collect();
    entries.sort_by_key(|(ordinal, index, _)| (*ordinal, *index));
    entries.into_iter().map(|(_, _, item)| item).collect()
}
