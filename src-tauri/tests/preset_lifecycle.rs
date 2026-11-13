mod support;

use serde_json::json;
use support::{preset, TestRoot};
use switcher_core::{storage, AppError, Target};

#[test]
fn legacy_storage_keeps_its_initialized_marker_when_presets_are_saved() {
    let root = TestRoot::new("preset-marker-preserved");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write(
        "data/presets.json",
        &json!({"formatVersion": 1, "presets": []}).to_string(),
    );

    let snapshot = root
        .engine()
        .save_preset(preset(Target::Opencode, "Alpha", "{\"a\":1}"))
        .expect("save preset");
    assert_eq!(snapshot.presets.len(), 1);

    let store = storage::read_presets_store(root.path()).expect("read store");
    assert!(store.initialized, "legacy file is already initialized");
    let raw: serde_json::Value =
        serde_json::from_str(&root.read("data/presets.json")).expect("stored json");
    assert_eq!(raw["firstImportCompleted"], true);
}

#[test]
fn create_assigns_id_and_preserves_content_exactly() {
    let root = TestRoot::new("preset-create");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let content = "{\n  \"model\": \"alpha\"\n}\n";

    let snapshot = root
        .engine()
        .save_preset(preset(Target::Opencode, "Alpha", content))
        .expect("create preset");

    assert_eq!(snapshot.presets.len(), 1);
    let stored = &snapshot.presets[0];
    assert_ne!(stored.id, "");
    assert_eq!(stored.name, "Alpha");
    assert_eq!(stored.target, Target::Opencode);
    assert_eq!(stored.content, content);

    let persisted = storage::read_presets(root.path()).expect("read stored presets");
    assert_eq!(persisted, snapshot.presets);

    let raw: serde_json::Value =
        serde_json::from_str(&root.read("data/presets.json")).expect("stored json");
    assert_eq!(raw["formatVersion"], 1);
    assert_eq!(raw["presets"][0]["content"], content);
}

#[test]
fn save_with_existing_id_renames_in_place() {
    let root = TestRoot::new("preset-rename");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let engine = root.engine();

    let first = engine
        .save_preset(preset(Target::Opencode, "Alpha", "{\"a\":1}"))
        .expect("create first")
        .presets[0]
        .clone();
    engine
        .save_preset(preset(Target::Omo, "Beta", "{\"b\":1}"))
        .expect("create second");

    let mut renamed = first.clone();
    renamed.name = "Alpha renamed".to_string();
    renamed.description = "updated description".to_string();
    renamed.content = "{\"a\":2}".to_string();
    let snapshot = engine.save_preset(renamed).expect("rename preset");

    assert_eq!(snapshot.presets.len(), 2);
    assert_eq!(snapshot.presets[0].id, first.id);
    assert_eq!(snapshot.presets[0].name, "Alpha renamed");
    assert_eq!(snapshot.presets[0].content, "{\"a\":2}");
    assert_eq!(snapshot.presets[1].name, "Beta");
}

#[test]
fn invalid_presets_are_rejected_without_writing_storage() {
    let root = TestRoot::new("preset-invalid");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let engine = root.engine();

    for content in ["[]", "{", "null", "\"text\"", "  "] {
        let error = engine
            .save_preset(preset(Target::Opencode, "Bad", content))
            .expect_err("invalid content must be rejected");
        assert!(
            matches!(error, AppError::PresetInvalid { .. }),
            "unexpected error for {content}: {error}"
        );
        assert!(!root.exists("data/presets.json"));
    }

    let error = engine
        .save_preset(preset(Target::Opencode, "   ", "{\"ok\":true}"))
        .expect_err("empty name must be rejected");
    assert!(matches!(error, AppError::PresetInvalid { .. }));
    assert!(!root.exists("data/presets.json"));
}

#[test]
fn save_with_unknown_id_is_rejected() {
    let root = TestRoot::new("preset-unknown");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");

    let mut candidate = preset(Target::Opencode, "Ghost", "{\"ok\":true}");
    candidate.id = "missing-id".to_string();
    let error = root
        .engine()
        .save_preset(candidate)
        .expect_err("unknown id must fail");
    assert!(matches!(error, AppError::PresetNotFound { .. }));
    assert!(!root.exists("data/presets.json"));
}

#[test]
fn delete_removes_preset_but_not_target_file() {
    let root = TestRoot::new("preset-delete");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let live = "{\"keep\":true}";
    root.write("targets/opencode.json", live);
    let engine = root.engine();

    let first = engine
        .save_preset(preset(Target::Opencode, "Alpha", live))
        .expect("create first")
        .presets[0]
        .id
        .clone();
    engine
        .save_preset(preset(Target::Opencode, "Beta", "{\"b\":1}"))
        .expect("create second");

    let snapshot = engine.delete_preset(&first).expect("delete preset");
    assert_eq!(snapshot.presets.len(), 1);
    assert_eq!(snapshot.presets[0].name, "Beta");
    assert_eq!(root.read("targets/opencode.json"), live);

    let error = engine
        .delete_preset(&first)
        .expect_err("second delete fails");
    assert!(matches!(error, AppError::PresetNotFound { .. }));
}
