use serde_json::{Number, Value};

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

/// Compares two JSON/JSONC object texts structurally.
///
/// Invalid JSON or non-object text never compares equal, even when the raw bytes match. Valid
/// objects ignore comments, a BOM, trailing commas, CRLF line endings, and object key order.
/// Array order, value types, and numeric values remain significant. Numbers compare by exact
/// decimal value without an `f64` round-trip, so `1` equals `1.0` and integers beyond `f64`
/// precision stay distinct.
#[must_use]
pub fn objects_equal(left: &str, right: &str) -> bool {
    match (parse_object(left), parse_object(right)) {
        (Ok(left), Ok(right)) => values_equal(&left, &right),
        _ => false,
    }
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| values_equal(value, other))
                })
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| values_equal(left, right))
        }
        (Value::Number(left), Value::Number(right)) => numbers_equal(left, right),
        _ => left == right,
    }
}

fn numbers_equal(left: &Number, right: &Number) -> bool {
    if left == right {
        return true;
    }
    match (
        canonical_decimal(&left.to_string()),
        canonical_decimal(&right.to_string()),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

#[derive(PartialEq, Eq)]
struct CanonicalNumber {
    negative: bool,
    digits: String,
    exponent: i64,
}

/// Normalizes one JSON number token to `sign * digits * 10^exponent`.
///
/// Returns `None` when the exponent does not fit `i64`; such numbers still compare equal only
/// when their raw tokens are identical.
fn canonical_decimal(raw: &str) -> Option<CanonicalNumber> {
    let (negative, body) = match raw.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, raw),
    };
    let (mantissa, exponent) = match body.find(['e', 'E']) {
        Some(index) => (
            body.get(..index)?,
            body.get(index + 1..)?.parse::<i64>().ok()?,
        ),
        None => (body, 0),
    };
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, fraction),
        None => (mantissa, ""),
    };
    let combined = format!("{integer}{fraction}");
    let Some(first_nonzero) = combined.find(|ch| ch != '0') else {
        return Some(CanonicalNumber {
            negative: false,
            digits: "0".to_string(),
            exponent: 0,
        });
    };
    let mut digits = combined.get(first_nonzero..)?.to_string();
    let fraction_len = i64::try_from(fraction.len()).ok()?;
    let mut exponent = exponent.checked_sub(fraction_len)?;
    while digits.ends_with('0') {
        digits.pop();
        exponent = exponent.checked_add(1)?;
    }
    Some(CanonicalNumber {
        negative,
        digits,
        exponent,
    })
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
    use super::{is_strict_object, normalize_object, objects_equal, parse_object};

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

    #[test]
    fn structural_equality_ignores_formatting_comments_and_key_order() {
        let left = "\u{feff}{\r\n  // actual\r\n  \"b\": 2,\r\n  \"a\": [1, 2,],\r\n}\r\n";
        let right = "{\"a\":[1,2],\"b\":2}";
        assert!(objects_equal(left, right));
    }

    #[test]
    fn structural_equality_keeps_types_array_order_and_big_integers_significant() {
        assert!(!objects_equal(r#"{"a":1}"#, r#"{"a":"1"}"#));
        assert!(!objects_equal(r#"{"a":[1,2]}"#, r#"{"a":[2,1]}"#));
        assert!(objects_equal(
            r#"{"n":9007199254740993}"#,
            r#"{"n":9007199254740993}"#
        ));
        assert!(!objects_equal(
            r#"{"n":9007199254740993}"#,
            r#"{"n":9007199254740992}"#
        ));
    }

    #[test]
    fn numeric_equality_is_exact_decimal_not_f64() {
        assert!(objects_equal(r#"{"n":1}"#, r#"{"n":1.0}"#));
        assert!(objects_equal(r#"{"n":1.20}"#, r#"{"n":1.2}"#));
        assert!(objects_equal(r#"{"n":1e2}"#, r#"{"n":100}"#));
        assert!(objects_equal(r#"{"n":-0}"#, r#"{"n":0}"#));
        assert!(objects_equal(
            r#"{"n":9007199254740993}"#,
            r#"{"n":9007199254740993.0}"#
        ));
        assert!(!objects_equal(r#"{"n":1}"#, r#"{"n":1e400}"#));
    }

    #[test]
    fn invalid_json_never_compares_equal_even_with_identical_bytes() {
        assert!(!objects_equal("{broken", "{broken"));
        assert!(!objects_equal("[]", "[]"));
        assert!(!objects_equal("[]", "{}"));
        assert!(!objects_equal("{broken", "{\"a\":1}"));
    }
}
