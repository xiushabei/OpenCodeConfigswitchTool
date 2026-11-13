use std::collections::HashSet;

use crate::error::AppError;

use super::ModelEntry;

const MAX_ID_LEN: usize = 200;
const MAX_DIAGNOSTIC_CHARS: usize = 400;
const REDACTED: &str = "<redacted>";
const SECRET_PREFIXES: [&str; 9] = [
    "sk-",
    "pk-",
    "ghp_",
    "gho_",
    "ghs_",
    "github_pat_",
    "xoxb-",
    "xoxp-",
    "aiza",
];

/// Parses non-verbose `opencode models` stdout: one `provider/model` per line.
///
/// Noisy lines are rejected instead of being turned into models. Duplicate IDs keep their first
/// occurrence and order. An empty output is a valid empty catalogue.
///
/// # Errors
///
/// Returns [`AppError::ModelOutputInvalid`] when stdout is not UTF-8 or every non-empty line is
/// unrecognizable.
pub fn parse_output(stdout: &[u8]) -> Result<Vec<ModelEntry>, AppError> {
    let text =
        std::str::from_utf8(stdout).map_err(|_| invalid("命令输出不是有效 UTF-8。".to_string()))?;
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    let mut rejected = 0_usize;
    for raw_line in text.split('\n') {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if let Some(entry) = parse_line(line) {
            if seen.insert(entry.id.clone()) {
                entries.push(entry);
            }
        } else {
            rejected += 1;
        }
    }
    if entries.is_empty() && rejected > 0 {
        return Err(invalid(format!(
            "{rejected} 行输出无法识别为 provider/model。"
        )));
    }
    Ok(entries)
}

/// Removes control characters, redacts likely secrets, and truncates command stderr.
#[must_use]
pub fn sanitize_diagnostic(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let flattened: String = text
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    let mut words = Vec::new();
    let mut redact_next = false;
    for word in flattened.split_whitespace() {
        if redact_next {
            words.push(REDACTED.to_string());
            redact_next = false;
        } else if word.eq_ignore_ascii_case("bearer") {
            words.push(word.to_string());
            redact_next = true;
        } else {
            words.push(redact_word(word));
        }
    }
    truncate(&words.join(" "))
}

fn parse_line(line: &str) -> Option<ModelEntry> {
    if line.len() > MAX_ID_LEN || line.trim() != line || line.contains("://") {
        return None;
    }
    if !line.chars().all(is_model_char) {
        return None;
    }
    let (provider, model) = line.split_once('/')?;
    if provider.is_empty()
        || model.is_empty()
        || provider == "."
        || provider == ".."
        || model == "."
        || model == ".."
    {
        return None;
    }
    Some(ModelEntry {
        id: line.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
    })
}

const fn is_model_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '@' | '+' | '~')
}

fn redact_word(word: &str) -> String {
    if let Some((name, value)) = word.split_once('=') {
        if !value.is_empty() && is_sensitive_name(name) {
            return format!("{name}={REDACTED}");
        }
    }
    let token = word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_');
    let lower = token.to_ascii_lowercase();
    if SECRET_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix) && token.len() > prefix.len() + 6)
        || (token.len() >= 32
            && token
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
    {
        return REDACTED.to_string();
    }
    word.to_string()
}

fn is_sensitive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "passwd",
        "apikey",
        "api_key",
        "authorization",
        "cookie",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_DIAGNOSTIC_CHARS {
        return text.to_string();
    }
    let mut shortened: String = text.chars().take(MAX_DIAGNOSTIC_CHARS).collect();
    shortened.push_str("...");
    shortened
}

fn invalid(reason: String) -> AppError {
    AppError::ModelOutputInvalid { reason }
}
