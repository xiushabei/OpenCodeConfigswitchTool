mod support;

use support::TestRoot;
use switcher_core::AppError;

#[test]
fn strict_json_file_is_returned_byte_for_byte() {
    let root = TestRoot::new("import-strict");
    let content = "{\n  \"model\": \"exact\"\n}";
    let path = root.write("drop/config.json", content);

    let text = root
        .engine()
        .read_import_file(&path.display().to_string())
        .expect("read import file");
    assert_eq!(text, content);
}

#[test]
fn jsonc_file_is_normalized_to_strict_object_text() {
    let root = TestRoot::new("import-jsonc");
    let path = root.write(
        "drop/config.jsonc",
        "\u{feff}{\n  // comment\n  \"a\": 1, /* block */\n  \"b\": [2, 3,],\n}\n",
    );

    let text = root
        .engine()
        .read_import_file(&path.display().to_string())
        .expect("read jsonc");
    let value: serde_json::Value = serde_json::from_str(&text).expect("strict json");
    assert_eq!(value["a"], 1);
    assert_eq!(value["b"][1], 3);
    assert!(!text.contains("//"));
}

#[test]
fn unsupported_extension_and_missing_file_are_rejected() {
    let root = TestRoot::new("import-invalid");
    let text_path = root.write("drop/config.txt", "{}");
    let error = root
        .engine()
        .read_import_file(&text_path.display().to_string())
        .expect_err("txt rejected");
    assert!(matches!(error, AppError::ImportInvalid { .. }), "{error}");
    assert!(!root.exists("data/presets.json"));

    let missing = root.path().join("drop/missing.json");
    let error = root
        .engine()
        .read_import_file(&missing.display().to_string())
        .expect_err("missing rejected");
    assert!(matches!(error, AppError::ImportInvalid { .. }), "{error}");
}

#[test]
fn non_object_json_is_rejected() {
    let root = TestRoot::new("import-array");
    let path = root.write("drop/array.json", "[1, 2]");
    let error = root
        .engine()
        .read_import_file(&path.display().to_string())
        .expect_err("array rejected");
    assert!(matches!(error, AppError::ImportInvalid { .. }), "{error}");
}

#[test]
fn normalize_content_keeps_strict_object_text_byte_for_byte() {
    let root = TestRoot::new("normalize-strict");
    let content = " {\n  \"a\": 1\n}\n";

    let text = root
        .engine()
        .normalize_import_content(content)
        .expect("strict object passes through");
    assert_eq!(text, content);
    assert!(!root.exists("data/presets.json"));
}

#[test]
fn normalize_content_accepts_jsonc_comments_and_trailing_commas() {
    let root = TestRoot::new("normalize-jsonc");
    let content = "{\n  /* key */ \"a\" // colon\n  : 1,\n  \"b\": [2,],\n}";

    let text = root
        .engine()
        .normalize_import_content(content)
        .expect("jsonc normalized");
    let value: serde_json::Value = serde_json::from_str(&text).expect("strict json");
    assert_eq!(value["a"], 1);
    assert_eq!(value["b"][0], 2);
}

#[test]
fn normalize_content_rejects_joined_tokens_and_unclosed_comments() {
    let root = TestRoot::new("normalize-invalid");
    let engine = root.engine();

    for content in [
        "{\"a\": 1/*x*/2}",
        "{\"a\": tru/*x*/e}",
        "{\"a\":1} /* unterminated",
        "[1, 2]",
    ] {
        let error = engine
            .normalize_import_content(content)
            .expect_err("invalid content rejected");
        assert!(
            matches!(error, AppError::ImportInvalid { .. }),
            "{content}: {error}"
        );
    }
}
