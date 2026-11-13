use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::error::AppError;
use crate::model::{Preset, Settings, Target};
use crate::{import, paths, storage};

const CURRENT_PREFIX: &str = "当前 ";
const LEGACY_PREFIX: &str = "启动导入 ";
const ID_PREFIX: &str = "import-";

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
        if let Some(error) = &candidate.read_error {
            warnings.push(format!(
                "导入 {} 当前配置失败：{error}",
                candidate.target.label()
            ));
            continue;
        }
        let Some(bytes) = &candidate.bytes else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            warnings.push(format!(
                "未导入 {} 配置：文件不是有效的 UTF-8 文本。",
                candidate.target.label()
            ));
            continue;
        };
        let content = match import::normalize_content(text) {
            Ok(content) => content,
            Err(reason) => {
                warnings.push(format!(
                    "未导入 {} 配置：{reason}",
                    candidate.target.label()
                ));
                continue;
            }
        };
        let base = format!("{CURRENT_PREFIX}{}", candidate.target.label());
        let name = unique_name(presets, None, &base);
        let id = unique_id(presets, candidate.target);
        presets.push(Preset {
            id,
            name,
            description: "从当前目标配置文件导入。".to_string(),
            target: candidate.target,
            content,
        });
        added += 1;
    }
    added
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
