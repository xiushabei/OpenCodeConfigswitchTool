mod support;

use support::{preset, TestRoot};
use switcher_core::{AppError, Target};

#[test]
fn snapshot_marks_active_for_structurally_equivalent_presets() {
    let root = TestRoot::new("snapshot-active");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let content = "{\n  \"model\": \"one\"\n}\n";
    root.write("targets/opencode.json", content);
    let engine = root.engine();

    let first_id = engine
        .save_preset(preset(Target::Opencode, "One", content))
        .expect("save first")
        .presets[0]
        .id
        .clone();
    let second_id = engine
        .save_preset(preset(
            Target::Opencode,
            "Reformatted",
            "{\"model\":\"one\"}",
        ))
        .expect("save reformatted")
        .presets[1]
        .id
        .clone();

    let snapshot = engine.snapshot().expect("snapshot");
    let opencode = snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Opencode)
        .expect("opencode state");
    assert_eq!(opencode.active_preset_ids, vec![first_id, second_id]);
    assert_eq!(opencode.content.as_deref(), Some(content));
    assert_eq!(opencode.error, None);
}

#[test]
fn snapshot_marks_active_for_jsonc_target_with_comments_crlf_and_reordered_keys() {
    let root = TestRoot::new("snapshot-jsonc-active");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let target = "{\r\n  // actual config\r\n  \"b\": 2,\r\n  \"n\": 9007199254740993,\r\n  \"a\": [1, 2,],\r\n}\r\n";
    root.write("targets/omo.json", target);
    let engine = root.engine();

    let id = engine
        .save_preset(preset(
            Target::Omo,
            "Current",
            "{\"a\":[1,2],\"n\":9007199254740993,\"b\":2}",
        ))
        .expect("save preset")
        .presets[0]
        .id
        .clone();

    let snapshot = engine.snapshot().expect("snapshot");
    let omo = snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Omo)
        .expect("omo state");
    assert_eq!(omo.active_preset_ids, vec![id]);
}

#[test]
fn snapshot_marks_active_when_numbers_differ_only_in_spelling() {
    let root = TestRoot::new("snapshot-number-spelling");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("targets/omo.json", "{\"ratio\":1,\"scale\":100}");
    let engine = root.engine();

    let id = engine
        .save_preset(preset(
            Target::Omo,
            "Decimal",
            "{\"ratio\":1.0,\"scale\":1e2}",
        ))
        .expect("save decimal preset")
        .presets[0]
        .id
        .clone();

    let snapshot = engine.snapshot().expect("snapshot");
    let omo = snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Omo)
        .expect("omo state");
    assert_eq!(omo.active_preset_ids, vec![id]);
}

#[test]
fn structural_differences_keep_presets_inactive() {
    let root = TestRoot::new("snapshot-structural-negative");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write(
        "targets/omo.json",
        "{\"model\":\"one\",\"budget\":9007199254740993,\"list\":[1,2]}",
    );
    let engine = root.engine();

    for (name, content) in [
        (
            "String number",
            "{\"model\":\"one\",\"budget\":\"9007199254740993\",\"list\":[1,2]}",
        ),
        (
            "Rounded big integer",
            "{\"model\":\"one\",\"budget\":9007199254740992,\"list\":[1,2]}",
        ),
        (
            "Reordered array",
            "{\"model\":\"one\",\"budget\":9007199254740993,\"list\":[2,1]}",
        ),
        (
            "Changed field",
            "{\"model\":\"two\",\"budget\":9007199254740993,\"list\":[1,2]}",
        ),
    ] {
        engine
            .save_preset(preset(Target::Omo, name, content))
            .expect("save preset");
    }

    let snapshot = engine.snapshot().expect("snapshot");
    let omo = snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Omo)
        .expect("omo state");
    assert!(
        omo.active_preset_ids.is_empty(),
        "{:?}",
        omo.active_preset_ids
    );
}

#[test]
fn external_target_change_clears_active_marking() {
    let root = TestRoot::new("snapshot-external");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let content = "{\"model\":\"one\"}";
    root.write("targets/opencode.json", content);
    let engine = root.engine();
    engine
        .save_preset(preset(Target::Opencode, "One", content))
        .expect("save preset");

    root.write("targets/opencode.json", "{\"model\":\"external\"}");
    let snapshot = engine.snapshot().expect("snapshot");

    let opencode = snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Opencode)
        .expect("opencode state");
    assert_eq!(opencode.active_preset_ids, Vec::<String>::new());
    assert_eq!(
        opencode.content.as_deref(),
        Some("{\"model\":\"external\"}")
    );
}

#[test]
fn non_utf8_target_reports_error_without_content() {
    let root = TestRoot::new("snapshot-utf8");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write_bytes("targets/opencode.json", &[0xff, 0xfe, 0x00]);

    let snapshot = root.engine().snapshot().expect("snapshot");
    let opencode = snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Opencode)
        .expect("opencode state");
    assert_eq!(opencode.content, None);
    assert!(opencode.error.is_some());
    assert_eq!(opencode.active_preset_ids, Vec::<String>::new());
}

#[test]
fn corrupt_preset_storage_is_surfaced_by_snapshot() {
    let root = TestRoot::new("snapshot-corrupt");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("data/presets.json", "{not json");

    let error = root
        .engine()
        .snapshot()
        .expect_err("invalid storage must fail");
    assert!(
        matches!(error, AppError::PresetStorageInvalid { .. }),
        "{error}"
    );

    root.write("data/presets.json", "{\"formatVersion\":99,\"presets\":[]}");
    let error = root
        .engine()
        .snapshot()
        .expect_err("unsupported version must fail");
    assert!(
        matches!(error, AppError::PresetStorageInvalid { .. }),
        "{error}"
    );
}

#[test]
fn export_writes_contract_envelope_inside_project() {
    let root = TestRoot::new("export-envelope");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let content = "{\"model\":\"exported\"}";
    let engine = root.engine();
    let id = engine
        .save_preset(preset(Target::Omo, "Exported", content))
        .expect("save preset")
        .presets[0]
        .id
        .clone();

    let relative = engine.export_preset(&id).expect("export preset");
    assert!(
        relative.starts_with("exports/"),
        "unexpected path: {relative}"
    );
    assert_eq!(
        std::path::Path::new(&relative).extension(),
        Some(std::ffi::OsStr::new("json"))
    );

    let raw: serde_json::Value = serde_json::from_str(&root.read(&relative)).expect("export json");
    assert_eq!(raw["formatVersion"], 1);
    assert_eq!(raw["preset"]["id"], id);
    assert_eq!(raw["preset"]["content"], content);
    assert_eq!(raw["preset"]["target"], "omo");

    let mut top_keys: Vec<&str> = raw
        .as_object()
        .expect("envelope object")
        .keys()
        .map(String::as_str)
        .collect();
    top_keys.sort_unstable();
    assert_eq!(top_keys, vec!["formatVersion", "preset"]);

    let mut preset_keys: Vec<&str> = raw["preset"]
        .as_object()
        .expect("preset object")
        .keys()
        .map(String::as_str)
        .collect();
    preset_keys.sort_unstable();
    assert_eq!(
        preset_keys,
        vec!["content", "description", "id", "name", "target"]
    );
}

#[test]
fn command_payloads_use_frontend_field_names() {
    let root = TestRoot::new("snapshot-shape");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let content = "{\"model\":\"shape\"}";
    root.write("targets/opencode.json", content);
    let engine = root.engine();
    let id = engine
        .save_preset(preset(Target::Opencode, "Shape", content))
        .expect("save preset")
        .presets[0]
        .id
        .clone();

    let snapshot = serde_json::to_value(engine.snapshot().expect("snapshot")).expect("json");
    for key in ["settings", "presets", "targets", "root"] {
        assert!(snapshot.get(key).is_some(), "missing snapshot key {key}");
    }
    let target = &snapshot["targets"][0];
    assert!(target.get("activePresetIds").is_some());
    assert!(target.get("active_preset_ids").is_none());
    assert_eq!(snapshot["presets"][0]["target"], "opencode");
    assert_eq!(snapshot["settings"]["theme"], "system");

    let applied = serde_json::to_value(engine.apply_preset(&id).expect("apply")).expect("json");
    for key in ["snapshot", "backup", "message"] {
        assert!(applied.get(key).is_some(), "missing apply key {key}");
    }
    assert!(applied["backup"]
        .as_str()
        .is_some_and(|path| path.starts_with("backups/")));
}

#[test]
fn export_unknown_preset_is_rejected() {
    let root = TestRoot::new("export-unknown");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");

    let error = root
        .engine()
        .export_preset("missing")
        .expect_err("unknown preset must fail");
    assert!(matches!(error, AppError::PresetNotFound { .. }), "{error}");
    assert!(!root.exists("exports"));
}
