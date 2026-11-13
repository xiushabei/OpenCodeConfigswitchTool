use serde_json::Value;

/// Parses JSONC (comments, trailing commas, optional BOM) and requires a top-level object.
///
/// # Errors
///
/// Returns a user-facing reason when the text is not valid JSONC or is not a JSON object.
pub fn parse_object(text: &str) -> Result<Value, String> {
    let without_comments = strip_comments(text)?;
    let cleaned = strip_trailing_commas(&without_comments);
    match serde_json::from_str::<Value>(&cleaned) {
        Ok(Value::Object(map)) => Ok(Value::Object(map)),
        Ok(_) => Err("顶层内容必须是 JSON 对象。".to_string()),
        Err(err) => Err(format!("不是有效的 JSON/JSONC：{err}")),
    }
}

/// Reports whether `text` is already a strict JSON object that `serde_json` accepts.
#[must_use]
pub fn is_strict_object(text: &str) -> bool {
    matches!(serde_json::from_str::<Value>(text), Ok(Value::Object(_)))
}

/// Serializes a value as pretty strict JSON with a trailing newline.
///
/// # Errors
///
/// Returns the serializer message when the value cannot be serialized.
pub fn normalize_object(value: &Value) -> Result<String, String> {
    let mut json = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    json.push('\n');
    Ok(json)
}

fn strip_comments(text: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }
        if ch != '/' {
            out.push(ch);
            continue;
        }
        match chars.peek() {
            Some('/') => {
                chars.next();
                out.push(' ');
                for next in chars.by_ref() {
                    if next == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            Some('*') => {
                chars.next();
                out.push(' ');
                let mut previous = '\0';
                let mut closed = false;
                for next in chars.by_ref() {
                    if previous == '*' && next == '/' {
                        closed = true;
                        break;
                    }
                    if next == '\n' {
                        out.push('\n');
                    }
                    previous = next;
                }
                if !closed {
                    return Err("块注释未闭合。".to_string());
                }
            }
            _ => out.push(ch),
        }
    }
    Ok(out)
}

fn strip_trailing_commas(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in chars.iter().enumerate() {
        if in_string {
            out.push(*ch);
            if escaped {
                escaped = false;
            } else if *ch == '\\' {
                escaped = true;
            } else if *ch == '"' {
                in_string = false;
            }
            continue;
        }
        if *ch == '"' {
            in_string = true;
            out.push(*ch);
            continue;
        }
        if *ch == ',' {
            let next = chars[index + 1..]
                .iter()
                .find(|candidate| !candidate.is_whitespace());
            if matches!(next, Some('}' | ']')) {
                continue;
            }
        }
        out.push(*ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{is_strict_object, normalize_object, parse_object};

    #[test]
    fn comments_and_trailing_commas_parse_as_object() {
        let text =
            "\u{feff}{\n  // line comment\n  \"a\": 1, /* block */\n  \"url\": \"http://x//y\",\n}";
        let value = parse_object(text).expect("JSONC object");
        assert_eq!(value["a"], 1);
        assert_eq!(value["url"], "http://x//y");
        let normalized = normalize_object(&value).expect("serialize");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&normalized).expect("strict json"),
            value
        );
    }

    #[test]
    fn non_object_and_broken_text_are_rejected() {
        assert!(parse_object("[]").is_err());
        assert!(parse_object("{").is_err());
        assert!(!is_strict_object("{\"a\":1,}"));
        assert!(is_strict_object("{\"a\":1}"));
    }

    #[test]
    fn comments_between_tokens_do_not_join_them() {
        assert!(parse_object("{\"a\": 1/*x*/2}").is_err());
        assert!(parse_object("{\"a\": tru/*x*/e}").is_err());
        assert!(parse_object("{\"a\": 1//x\n2}").is_err());
    }

    #[test]
    fn unterminated_block_comment_is_rejected_even_after_closed_object() {
        assert!(parse_object("{\"a\":1} /* unterminated").is_err());
        assert!(parse_object("/*").is_err());
        assert!(parse_object("{\"a\":1} /*").is_err());
    }

    #[test]
    fn comments_between_key_colon_and_value_are_accepted() {
        let value = parse_object("{\n  /* key */ \"a\" // colon\n  : /* value */ 1,\n}")
            .expect("valid JSONC");
        assert_eq!(value["a"], 1);
    }
}
