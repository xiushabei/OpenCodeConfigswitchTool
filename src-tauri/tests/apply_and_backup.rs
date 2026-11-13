mod support;

use std::fs;

use serde_json::json;
use support::{preset, TestRoot};
use switcher_core::{AppError, Target};

#[test]
fn apply_writes_exact_content_and_backs_up_original_bytes() {
    let root = TestRoot::new("apply-bytes");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let original = "{\r\n  \"model\": \"old\"\r\n}\r\n";
    root.write("targets/opencode.json", original);
    let content = "{\"model\":\"new\"}\r\n";
    let engine = root.engine();

    let id = engine
        .save_preset(preset(Target::Opencode, "New", content))
        .expect("save preset")
        .presets[0]
        .id
        .clone();
    let result = engine.apply_preset(&id).expect("apply preset");

    assert_eq!(
        fs::read(root.path().join("targets/opencode.json")).expect("read target"),
        content.as_bytes()
    );

    let backup = result.backup.expect("existing target must be backed up");
    assert!(
        backup.starts_with("backups/"),
        "unexpected backup path: {backup}"
    );
    assert_eq!(
        fs::read(root.path().join(&backup)).expect("read backup"),
        original.as_bytes()
    );

    assert!(result.message.contains("OpenCode"));
    assert!(result.message.contains("重启"));

    let opencode = result
        .snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Opencode)
        .expect("opencode state");
    assert_eq!(opencode.active_preset_ids, vec![id]);
    assert_eq!(opencode.content.as_deref(), Some(content));

    let omo = result
        .snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Omo)
        .expect("omo state");
    assert_eq!(omo.content, None);
    assert_eq!(omo.error, None);
    assert!(!root.exists("targets/omo.json"));
}

#[test]
fn apply_to_missing_target_writes_without_backup() {
    let root = TestRoot::new("apply-missing");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let engine = root.engine();
    let id = engine
        .save_preset(preset(Target::Omo, "Fresh", "{\"fresh\":true}"))
        .expect("save preset")
        .presets[0]
        .id
        .clone();

    let result = engine.apply_preset(&id).expect("apply preset");
    assert_eq!(result.backup, None);
    assert_eq!(root.read("targets/omo.json"), "{\"fresh\":true}");
    assert_eq!(
        root.read("targets/omo.json").as_bytes(),
        b"{\"fresh\":true}"
    );
    assert!(!root.exists("backups"));
}

#[test]
fn invalid_stored_preset_content_never_touches_target() {
    let root = TestRoot::new("apply-invalid");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let original = "{\"original\":true}";
    root.write("targets/opencode.json", original);
    let engine = root.engine();

    let id = engine
        .save_preset(preset(Target::Opencode, "Seed", "{\"seed\":true}"))
        .expect("save preset")
        .presets[0]
        .id
        .clone();
    let tampered = json!({
        "formatVersion": 1,
        "presets": [{
            "id": id,
            "name": "Tampered",
            "description": "",
            "target": "opencode",
            "content": "[]"
        }]
    });
    root.write("data/presets.json", &tampered.to_string());

    let error = engine
        .apply_preset(&id)
        .expect_err("invalid content rejected");
    assert!(matches!(error, AppError::PresetInvalid { .. }), "{error}");
    assert_eq!(root.read("targets/opencode.json"), original);
    assert!(!root.exists("backups"));
}

#[test]
fn corrupt_preset_storage_blocks_apply() {
    let root = TestRoot::new("apply-corrupt");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let original = "{\"original\":true}";
    root.write("targets/opencode.json", original);

    root.write("data/presets.json", "{not json");
    let error = root
        .engine()
        .apply_preset("anything")
        .expect_err("malformed storage must fail");
    assert!(
        matches!(error, AppError::PresetStorageInvalid { .. }),
        "{error}"
    );
    assert_eq!(root.read("targets/opencode.json"), original);

    root.write("data/presets.json", "{\"formatVersion\":99,\"presets\":[]}");
    let error = root
        .engine()
        .apply_preset("anything")
        .expect_err("unsupported storage version must fail");
    assert!(
        matches!(error, AppError::PresetStorageInvalid { .. }),
        "{error}"
    );
    assert_eq!(root.read("targets/opencode.json"), original);
    assert!(!root.exists("backups"));
}

#[test]
fn repeated_applies_create_unique_backups() {
    let root = TestRoot::new("apply-repeat");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("targets/opencode.json", "{\"generation\":0}");
    let engine = root.engine();

    let first = engine
        .save_preset(preset(Target::Opencode, "One", "{\"generation\":1}"))
        .expect("save preset")
        .presets[0]
        .id
        .clone();
    let second = engine
        .save_preset(preset(Target::Opencode, "Two", "{\"generation\":2}"))
        .expect("save second")
        .presets[1]
        .id
        .clone();

    let first_result = engine.apply_preset(&first).expect("apply first");
    let second_result = engine.apply_preset(&second).expect("apply second");
    assert_ne!(first_result.backup, second_result.backup);

    let backups: Vec<String> = fs::read_dir(root.path().join("backups"))
        .expect("read backups dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(backups.len(), 2);
    assert_eq!(root.read("targets/opencode.json"), "{\"generation\":2}");
    assert_eq!(
        root.read(second_result.backup.as_deref().expect("second backup")),
        "{\"generation\":1}"
    );
}

#[test]
fn apply_unknown_id_leaves_no_backup_directory() {
    let root = TestRoot::new("apply-unknown");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("targets/opencode.json", "{\"keep\":true}");

    let error = root
        .engine()
        .apply_preset("missing")
        .expect_err("unknown preset must fail");
    assert!(matches!(error, AppError::PresetNotFound { .. }), "{error}");
    assert_eq!(root.read("targets/opencode.json"), "{\"keep\":true}");
    assert!(!root.exists("backups"));
}
