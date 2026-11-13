use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::types::DiscoveryResult;

const DEFAULT_RELATIVE: &str = ".local/share/opencode/opencode.db";
const MAX_RESULTS: usize = 20;
const MAX_ENTRIES: usize = 250_000;
const MAX_DEPTH: usize = 12;
const CLI_TIMEOUT: Duration = Duration::from_secs(8);
const SKIP_DIRS: &[&str] = &[
    "windows",
    "$recycle.bin",
    "system volume information",
    "recovery",
    "perflogs",
    "node_modules",
    ".git",
    "target",
];

static FULLSCAN: OnceLock<Mutex<Option<ScanCache>>> = OnceLock::new();

#[derive(Clone)]
struct ScanCache {
    paths: Vec<PathBuf>,
    warnings: Vec<String>,
    searched: bool,
}

/// Locates existing `opencode.db` files. The default or environment path is used first; the
/// single cached all-drive scan only runs for users whose canonical paths do not exist.
#[must_use]
pub fn discover() -> DiscoveryResult {
    let mut warnings = Vec::new();
    let mut found = Vec::new();
    if let Ok(value) = std::env::var("OPENCODE_DB") {
        let value = value.trim().trim_matches('"');
        if !value.is_empty() {
            add_existing(Path::new(value), &mut found, Some(&mut warnings));
        }
    }
    if found.is_empty() {
        if let Some(path) = cli_candidate() {
            add_existing(&path, &mut found, None);
        }
    }
    if found.is_empty() {
        if let Some(home) = home_dir() {
            add_existing(&home.join(DEFAULT_RELATIVE), &mut found, None);
        }
    }
    if !found.is_empty() {
        return DiscoveryResult {
            paths: found
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            searched_all_drives: false,
            warnings,
        };
    }

    let scan = cached_scan();
    warnings.extend(scan.warnings);
    DiscoveryResult {
        paths: scan
            .paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        searched_all_drives: scan.searched,
        warnings,
    }
}

fn add_existing(path: &Path, found: &mut Vec<PathBuf>, warnings: Option<&mut Vec<String>>) {
    if path.is_file() {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if !found.contains(&canonical) {
            found.push(canonical);
        }
        return;
    }
    if let Some(warnings) = warnings {
        warnings.push(format!(
            "OPENCODE_DB 指向的数据库不存在：{}",
            path.display()
        ));
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}
fn cli_candidate() -> Option<PathBuf> {
    let output = run_cli(CLI_TIMEOUT)?;
    let text = String::from_utf8_lossy(&output);
    for line in text.lines() {
        let candidate = line.trim().trim_matches('"');
        if !candidate.is_empty() && Path::new(candidate).is_file() {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

fn run_cli(timeout: Duration) -> Option<Vec<u8>> {
    let mut command = Command::new("cmd");
    command
        .args(["/C", "opencode", "db", "path"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        use std::io::Read as _;
        let mut data = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => data.extend_from_slice(&buffer[..read]),
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
            if data.len() > 64 * 1024 {
                break;
            }
        }
        data
    });
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait().ok()? {
            Some(status) if status.success() => break,
            Some(_) => return None,
            None => {}
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    reader.join().ok()
}

fn cached_scan() -> ScanCache {
    let cell = FULLSCAN.get_or_init(|| Mutex::new(None));
    let Ok(mut guard) = cell.lock() else {
        return scan_drives();
    };
    if let Some(cache) = guard.as_ref() {
        return cache.clone();
    }
    let cache = scan_drives();
    *guard = Some(cache.clone());
    cache
}

fn scan_drives() -> ScanCache {
    let roots = drive_roots();
    let mut paths = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    let mut entries = 0_usize;
    let mut inaccessible = 0_usize;
    let mut capped = false;

    'roots: for root in &roots {
        let mut stack = vec![(root.clone(), 0_usize)];
        while let Some((directory, depth)) = stack.pop() {
            let Ok(read) = fs::read_dir(&directory) else {
                inaccessible += 1;
                continue;
            };
            for entry in read {
                if entries >= MAX_ENTRIES || paths.len() >= MAX_RESULTS {
                    capped = true;
                    break 'roots;
                }
                entries += 1;
                let Ok(entry) = entry else {
                    inaccessible += 1;
                    continue;
                };
                let Ok(file_type) = entry.file_type() else {
                    inaccessible += 1;
                    continue;
                };
                if file_type.is_symlink() || is_reparse(&entry) {
                    continue;
                }
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if file_type.is_dir() {
                    let lower = name.to_ascii_lowercase();
                    if depth < MAX_DEPTH && !SKIP_DIRS.contains(&lower.as_str()) {
                        stack.push((entry.path(), depth + 1));
                    }
                } else if file_type.is_file() && name.eq_ignore_ascii_case("opencode.db") {
                    let path = entry.path();
                    let canonical = path.canonicalize().unwrap_or(path);
                    if seen.insert(canonical.clone()) {
                        paths.push(canonical);
                    }
                }
            }
        }
    }

    if inaccessible > 0 {
        warnings.push(format!(
            "扫描时有 {inaccessible} 个目录项无法访问，已跳过。"
        ));
    }
    if capped {
        warnings.push("扫描达到上限，可能未返回全部 opencode.db。".to_string());
    }
    if paths.is_empty() {
        warnings.push("未找到 opencode.db，请手动选择数据库路径。".to_string());
    }
    ScanCache {
        paths,
        warnings,
        searched: !roots.is_empty(),
    }
}

fn drive_roots() -> Vec<PathBuf> {
    (b'A'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
        .filter(|root| root.is_dir())
        .collect()
}

fn is_reparse(entry: &fs::DirEntry) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const REPARSE_POINT: u32 = 0x0400;
        if let Ok(metadata) = entry.metadata() {
            return metadata.file_attributes() & REPARSE_POINT != 0;
        }
    }
    #[cfg(not(windows))]
    let _ = entry;
    false
}
