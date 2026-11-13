mod support;

use std::fs;

use support::TestRoot;
use switcher_core::{paths, AppError, Settings, Target, TargetPaths, Theme};

fn settings(opencode: &str, omo: &str, theme: Theme) -> Settings {
    Settings {
        targets: TargetPaths {
            opencode: opencode.to_string(),
            omo: omo.to_string(),
        },
        theme,
    }
}

#[test]
fn save_settings_persists_theme_and_relative_targets() {
    let root = TestRoot::new("settings-save");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let engine = root.engine();

    let snapshot = engine
        .save_settings(settings("targets/a.json", "targets/b.json", Theme::Dark))
        .expect("save settings");

    assert_eq!(snapshot.settings.theme, Theme::Dark);
    assert_eq!(snapshot.settings.targets.opencode, "targets/a.json");

    let raw: serde_json::Value =
        serde_json::from_str(&root.read("switcher.settings.json")).expect("stored settings");
    assert_eq!(raw["theme"], "dark");
    assert_eq!(raw["targets"]["opencode"], "targets/a.json");

    let opencode = snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Opencode)
        .expect("opencode state");
    assert!(paths::same_path(
        std::path::Path::new(&opencode.path),
        &root.path().join("targets/a.json")
    ));
}

#[test]
fn collisions_with_application_owned_paths_are_rejected() {
    let root = TestRoot::new("settings-collision");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let engine = root.engine();
    let before = root.read("switcher.settings.json");

    let colliding = [
        ("data/presets.json", "targets/ok.json"),
        ("backups/old.json", "targets/ok.json"),
        ("exports/old.json", "targets/ok.json"),
        ("data/webview/x.json", "targets/ok.json"),
        ("switcher.settings.json", "targets/ok.json"),
        ("src/main.ts", "targets/ok.json"),
        ("src-tauri/Cargo.toml", "targets/ok.json"),
        ("targets/same.json", "targets/same.json"),
        ("data./presets.json", "targets/ok.json"),
        ("NUL", "targets/ok.json"),
        ("COM1.json", "targets/ok.json"),
        (".", "targets/ok.json"),
    ];

    for (opencode, omo) in colliding {
        let error = engine
            .save_settings(settings(opencode, omo, Theme::System))
            .expect_err("collision must be rejected");
        assert!(
            matches!(
                error,
                AppError::PathCollision { .. } | AppError::TargetPathInvalid { .. }
            ),
            "case {opencode}: {error}"
        );
    }

    assert_eq!(root.read("switcher.settings.json"), before);
}

#[test]
fn absolute_targets_are_accepted_when_explicit() {
    let root = TestRoot::new("settings-absolute");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    let absolute = root.path().join("targets/absolute.json");

    let snapshot = root
        .engine()
        .save_settings(settings(
            &absolute.display().to_string(),
            "targets/omo.json",
            Theme::Light,
        ))
        .expect("absolute target accepted");

    let opencode = snapshot
        .targets
        .iter()
        .find(|state| state.target == Target::Opencode)
        .expect("opencode state");
    assert!(paths::same_path(
        std::path::Path::new(&opencode.path),
        &absolute
    ));
}

#[test]
fn target_directory_is_rejected() {
    let root = TestRoot::new("settings-directory");
    root.seed_settings("targets/opencode.json", "targets/omo.json", "system");
    fs::create_dir_all(root.path().join("targets/config-dir")).expect("create dir");

    let error = root
        .engine()
        .save_settings(settings(
            "targets/config-dir",
            "targets/omo.json",
            Theme::System,
        ))
        .expect_err("directory target must be rejected");
    assert!(
        matches!(error, AppError::TargetPathInvalid { .. }),
        "{error}"
    );
}

#[test]
fn tilde_expands_against_injected_home() {
    let root = TestRoot::new("paths-tilde");
    let home = root.path().join("home");
    fs::create_dir_all(&home).expect("create fake home");

    let resolved =
        paths::resolve_target_in(root.path(), "~/.config/opencode/opencode.json", Some(&home))
            .expect("expand tilde");
    assert_eq!(resolved, home.join(".config/opencode/opencode.json"));

    let error = paths::resolve_target_in(root.path(), "~/x.json", None)
        .expect_err("missing home must fail");
    assert!(matches!(error, AppError::TargetPathInvalid { .. }));

    let error = paths::resolve_target_in(root.path(), "~/", Some(&home))
        .expect_err("directory path must fail");
    assert!(matches!(error, AppError::TargetPathInvalid { .. }));
}

#[test]
fn root_discovery_prefers_executable_then_falls_back_to_cwd() {
    let exe_root = TestRoot::new("discovery-exe");
    exe_root.write("switcher.settings.json", "{}");
    let nested = exe_root.path().join("a/b/c");
    fs::create_dir_all(&nested).expect("create nested");

    let cwd_root = TestRoot::new("discovery-cwd");
    cwd_root.write("switcher.settings.json", "{}");

    let from_exe = paths::discover_root(Some(&nested.join("app.exe")), cwd_root.path())
        .expect("exe ancestors contain settings");
    assert!(paths::same_path(&from_exe, exe_root.path()));

    let from_cwd = paths::discover_root(
        Some(std::path::Path::new("C:/Windows/explorer.exe")),
        cwd_root.path(),
    )
    .expect("cwd ancestors contain settings");
    assert!(paths::same_path(&from_cwd, cwd_root.path()));
}

#[test]
fn missing_settings_file_is_reported_clearly() {
    let root = TestRoot::new("settings-missing");
    let error = root
        .engine()
        .snapshot()
        .expect_err("missing settings must fail");
    assert!(matches!(error, AppError::Io { .. }), "{error}");
}
