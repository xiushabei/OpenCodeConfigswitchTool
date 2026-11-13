use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

use serde_json::Value;

use crate::error::AppError;
use crate::model::{Preset, Settings, Target, BACKUPS_DIR, SETTINGS_FILE};
use crate::{import, jsonc, paths, storage};

const CURRENT_PREFIX: &str = "当前 ";
const LEGACY_PREFIX: &str = "启动导入 ";
const ID_PREFIX: &str = "import-";
const LEGACY_OMO_DEFAULT: &str = "~/.config/opencode/oh-my-openagent.json";
const CURRENT_OMO_DEFAULT: &str = "~/.omo/omo.jsonc";

pub(crate) struct Candidate {
    pub target: Target,
    pub bytes: Option<Vec<u8>>,
    pub read_error: Option<String>,
}

pub(crate) struct ImportOutcome {
    pub warnings: Vec<String>,
}

/// Appends one preset per readable candidate, even when an identical preset already exists.
pub(crate) fn append_candidates(
    presets: &mut Vec<Preset>,
    candidates: &[Candidate],
    warnings: &mut Vec<String>,
) -> usize {
    let mut added = 0;
    for candidate in candidates {
        if let Some(content) = normalized_content(candidate, warnings) {
            push_preset(presets, candidate.target, content);
            added += 1;
        }
    }
    added
}

/// Appends only candidates whose content is not already represented structurally.
pub(crate) fn append_missing_candidates(
    presets: &mut Vec<Preset>,
    candidates: &[Candidate],
    warnings: &mut Vec<String>,
) -> usize {
    let mut added = 0;
    for candidate in candidates {
        let Some(content) = normalized_content(candidate, warnings) else {
            continue;
        };
        let already_present = presets.iter().any(|preset| {
            preset.target == candidate.target && jsonc::objects_equal(&preset.content, &content)
        });
        if already_present {
            continue;
        }
        push_preset(presets, candidate.target, content);
        added += 1;
    }
    added
}

/// Reports whether `raw` resolves to the current default OMO configuration path.
pub(crate) fn is_current_omo_default(root: &Path, raw: &str) -> bool {
    is_current_omo_default_in(root, raw, paths::home_dir().as_deref())
}

/// Renames presets created by the old automatic import, preserving their ids and content.
///
/// Only ids starting with `startup-` whose name is exactly the old label are touched.
pub(crate) fn rename_legacy_presets(presets: &mut [Preset]) -> bool {
    let mut changed = false;
    for index in 0..presets.len() {
        if !presets[index].id.starts_with("startup-") {
            continue;
        }
        let Some(label) = presets[index].name.strip_prefix(LEGACY_PREFIX) else {
            continue;
        };
        if !matches!(label, "OpenCode" | "OMO") {
            continue;
        }
        let base = format!("{CURRENT_PREFIX}{label}");
        let name = unique_name(presets, Some(index), &base);
        if presets[index].name != name {
            presets[index].name = name;
            changed = true;
        }
    }
    changed
}

/// Reads the current settings targets and appends their presets; never replaces existing ones.
///
/// # Errors
///
/// Returns [`AppError`] when settings or preset storage cannot be read, or the updated preset
/// storage cannot be written.
pub(crate) fn import_current(root: &Path) -> Result<ImportOutcome, AppError> {
    let settings = storage::read_settings(root)?;
    let store = storage::read_presets_store(root)?;
    let mut presets = store.presets;
    let mut warnings = Vec::new();
    rename_legacy_presets(&mut presets);
    let candidates = read_candidates(root, &settings);
    append_candidates(&mut presets, &candidates, &mut warnings);
    storage::write_presets(root, &presets, true)?;
    Ok(ImportOutcome { warnings })
}

/// Rewrites the settings OMO target from the legacy default to the current default once.
///
/// Migration requires the configured OMO path to be exactly the legacy default and the current
/// default file to exist. Custom paths are never replaced. The original settings bytes are backed
/// up under `backups/` before the rewrite. Returns `true` only when settings were changed.
pub(crate) fn migrate_legacy_omo_path(root: &Path, warnings: &mut Vec<String>) -> bool {
    migrate_legacy_omo_path_in(root, paths::home_dir().as_deref(), warnings)
}

fn is_current_omo_default_in(root: &Path, raw: &str, home: Option<&Path>) -> bool {
    let Ok(configured) = paths::resolve_target_in(root, raw, home) else {
        return false;
    };
    let Ok(default) = paths::resolve_target_in(root, CURRENT_OMO_DEFAULT, home) else {
        return false;
    };
    paths::same_path(&configured, &default)
}

fn migrate_legacy_omo_path_in(
    root: &Path,
    home: Option<&Path>,
    warnings: &mut Vec<String>,
) -> bool {
    let settings_path = root.join(SETTINGS_FILE);
    let original = match fs::read(&settings_path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == ErrorKind::NotFound => return false,
        Err(err) => {
            warnings.push(format!("无法读取设置文件，跳过旧 OMO 路径迁移：{err}"));
            return false;
        }
    };
    let mut value: Value = match serde_json::from_slice(&original) {
        Ok(value) => value,
        Err(err) => {
            warnings.push(format!("设置文件无法解析，跳过旧 OMO 路径迁移：{err}"));
            return false;
        }
    };
    let Some(targets) = value.get_mut("targets").and_then(Value::as_object_mut) else {
        return false;
    };
    let Some(configured) = targets.get("omo").and_then(Value::as_str) else {
        return false;
    };
    if configured.trim() != LEGACY_OMO_DEFAULT {
        return false;
    }
    let Ok(new_path) = paths::resolve_target_in(root, CURRENT_OMO_DEFAULT, home) else {
        return false;
    };
    if !new_path.is_file() {
        return false;
    }
    let collides = targets
        .get("opencode")
        .and_then(Value::as_str)
        .is_some_and(|opencode| {
            paths::resolve_target_in(root, opencode, home)
                .is_ok_and(|opencode_path| paths::same_path(&new_path, &opencode_path))
        });
    if collides {
        warnings.push("旧 OMO 路径迁移已跳过：当前默认 OMO 路径与 OpenCode 目标相同。".to_string());
        return false;
    }
    if let Err(err) = write_settings_backup(root, &original) {
        warnings.push(format!("旧 OMO 路径迁移已跳过：无法备份设置文件：{err}"));
        return false;
    }
    targets.insert(
        "omo".to_string(),
        Value::String(CURRENT_OMO_DEFAULT.to_string()),
    );
    let mut json = match serde_json::to_string_pretty(&value) {
        Ok(json) => json,
        Err(err) => {
            warnings.push(format!("旧 OMO 路径迁移已跳过：无法序列化设置文件：{err}"));
            return false;
        }
    };
    json.push('\n');
    if let Err(err) = storage::atomic_write(&settings_path, json.as_bytes()) {
        warnings.push(format!("旧 OMO 路径迁移已跳过：无法写入设置文件：{err}"));
        return false;
    }
    true
}

fn write_settings_backup(root: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let dir = root.join(BACKUPS_DIR);
    fs::create_dir_all(&dir).map_err(|err| AppError::io("创建备份目录", &dir, err))?;
    let stamp = storage::timestamp_millis();
    for attempt in 0..100 {
        let name = if attempt == 0 {
            format!("switcher.settings-{stamp}.json")
        } else {
            format!("switcher.settings-{stamp}-{attempt}.json")
        };
        let path = dir.join(&name);
        match storage::try_create_file(&path, bytes) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
            Err(err) => return Err(AppError::io("写入设置备份", &path, err)),
        }
    }
    Err(AppError::io(
        "写入设置备份",
        &dir,
        io::Error::new(ErrorKind::AlreadyExists, "无法生成唯一的设置备份文件名"),
    ))
}

fn normalized_content(candidate: &Candidate, warnings: &mut Vec<String>) -> Option<String> {
    if let Some(error) = &candidate.read_error {
        warnings.push(format!(
            "导入 {} 当前配置失败：{error}",
            candidate.target.label()
        ));
        return None;
    }
    let bytes = candidate.bytes.as_ref()?;
    let Ok(text) = std::str::from_utf8(bytes) else {
        warnings.push(format!(
            "未导入 {} 配置：文件不是有效的 UTF-8 文本。",
            candidate.target.label()
        ));
        return None;
    };
    match import::normalize_content(text) {
        Ok(content) => Some(content),
        Err(reason) => {
            warnings.push(format!(
                "未导入 {} 配置：{reason}",
                candidate.target.label()
            ));
            None
        }
    }
}

fn push_preset(presets: &mut Vec<Preset>, target: Target, content: String) {
    let base = format!("{CURRENT_PREFIX}{}", target.label());
    let name = unique_name(presets, None, &base);
    let id = unique_id(presets, target);
    presets.push(Preset {
        id,
        name,
        description: "从当前目标配置文件导入。".to_string(),
        target,
        content,
    });
}

fn read_candidates(root: &Path, settings: &Settings) -> Vec<Candidate> {
    Target::ALL
        .iter()
        .map(|target| {
            let target = *target;
            let path = match paths::resolve_target(root, settings.targets.get(target)) {
                Ok(path) => path,
                Err(err) => {
                    return Candidate {
                        target,
                        bytes: None,
                        read_error: Some(err.to_string()),
                    }
                }
            };
            match fs::read(&path) {
                Ok(bytes) => Candidate {
                    target,
                    bytes: Some(bytes),
                    read_error: None,
                },
                Err(err) if err.kind() == ErrorKind::NotFound => Candidate {
                    target,
                    bytes: None,
                    read_error: None,
                },
                Err(err) => Candidate {
                    target,
                    bytes: None,
                    read_error: Some(format!("读取目标文件失败：{err}")),
                },
            }
        })
        .collect()
}

fn unique_name(presets: &[Preset], skip: Option<usize>, base: &str) -> String {
    let taken = |candidate: &str| {
        presets
            .iter()
            .enumerate()
            .any(|(index, preset)| Some(index) != skip && preset.name == candidate)
    };
    if !taken(base) {
        return base.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base} ({suffix})");
        if !taken(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn unique_id(presets: &[Preset], target: Target) -> String {
    let stem = format!(
        "{ID_PREFIX}{}-{}",
        target.as_str(),
        storage::timestamp_millis()
    );
    let mut candidate = stem.clone();
    let mut suffix = 1;
    while presets.iter().any(|preset| preset.id == candidate) {
        candidate = format!("{stem}-{suffix}");
        suffix += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::Value;

    use super::{
        append_missing_candidates, is_current_omo_default_in, migrate_legacy_omo_path_in,
        Candidate, CURRENT_OMO_DEFAULT, LEGACY_OMO_DEFAULT,
    };
    use crate::model::{Preset, Target, SETTINGS_FILE};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let base = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("unit-work");
            fs::create_dir_all(&base).expect("create unit work dir");
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let path = base.join(format!("{name}-{}-{stamp}", std::process::id()));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_settings(root: &Path, omo: &str, schema: bool) -> String {
        let schema = if schema { r#","$schema":"keep""# } else { "" };
        let content = format!(
            r#"{{"targets":{{"opencode":"targets/opencode.json","omo":"{omo}"}},"theme":"system"{schema}}}"#
        );
        fs::write(root.join(SETTINGS_FILE), &content).expect("write settings");
        content
    }

    fn read_settings(root: &Path) -> Value {
        serde_json::from_str(&fs::read_to_string(root.join(SETTINGS_FILE)).expect("read settings"))
            .expect("parse settings")
    }

    fn backup_count(root: &Path) -> usize {
        fs::read_dir(root.join("backups")).map_or(0, std::iter::Iterator::count)
    }

    #[test]
    fn legacy_omo_default_migrates_only_when_current_default_exists() {
        let dir = TempDir::new("migrate-present");
        let home = dir.path().join("home");
        fs::create_dir_all(home.join(".omo")).expect("create omo dir");
        fs::write(home.join(".omo/omo.jsonc"), "{}").expect("write current omo");
        let original = write_settings(dir.path(), LEGACY_OMO_DEFAULT, true);
        let mut warnings = Vec::new();

        assert!(migrate_legacy_omo_path_in(
            dir.path(),
            Some(&home),
            &mut warnings
        ));
        assert!(warnings.is_empty(), "{warnings:?}");
        let settings = read_settings(dir.path());
        assert_eq!(settings["targets"]["omo"], CURRENT_OMO_DEFAULT);
        assert_eq!(settings["$schema"], "keep");
        assert_eq!(backup_count(dir.path()), 1);
        let backup = fs::read_dir(dir.path().join("backups"))
            .expect("read backups")
            .next()
            .expect("one backup")
            .expect("backup entry")
            .path();
        assert_eq!(fs::read_to_string(backup).expect("read backup"), original);
    }

    #[test]
    fn legacy_omo_default_stays_when_current_default_is_missing() {
        let dir = TempDir::new("migrate-missing");
        let home = dir.path().join("home");
        fs::create_dir_all(&home).expect("create home");
        let original = write_settings(dir.path(), LEGACY_OMO_DEFAULT, false);
        let mut warnings = Vec::new();

        assert!(!migrate_legacy_omo_path_in(
            dir.path(),
            Some(&home),
            &mut warnings
        ));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            fs::read_to_string(dir.path().join(SETTINGS_FILE)).expect("settings"),
            original
        );
        assert_eq!(backup_count(dir.path()), 0);
    }

    #[test]
    fn custom_omo_path_is_never_overwritten() {
        let dir = TempDir::new("migrate-custom");
        let home = dir.path().join("home");
        fs::create_dir_all(home.join(".omo")).expect("create omo dir");
        fs::write(home.join(".omo/omo.jsonc"), "{}").expect("write current omo");
        let original = write_settings(dir.path(), "custom/omo.json", false);
        let mut warnings = Vec::new();

        assert!(!migrate_legacy_omo_path_in(
            dir.path(),
            Some(&home),
            &mut warnings
        ));
        assert_eq!(
            fs::read_to_string(dir.path().join(SETTINGS_FILE)).expect("settings"),
            original
        );
        assert_eq!(backup_count(dir.path()), 0);
    }

    #[test]
    fn migration_is_idempotent_and_does_not_backup_twice() {
        let dir = TempDir::new("migrate-twice");
        let home = dir.path().join("home");
        fs::create_dir_all(home.join(".omo")).expect("create omo dir");
        fs::write(home.join(".omo/omo.jsonc"), "{}").expect("write current omo");
        write_settings(dir.path(), LEGACY_OMO_DEFAULT, false);
        let mut warnings = Vec::new();

        assert!(migrate_legacy_omo_path_in(
            dir.path(),
            Some(&home),
            &mut warnings
        ));
        assert!(!migrate_legacy_omo_path_in(
            dir.path(),
            Some(&home),
            &mut warnings
        ));
        assert_eq!(backup_count(dir.path()), 1);
    }

    #[test]
    fn current_omo_default_detection_resolves_equivalent_paths() {
        let dir = TempDir::new("current-omo-detect");
        let home = dir.path().join("home");
        fs::create_dir_all(&home).expect("create home");

        assert!(is_current_omo_default_in(
            dir.path(),
            "~/.omo/omo.jsonc",
            Some(&home)
        ));
        assert!(is_current_omo_default_in(
            dir.path(),
            r"~\.omo\omo.jsonc",
            Some(&home)
        ));
        assert!(!is_current_omo_default_in(
            dir.path(),
            "custom/omo.json",
            Some(&home)
        ));
        assert!(!is_current_omo_default_in(
            dir.path(),
            LEGACY_OMO_DEFAULT,
            Some(&home)
        ));
    }

    #[test]
    fn shipped_example_uses_current_omo_default() {
        let example: Value =
            serde_json::from_str(include_str!("../../switcher.settings.example.json"))
                .expect("example settings parse");
        assert_eq!(example["targets"]["omo"], CURRENT_OMO_DEFAULT);
    }

    #[test]
    fn append_missing_candidates_skips_structural_duplicates() {
        let mut presets = vec![Preset {
            id: "existing".to_string(),
            name: "Existing".to_string(),
            description: String::new(),
            target: Target::Omo,
            content: "{\"a\":1}".to_string(),
        }];
        let candidates = vec![Candidate {
            target: Target::Omo,
            bytes: Some(b"{ /* current */ \"a\": 1, }".to_vec()),
            read_error: None,
        }];
        let mut warnings = Vec::new();

        assert_eq!(
            append_missing_candidates(&mut presets, &candidates, &mut warnings),
            0
        );
        assert_eq!(presets.len(), 1);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn append_missing_candidates_adds_a_real_change() {
        let mut presets = vec![Preset {
            id: "existing".to_string(),
            name: "Existing".to_string(),
            description: String::new(),
            target: Target::Omo,
            content: "{\"a\":1}".to_string(),
        }];
        let candidates = vec![Candidate {
            target: Target::Omo,
            bytes: Some(b"{\"a\":2}".to_vec()),
            read_error: None,
        }];
        let mut warnings = Vec::new();

        assert_eq!(
            append_missing_candidates(&mut presets, &candidates, &mut warnings),
            1
        );
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[1].name, "当前 OMO");
    }
}
