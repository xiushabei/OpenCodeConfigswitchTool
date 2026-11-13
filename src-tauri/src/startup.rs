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

    let files = read_baseline(root, &settings, &mut warnings);
    import_on_first_start(root, &files, &mut warnings);
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

fn import_on_first_start(root: &Path, files: &[BaselineFile], warnings: &mut Vec<String>) {
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
