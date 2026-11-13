mod support;

use std::fs;

use serde_json::json;
use support::{preset, TestRoot};
use switcher_core::{AppError, Engine, Snapshot, Target};

fn initialized(root: &TestRoot) -> Engine {
    let engine = root.engine();
    engine.initialize_startup();
    engine
}

fn by_target(snapshot: &Snapshot, target: Target) -> &switcher_core::Preset {
    snapshot
        .presets
        .iter()
        .find(|preset| preset.target == target)
        .expect("imported preset")
}

fn backup_contents(root: &TestRoot) -> Vec<Vec<u8>> {
    let dir = root.path().join("backups");
    fs::read_dir(dir)
        .expect("read backups")
        .map(|entry| {
            let path = entry.expect("backup entry").path();
            fs::read(path).expect("read backup")
        })
        .collect()
}

fn backup_paths(root: &TestRoot) -> Vec<String> {
    let dir = root.path().join("backups");
    fs::read_dir(dir)
        .expect("read backups")
        .map(|entry| {
            let name = entry
                .expect("backup entry")
                .file_name()
                .to_string_lossy()
                .into_owned();
            format!("backups/{name}")
        })
        .collect()
}

fn lock_read_only(path: &std::path::Path) -> fs::Permissions {
    let original = fs::metadata(path).expect("metadata").permissions();
    let mut locked = original.clone();
    locked.set_readonly(true);
    fs::set_permissions(path, locked).expect("lock file");
    original
}

fn state(snapshot: &switcher_core::Snapshot, target: Target) -> &switcher_core::TargetState {
    snapshot
        .targets
        .iter()
        .find(|state| state.target == target)
        .expect("target state")
}

#[test]
fn first_startup_imports_once_and_later_launches_never_reimport() {
    let root = TestRoot::new("startup-first-only");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let opencode = "{\"model\":\"one\"}";
    root.write("targets/opencode.json", opencode);
    root.write(
        "targets/omo.json",
        "{\n  // launch config\n  \"agent\": {\"enabled\": true},\n}\n",
    );

    let engine = initialized(&root);
    let snapshot = engine.snapshot().expect("snapshot");
    assert!(snapshot.can_restore_startup);
    assert!(
        snapshot.startup_warnings.is_empty(),
        "{:?}",
        snapshot.startup_warnings
    );
    assert_eq!(snapshot.presets.len(), 2);
    assert_eq!(by_target(&snapshot, Target::Opencode).name, "当前 OpenCode");
    assert_eq!(by_target(&snapshot, Target::Omo).name, "当前 OMO");
    assert_eq!(by_target(&snapshot, Target::Opencode).content, opencode);
    assert!(!by_target(&snapshot, Target::Omo).content.contains("//"));
    let raw: serde_json::Value =
        serde_json::from_str(&root.read("data/presets.json")).expect("stored json");
    assert_eq!(raw["firstImportCompleted"], true);

    root.write("targets/opencode.json", "{\"model\":\"two\"}");
    root.write("targets/omo.json", "{\"agent\":{\"enabled\":false}}");
    let restarted = initialized(&root);
    let again = restarted.snapshot().expect("snapshot after restart");
    assert_eq!(
        again.presets.len(),
        2,
        "second startup must not import changed targets"
    );
    assert_eq!(again.presets, snapshot.presets);

    root.write("targets/opencode.json", "{\"model\":\"after-launch\"}");
    restarted
        .restore_startup()
        .expect("restore second baseline");
    assert_eq!(root.read("targets/opencode.json"), "{\"model\":\"two\"}");
    assert_eq!(
        root.read("targets/omo.json"),
        "{\"agent\":{\"enabled\":false}}"
    );
}

#[test]
fn first_startup_with_missing_targets_persists_marker_without_presets() {
    let root = TestRoot::new("startup-empty-marker");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");

    let engine = initialized(&root);
    let snapshot = engine.snapshot().expect("snapshot");
    assert_eq!(snapshot.presets, Vec::<switcher_core::Preset>::new());
    let raw: serde_json::Value =
        serde_json::from_str(&root.read("data/presets.json")).expect("stored json");
    assert_eq!(raw["firstImportCompleted"], true);

    root.write("targets/opencode.json", "{\"later\":true}");
    let restarted = initialized(&root);
    assert!(
        restarted.snapshot().expect("snapshot").presets.is_empty(),
        "a later launch must not import configs created after the first start"
    );

    let imported = restarted.import_current_configs().expect("manual import");
    assert_eq!(imported.presets.len(), 1);
    assert_eq!(imported.presets[0].name, "当前 OpenCode");
}

#[test]
fn legacy_storage_is_initialized_and_only_old_startup_names_are_renamed() {
    let root = TestRoot::new("startup-legacy-rename");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("targets/opencode.json", "{\"current\":true}");
    root.write("data/presets.json", &json!({
        "formatVersion": 1,
        "presets": [
            {"id": "startup-opencode-1", "name": "启动导入 OpenCode", "description": "old", "target": "opencode", "content": "{\"a\":1}"},
            {"id": "startup-omo-1", "name": "启动导入 OMO", "description": "old", "target": "omo", "content": "{\"b\":2}"},
            {"id": "custom-name", "name": "启动导入 OpenCode", "description": "user", "target": "opencode", "content": "{\"c\":3}"},
            {"id": "startup-user", "name": "我的配置", "description": "user", "target": "omo", "content": "{\"d\":4}"}
        ]
    }).to_string());

    let engine = initialized(&root);
    let snapshot = engine.snapshot().expect("snapshot");
    assert_eq!(
        snapshot.presets.len(),
        4,
        "legacy storage must not auto import"
    );
    assert_eq!(snapshot.presets[0].id, "startup-opencode-1");
    assert_eq!(snapshot.presets[1].id, "startup-omo-1");
    assert_eq!(snapshot.presets[0].name, "当前 OpenCode");
    assert_eq!(snapshot.presets[1].name, "当前 OMO");
    assert_eq!(snapshot.presets[0].content, "{\"a\":1}");
    assert_eq!(snapshot.presets[1].content, "{\"b\":2}");
    assert_eq!(snapshot.presets[2].name, "启动导入 OpenCode");
    assert_eq!(snapshot.presets[3].name, "我的配置");
    assert_eq!(snapshot.presets[2].id, "custom-name");
    assert_eq!(snapshot.presets[3].id, "startup-user");

    let raw: serde_json::Value =
        serde_json::from_str(&root.read("data/presets.json")).expect("stored json");
    assert_eq!(raw["firstImportCompleted"], true);
}

#[test]
fn manual_import_twice_appends_four_presets_without_touching_existing() {
    let root = TestRoot::new("import-manual-appends");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("targets/opencode.json", "{\"a\":1}");
    root.write("targets/omo.json", "{\"b\":2}");
    let engine = root.engine();
    let seeded = engine
        .save_preset(preset(Target::Opencode, "我的配置", "{\"custom\":true}"))
        .expect("seed preset")
        .presets[0]
        .clone();

    let first = engine.import_current_configs().expect("first import");
    assert_eq!(first.presets.len(), 3);
    assert_eq!(first.presets[0], seeded);
    assert_eq!(first.presets[1].name, "当前 OpenCode");
    assert_eq!(first.presets[2].name, "当前 OMO");
    assert!(
        first.startup_warnings.is_empty(),
        "{:?}",
        first.startup_warnings
    );

    let before = first.presets.clone();
    let second = engine.import_current_configs().expect("second import");
    assert_eq!(second.presets.len(), 5, "identical content still appends");
    assert_eq!(
        &second.presets[..3],
        &before[..],
        "old presets are unchanged"
    );
    assert_eq!(second.presets[3].name, "当前 OpenCode (2)");
    assert_eq!(second.presets[4].name, "当前 OMO (2)");
    assert_eq!(second.presets[3].target, Target::Opencode);
    assert_eq!(second.presets[4].target, Target::Omo);

    let mut ids: Vec<&str> = second
        .presets
        .iter()
        .map(|preset| preset.id.as_str())
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 5, "every imported preset gets a unique id");
}

#[test]
fn unreadable_preset_storage_is_reported_without_rewriting_it() {
    let root = TestRoot::new("startup-storage-error");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("targets/opencode.json", "{\"model\":\"one\"}");
    root.write("data/presets.json", "{broken");

    let engine = initialized(&root);
    let error = engine.snapshot().expect_err("corrupt storage is visible");
    assert!(
        matches!(error, AppError::PresetStorageInvalid { .. }),
        "{error}"
    );
    assert_eq!(root.read("data/presets.json"), "{broken");
}

#[test]
fn invalid_current_config_is_a_warning_and_never_a_preset() {
    let root = TestRoot::new("startup-invalid");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("targets/opencode.json", "{broken");

    let engine = initialized(&root);
    let snapshot = engine.snapshot().expect("snapshot still opens");
    assert_eq!(snapshot.presets, Vec::<switcher_core::Preset>::new());
    assert!(snapshot.can_restore_startup, "bytes are still restorable");
    assert!(
        snapshot
            .startup_warnings
            .iter()
            .any(|warning| warning.contains("未导入 OpenCode")),
        "{:?}",
        snapshot.startup_warnings
    );
}

#[test]
fn restore_writes_exact_launch_bytes_to_original_paths_after_settings_change() {
    let root = TestRoot::new("restore-settings");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let opencode = "{\r\n  \"model\": \"launch\"\r\n}";
    let omo = "{\"omo\":true}";
    root.write("targets/opencode.json", opencode);
    root.write("targets/omo.json", omo);

    let engine = initialized(&root);
    let settings = {
        let mut settings = engine.snapshot().expect("snapshot").settings;
        settings.targets.opencode = "next/opencode.json".to_string();
        settings.targets.omo = "next/omo.json".to_string();
        settings
    };
    let changed = engine.save_settings(settings).expect("save settings");
    assert!(
        changed.can_restore_startup,
        "settings change must keep baseline"
    );
    assert_eq!(changed.settings.targets.opencode, "next/opencode.json");

    root.write("targets/opencode.json", "{\"model\":\"changed\"}");
    root.write("targets/omo.json", "{\"omo\":\"changed\"}");
    root.write("next/opencode.json", "{\"next\":\"opencode\"}");
    root.write("next/omo.json", "{\"next\":\"omo\"}");

    let result = engine.restore_startup().expect("restore");
    assert_eq!(
        fs::read(root.path().join("targets/opencode.json")).expect("opencode"),
        opencode.as_bytes()
    );
    assert_eq!(
        fs::read(root.path().join("targets/omo.json")).expect("omo"),
        omo.as_bytes()
    );
    assert_eq!(root.read("next/opencode.json"), "{\"next\":\"opencode\"}");
    assert_eq!(root.read("next/omo.json"), "{\"next\":\"omo\"}");
    assert!(result.message.contains("OpenCode"));
    assert!(result.message.contains("重启"));

    let backups = backup_contents(&root);
    assert_eq!(backups.len(), 2);
    assert!(backups.contains(&b"{\"model\":\"changed\"}".to_vec()));
    assert!(backups.contains(&b"{\"omo\":\"changed\"}".to_vec()));

    let snapshot = engine.snapshot().expect("snapshot after restore");
    assert_eq!(snapshot.settings.targets.opencode, "next/opencode.json");
    assert!(snapshot.can_restore_startup);
    assert_eq!(
        state(&snapshot, Target::Opencode).content.as_deref(),
        Some("{\"next\":\"opencode\"}")
    );

    engine.restore_startup().expect("second restore");
    assert_eq!(
        fs::read(root.path().join("targets/opencode.json")).expect("opencode"),
        opencode.as_bytes()
    );
    assert_eq!(backup_contents(&root).len(), 4, "baseline stayed immutable");
}

#[test]
fn file_missing_at_launch_is_removed_only_after_backup() {
    let root = TestRoot::new("restore-absent");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let opencode = "{\"model\":\"launch\"}";
    root.write("targets/opencode.json", opencode);

    let engine = initialized(&root);
    root.write("targets/opencode.json", "{\"model\":\"changed\"}");
    let later = "{\"created\":\"later\"}";
    root.write("targets/omo.json", later);

    let result = engine.restore_startup().expect("restore");
    assert!(
        !root.exists("targets/omo.json"),
        "absent-at-launch file removed"
    );
    assert_eq!(root.read("targets/opencode.json"), opencode);
    assert!(result.message.contains("OMO 已删除"));
    let backups = backup_contents(&root);
    assert_eq!(backups.len(), 2);
    assert!(backups.contains(&later.as_bytes().to_vec()));
}

#[test]
fn partial_restore_reports_all_target_states_and_every_backup_path() {
    let root = TestRoot::new("restore-partial");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let opencode = "{\"model\":\"launch\"}";
    root.write("targets/opencode.json", opencode);
    root.write("targets/omo.json", "{\"omo\":\"launch\"}");

    let engine = initialized(&root);
    root.write("targets/opencode.json", "{\"model\":\"changed\"}");
    let omo_changed = "{\"omo\":\"changed\"}";
    root.write("targets/omo.json", omo_changed);

    let omo_path = root.path().join("targets/omo.json");
    let original_permissions = lock_read_only(&omo_path);
    let error = engine
        .restore_startup()
        .expect_err("read-only omo blocks the second write");
    fs::set_permissions(&omo_path, original_permissions).expect("unlock omo");

    let AppError::RestorePartial { reason } = error else {
        panic!("expected RestorePartial, got {error}");
    };
    assert!(reason.contains("已变更：OpenCode 已恢复"), "{reason}");
    assert!(reason.contains("失败：OMO"), "{reason}");
    assert!(reason.contains("未处理：无"), "{reason}");
    let backups = backup_paths(&root);
    assert_eq!(backups.len(), 2, "{backups:?}");
    for backup in &backups {
        assert!(reason.contains(backup), "{backup} missing from {reason}");
    }

    assert_eq!(root.read("targets/opencode.json"), opencode);
    assert_eq!(root.read("targets/omo.json"), omo_changed);
}

#[test]
fn restore_without_startup_capture_fails_without_touching_targets() {
    let root = TestRoot::new("restore-uninitialized");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    root.write("targets/opencode.json", "{\"keep\":true}");
    let engine = root.engine();

    assert!(!engine.can_restore_startup());
    let error = engine.restore_startup().expect_err("no baseline");
    assert!(matches!(error, AppError::StartupBaselineMissing), "{error}");
    assert_eq!(root.read("targets/opencode.json"), "{\"keep\":true}");
    assert!(!root.exists("backups"));
}
