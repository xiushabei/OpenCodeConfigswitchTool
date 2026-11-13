use serde_json::Value;

const SERIALIZE_PLACEHOLDER: &str = "[无法序列化 JSON]";

/// Formats any JSON value in full, indented for reading. Nothing is removed or shortened.
pub fn format_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => pretty(other),
    }
}

/// Renders a tool part as its readable name, status, and complete input/output.
///
/// String payloads stay verbatim; structured payloads are pretty-printed JSON.
pub fn summarize_tool(value: &Value) -> String {
    let name = value
        .get("tool")
        .and_then(Value::as_str)
        .or_else(|| value.get("name").and_then(Value::as_str))
        .unwrap_or("tool");
    let state = value.get("state").unwrap_or(value);
    let status = state
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let mut text = format!("工具：{name}\n状态：{status}");
    if let Some(input) = state.get("input").or_else(|| value.get("input")) {
        text.push_str("\n输入：\n");
        text.push_str(&format_value(input));
    }
    if let Some(output) = state.get("output").or_else(|| value.get("output")) {
        text.push_str("\n输出：\n");
        text.push_str(&format_value(output));
    }
    text
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value)
        .or_else(|_| serde_json::to_string(value))
        .unwrap_or_else(|_| SERIALIZE_PLACEHOLDER.to_string())
}
