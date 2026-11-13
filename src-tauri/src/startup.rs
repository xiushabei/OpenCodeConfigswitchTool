use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::config_import::{self, Candidate};
use crate::model::{Settings, Target};
use crate::{paths, storage};

/// One-shot launch state: the immutable restore baseline plus warnings seen during startup.
pub(crate) struct StartupCapture {
    baseline: Option<StartupBaseline>,
    warnings: Vec<String>,
}

impl StartupCapture {
    #[must_use]
    pub(crate) fn baseline(&self) -> Option<&StartupBaseline> {
        self.baseline.as_ref()
    }

    #[must_use]
    pub(crate) fn warnings(&self) -> &[String] {
        &self.warnings
    }

    #[must_use]
    pub(crate) fn can_restore(&self) -> bool {
        self.baseline
            .as_ref()
            .is_some_and(StartupBaseline::is_complete)
    }

    #[must_use]
    pub(crate) fn unavailable_reason(&self) -> String {
        self.warnings
            .first()
            .cloned()
            .unwrap_or_else(|| "启动时没有可用的目标读取结果。".to_string())
    }
}

/// Exact launch snapshot of both configured targets, keyed by their resolved absolute paths.
pub(crate) struct StartupBaseline {
    pub(crate) files: Vec<BaselineFile>,
}

impl StartupBaseline {
    #[must_use]
    pub(crate) fn files(&self) -> &[BaselineFile] {
        &self.files
    }

    #[must_use]
    fn is_complete(&self) -> bool {
        self.files.len() == Target::ALL.len()
            && self
                .files
                .iter()
                .all(|file| !matches!(&file.state, BaselineState::Unreadable(_)))
    }
}

pub(crate) struct BaselineFile {
    pub(crate) target: Target,
    pub(crate) path: PathBuf,
    pub(crate) state: BaselineState,
}

pub(crate) enum BaselineState {
    Bytes(Vec<u8>),
    Missing,
    Unreadable(String),
}

/// Captures both targets before any owned-storage write, then runs the one-time first import.
pub(crate) fn capture(root: &Path) -> StartupCapture {
    let mut warnings = Vec::new();
    let migrated = config_import::migrate_legacy_omo_path(root, &mut warnings);
    let settings = match storage::read_settings(root) {
        Ok(settings) => settings,
        Err(err) => {
            warnings.push(format!("启动时无法读取设置，无法恢复启动状态：{err}"));
            return StartupCapture {
                baseline: None,
                warnings,
            };
        }
    };
    let ensure_current_omo =
        migrated || config_import::is_current_omo_default(root, &settings.targets.omo);

    let files = read_baseline(root, &settings, &mut warnings);
    import_on_first_start(
        StartupImport {
            root,
            files: &files,
            ensure_current_omo,
        },
        &mut warnings,
    );
    StartupCapture {
        baseline: Some(StartupBaseline { files }),
        warnings,
    }
}

fn read_baseline(
    root: &Path,
    settings: &Settings,
    warnings: &mut Vec<String>,
) -> Vec<BaselineFile> {
    let mut files = Vec::with_capacity(Target::ALL.len());
    for target in Target::ALL {
        let raw = settings.targets.get(target);
        let path = match paths::resolve_target(root, raw) {
            Ok(path) => path,
            Err(err) => {
                warnings.push(format!(
                    "启动时无法解析 {} 目标路径，无法恢复启动状态：{err}",
                    target.label()
                ));
                files.push(BaselineFile {
                    target,
                    path: PathBuf::from(raw),
                    state: BaselineState::Unreadable(err.to_string()),
                });
                continue;
            }
        };
        let state = match fs::read(&path) {
            Ok(bytes) => BaselineState::Bytes(bytes),
            Err(err) if err.kind() == ErrorKind::NotFound => BaselineState::Missing,
            Err(err) => {
                warnings.push(format!(
                    "启动时读取 {} 目标失败，无法恢复启动状态：{err}",
                    target.label()
                ));
                BaselineState::Unreadable(err.to_string())
            }
        };
        files.push(BaselineFile {
            target,
            path,
            state,
        });
    }
    files
}

#[derive(Clone, Copy)]
struct StartupImport<'a> {
    root: &'a Path,
    files: &'a [BaselineFile],
    ensure_current_omo: bool,
}

fn import_on_first_start(import: StartupImport<'_>, warnings: &mut Vec<String>) {
    let StartupImport {
        root,
        files,
        ensure_current_omo,
    } = import;
    let store = match storage::read_presets_store(root) {
        Ok(store) => store,
        Err(err) => {
            warnings.push(format!("启动时无法读取预设存储，未导入当前配置：{err}"));
            return;
        }
    };
    let mut presets = store.presets;
    let renamed = config_import::rename_legacy_presets(&mut presets);
    let mut imported = 0;
    if !store.initialized {
        let candidates: Vec<Candidate> = files.iter().map(candidate_from_baseline).collect();
        imported = config_import::append_candidates(&mut presets, &candidates, warnings);
    } else if ensure_current_omo {
        let candidates: Vec<Candidate> = files
            .iter()
            .filter(|file| file.target == Target::Omo)
            .map(candidate_from_baseline)
            .collect();
        imported = config_import::append_missing_candidates(&mut presets, &candidates, warnings);
    }
    if renamed || imported > 0 || !store.initialized {
        if let Err(err) = storage::write_presets(root, &presets, true) {
            warnings.push(format!("启动时无法更新预设存储：{err}"));
        }
    }
}

fn candidate_from_baseline(file: &BaselineFile) -> Candidate {
    match &file.state {
        BaselineState::Bytes(bytes) => Candidate {
            target: file.target,
            bytes: Some(bytes.clone()),
            read_error: None,
        },
        BaselineState::Missing | BaselineState::Unreadable(_) => Candidate {
            target: file.target,
            bytes: None,
            read_error: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{import_on_first_start, BaselineFile, BaselineState, StartupImport};
    use crate::model::{Preset, PresetsFile, Target, PRESET_FORMAT_VERSION};
    use crate::storage;

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

    fn write_store(root: &Path, presets: Vec<Preset>, initialized: bool) {
        fs::create_dir_all(root.join("data")).expect("create data dir");
        let file = PresetsFile {
            format_version: PRESET_FORMAT_VERSION,
            presets,
            first_import_completed: Some(initialized),
        };
        fs::write(
            root.join("data/presets.json"),
            serde_json::to_string(&file).expect("serialize store"),
        )
        .expect("write store");
    }

    fn baseline(target: Target, content: &str) -> BaselineFile {
        BaselineFile {
            target,
            path: PathBuf::from("targets/config.json"),
            state: BaselineState::Bytes(content.as_bytes().to_vec()),
        }
    }

    fn preset_count(root: &Path) -> usize {
        storage::read_presets(root).expect("read presets").len()
    }

    #[test]
    fn current_default_omo_is_imported_once_when_the_marker_is_set() {
        let dir = TempDir::new("startup-current-omo");
        write_store(dir.path(), Vec::new(), true);
        let files = vec![baseline(Target::Omo, "{\n  // current\n  \"a\": 1,\n}\n")];
        let mut warnings = Vec::new();

        import_on_first_start(
            StartupImport {
                root: dir.path(),
                files: &files,
                ensure_current_omo: true,
            },
            &mut warnings,
        );
        assert!(warnings.is_empty(), "{warnings:?}");
        let presets = storage::read_presets(dir.path()).expect("read presets");
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "当前 OMO");
        assert_eq!(presets[0].content, "{\n  \"a\": 1\n}\n");

        import_on_first_start(
            StartupImport {
                root: dir.path(),
                files: &files,
                ensure_current_omo: true,
            },
            &mut warnings,
        );
        assert_eq!(preset_count(dir.path()), 1);
    }

    #[test]
    fn current_default_omo_import_skips_an_equivalent_preset() {
        let dir = TempDir::new("startup-current-omo-dedupe");
        write_store(
            dir.path(),
            vec![Preset {
                id: "current-omo".to_string(),
                name: "我的 OMO".to_string(),
                description: String::new(),
                target: Target::Omo,
                content: "{\"a\":1}".to_string(),
            }],
            true,
        );
        let files = vec![baseline(Target::Omo, "{ /* current */ \"a\": 1 }")];
        let mut warnings = Vec::new();

        import_on_first_start(
            StartupImport {
                root: dir.path(),
                files: &files,
                ensure_current_omo: true,
            },
            &mut warnings,
        );
        assert_eq!(preset_count(dir.path()), 1);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn custom_omo_path_is_not_imported_when_flag_is_false() {
        let dir = TempDir::new("startup-custom-omo");
        write_store(dir.path(), Vec::new(), true);
        let files = vec![baseline(Target::Omo, "{\"custom\":true}")];
        let mut warnings = Vec::new();

        import_on_first_start(
            StartupImport {
                root: dir.path(),
                files: &files,
                ensure_current_omo: false,
            },
            &mut warnings,
        );
        assert_eq!(preset_count(dir.path()), 0);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn uninitialized_storage_imports_both_targets_once() {
        let dir = TempDir::new("startup-first-both");
        let files = vec![
            baseline(Target::Opencode, "{\"o\":1}"),
            baseline(Target::Omo, "{\"m\":2}"),
        ];
        let mut warnings = Vec::new();

        import_on_first_start(
            StartupImport {
                root: dir.path(),
                files: &files,
                ensure_current_omo: true,
            },
            &mut warnings,
        );
        assert_eq!(preset_count(dir.path()), 2);
        assert!(
            storage::read_presets_store(dir.path())
                .expect("read store")
                .initialized
        );

        import_on_first_start(
            StartupImport {
                root: dir.path(),
                files: &files,
                ensure_current_omo: false,
            },
            &mut warnings,
        );
        assert_eq!(preset_count(dir.path()), 2);
    }
}
