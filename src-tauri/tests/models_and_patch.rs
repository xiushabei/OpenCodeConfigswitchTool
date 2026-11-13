use serde_json::{json, Value};
use switcher_core::models::{parse_output, sanitize_diagnostic};
use switcher_core::omo_patch::{patch, Section};
use switcher_core::AppError;

fn parse(output: &str) -> Value {
    serde_json::from_str(output).expect("patched content is JSON")
}

#[test]
fn parser_preserves_nested_model_paths_and_dedupes() {
    let stdout = b"openrouter/anthropic/claude-3.5-sonnet\r\nopencode/gpt-5\n\nopenrouter/anthropic/claude-3.5-sonnet\n";
    let models = parse_output(stdout).expect("valid output");
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].id, "openrouter/anthropic/claude-3.5-sonnet");
    assert_eq!(models[0].provider, "openrouter");
    assert_eq!(models[0].model, "anthropic/claude-3.5-sonnet");
    assert_eq!(models[1].id, "opencode/gpt-5");
}

#[test]
fn parser_rejects_noisy_lines_instead_of_inventing_models() {
    let stdout = b"loading models...\nopenai/gpt-5\n  spaced/line  \nhttps://example.com/a/b\n";
    let models = parse_output(stdout).expect("one valid line remains");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].id, "openai/gpt-5");

    let noisy = b"loading models...\nrefreshing models from models.dev\n";
    assert!(parse_output(noisy).is_err());
    assert_eq!(
        parse_output(b"").expect("empty output"),
        Vec::<switcher_core::models::ModelEntry>::new()
    );
    assert!(parse_output(&[0xff, 0xfe]).is_err());
}

#[test]
fn diagnostic_sanitizer_redacts_tokens_and_controls() {
    let raw = b"auth failed token=sk-abcdefghijklmnopqrstuvwxyz\nAuthorization: Bearer abcdefghijklmnopqrstuvwxyz012345\x1b[31m";
    let text = sanitize_diagnostic(raw);
    assert!(!text.contains("sk-abcdefghijklmnopqrstuvwxyz"));
    assert!(!text.contains("abcdefghijklmnopqrstuvwxyz012345"));
    assert!(text.contains("<redacted>"));
    assert!(!text.contains('\u{1b}'));
}

#[test]
fn patch_adds_agent_model_and_preserves_unknown_fields_and_big_ints() {
    let input = r#"{
        "theme": "system",
        "agents": {
            "build": {
                "model": "old/model",
                "temperature": 0.2,
                "budget": 123456789012345678901234567890,
                "x-extra": {"nested": [1, true, null]}
            }
        },
        "unknown": {"keep": "yes", "huge": 987654321098765432109876543210}
    }"#;
    let output = patch(
        input,
        Section::Agents,
        "build",
        Some("openrouter/anthropic/claude-3.5-sonnet"),
    )
    .expect("patch agent model");

    assert!(output.contains("123456789012345678901234567890"));
    assert!(output.contains("987654321098765432109876543210"));
    let value = parse(&output);
    assert_eq!(
        value["agents"]["build"]["model"],
        json!("openrouter/anthropic/claude-3.5-sonnet")
    );
    assert_eq!(value["agents"]["build"]["temperature"], json!(0.2));
    assert_eq!(
        value["agents"]["build"]["budget"].to_string(),
        "123456789012345678901234567890"
    );
    assert_eq!(
        value["agents"]["build"]["x-extra"],
        json!({"nested": [1, true, null]})
    );
    assert_eq!(value["theme"], json!("system"));
    assert_eq!(value["unknown"]["keep"], json!("yes"));
    assert_eq!(
        value["unknown"]["huge"].to_string(),
        "987654321098765432109876543210"
    );
}

#[test]
fn patch_adds_missing_section_and_entry() {
    let input = r#"{"agents":{"build":{"model":"a/b","keep":true}}}"#;
    let output = patch(input, Section::Categories, "fast", Some("openai/gpt-5"))
        .expect("patch new category");
    let value = parse(&output);
    assert_eq!(value["categories"]["fast"]["model"], json!("openai/gpt-5"));
    assert_eq!(value["agents"]["build"]["keep"], json!(true));

    let output = patch(input, Section::Agents, " plan ", Some("a/b")).expect("trim key");
    let value = parse(&output);
    assert_eq!(value["agents"]["plan"]["model"], json!("a/b"));
    assert_eq!(value["agents"]["build"]["model"], json!("a/b"));
}

#[test]
fn patch_null_removes_only_model_override() {
    let input = r#"{"agents":{"build":{"model":"a/b","temperature":0.2,"tools":{"x":true}}}}"#;
    let output = patch(input, Section::Agents, "build", None).expect("clear model");
    let value = parse(&output);
    assert_eq!(
        value["agents"]["build"],
        json!({"temperature": 0.2, "tools": {"x": true}})
    );
    assert!(value["agents"]["build"].get("model").is_none());
}

#[test]
fn patch_rejects_malformed_and_wrong_types() {
    assert!(patch("not json", Section::Agents, "build", Some("a/b")).is_err());
    assert!(patch("[]", Section::Agents, "build", Some("a/b")).is_err());
    assert!(patch(r#"{"agents":[]}"#, Section::Agents, "build", Some("a/b")).is_err());
    assert!(patch(
        r#"{"agents":{"build":"x"}}"#,
        Section::Agents,
        "build",
        Some("a/b")
    )
    .is_err());
    assert!(patch(r#"{"categories":"x"}"#, Section::Categories, "fast", None).is_err());
}

#[test]
fn patch_rejects_empty_prototype_and_blank_model_values() {
    for key in [
        "",
        "   ",
        "__proto__",
        "constructor",
        "prototype",
        " Prototype ",
    ] {
        assert!(
            patch("{}", Section::Agents, key, Some("a/b")).is_err(),
            "key {key:?} must be rejected"
        );
    }
    assert!(patch("{}", Section::Agents, "build", Some("   ")).is_err());
    assert!(patch("{}", Section::Categories, "fast", Some("")).is_err());
}

#[test]
fn patch_edits_opencode_wrapper_and_preserves_root_metadata_and_big_integers() {
    let input = r#"{
        "$schema": "root-schema",
        "_migrations": {"version": 3, "huge": 123456789012345678901234567890},
        "[opencode]": {
            "$schema": "wrapper-schema",
            "agents": {
                "build": {
                    "model": "old/model",
                    "budget": 9007199254740993,
                },
            },
            "categories": {"fast": {"model": "a/b"}},
            "extra": {"keep": true},
        },
    }"#;
    let output =
        patch(input, Section::Agents, "build", Some("new/model")).expect("patch wrapped OMO");

    assert!(output.contains("123456789012345678901234567890"));
    assert!(output.contains("9007199254740993"));
    let value = parse(&output);
    assert_eq!(value["$schema"], json!("root-schema"));
    assert_eq!(value["_migrations"]["version"], json!(3));
    assert_eq!(value["[opencode]"]["$schema"], json!("wrapper-schema"));
    assert_eq!(
        value["[opencode]"]["agents"]["build"]["model"],
        json!("new/model")
    );
    assert_eq!(
        value["[opencode]"]["categories"]["fast"]["model"],
        json!("a/b")
    );
    assert_eq!(value["[opencode]"]["extra"]["keep"], json!(true));
    assert!(value.get("agents").is_none());
}

#[test]
fn patch_accepts_jsonc_wrapper_and_rejects_non_object_wrapper() {
    let input = r#"{
        // wrapper config
        "[opencode]": {
            "agents": {"build": {"model": "old/model",},},
        },
    }"#;
    let output =
        patch(input, Section::Agents, "build", Some("new/model")).expect("patch JSONC wrapper");
    assert_eq!(
        parse(&output)["[opencode]"]["agents"]["build"]["model"],
        json!("new/model")
    );

    let error = patch(
        r#"{"[opencode]": []}"#,
        Section::Agents,
        "build",
        Some("new/model"),
    )
    .expect_err("non-object wrapper rejected");
    assert!(matches!(error, AppError::OmoPatchInvalid { .. }), "{error}");
}
