use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::error::{ChatError, ChatResult};

const TIMEOUT: Duration = Duration::from_secs(8);
const MAX_OUTPUT: usize = 1024 * 1024;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Reports whether an `OpenCode` process is running. Any failure is an error so callers block.
///
/// # Errors
///
/// Returns [`ChatError::Blocked`] when `tasklist` cannot run, times out, exits unsuccessfully, or
/// prints output that cannot be parsed. Callers must treat this as "running" and refuse deletion.
pub fn opencode_running() -> ChatResult<bool> {
    #[cfg(windows)]
    {
        run_tasklist()
    }
    #[cfg(not(windows))]
    {
        Err(ChatError::Blocked {
            reason: "当前平台无法可靠检查 OpenCode 进程，已阻止删除。".to_string(),
        })
    }
}

#[cfg(windows)]
fn run_tasklist() -> ChatResult<bool> {
    let mut command = Command::new("tasklist");
    command
        .args(["/FO", "CSV", "/NH"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|err| ChatError::Blocked {
        reason: format!("无法运行 tasklist 检查 OpenCode 进程：{err}"),
    })?;
    let mut stdout = child.stdout.take().ok_or_else(|| ChatError::Blocked {
        reason: "无法读取 tasklist 输出。".to_string(),
    })?;
    let reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        let mut buffer = [0_u8; 8192];
        let mut overflowed = false;
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let room = MAX_OUTPUT.saturating_sub(data.len());
                    if read > room {
                        overflowed = true;
                    }
                    data.extend_from_slice(&buffer[..read.min(room)]);
                }
                Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        (data, overflowed)
    });

    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ChatError::Blocked {
                    reason: format!("等待 tasklist 失败：{err}"),
                });
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ChatError::Blocked {
                reason: "tasklist 超时，无法确认 OpenCode 是否正在运行。".to_string(),
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (output, overflowed) = reader.join().map_err(|_| ChatError::Blocked {
        reason: "tasklist 输出读取线程异常结束。".to_string(),
    })?;
    if overflowed {
        return Err(ChatError::Blocked {
            reason: "tasklist 输出超过上限，无法确认 OpenCode 是否正在运行。".to_string(),
        });
    }
    if !status.success() {
        return Err(ChatError::Blocked {
            reason: format!("tasklist 以退出码 {:?} 结束。", status.code()),
        });
    }
    parse_names(&output)
}

#[cfg(windows)]
fn parse_names(output: &[u8]) -> ChatResult<bool> {
    let text = String::from_utf8_lossy(output);
    let mut parsed_any = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(name) = first_csv_field(line) else {
            return Err(ChatError::Blocked {
                reason: "无法解析 tasklist 输出，已按运行中处理。".to_string(),
            });
        };
        parsed_any = true;
        if is_opencode(name) {
            return Ok(true);
        }
    }
    if !parsed_any {
        return Err(ChatError::Blocked {
            reason: "tasklist 未返回可解析的进程列表。".to_string(),
        });
    }
    Ok(false)
}

#[cfg(windows)]
fn first_csv_field(line: &str) -> Option<&str> {
    let mut chars = line.char_indices();
    let (_, first) = chars.next()?;
    if first != '"' {
        return None;
    }
    let (end, _) = chars.find(|(_, character)| *character == '"')?;
    Some(&line[1..end])
}

#[cfg(windows)]
fn is_opencode(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let is_exe = std::path::Path::new(&lower)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    lower.starts_with("opencode") && (is_exe || lower.contains("desktop"))
}

#[cfg(all(test, windows))]
mod tests {
    use super::parse_names;

    #[test]
    fn detects_cli_and_desktop_names() {
        let output = b"\"opencode.exe\",\"1\",\"Console\",\"1\",\"1 K\"\n";
        assert!(parse_names(output).expect("parse tasklist"));
        let output = b"\"OpenCode Desktop.exe\",\"2\",\"Console\",\"1\",\"1 K\"\n";
        assert!(parse_names(output).expect("parse tasklist"));
    }

    #[test]
    fn ignores_unrelated_processes() {
        let output = b"\"Code.exe\",\"1\",\"Console\",\"1\",\"1 K\"\n";
        assert!(!parse_names(output).expect("parse tasklist"));
    }

    #[test]
    fn unparsable_output_fails_closed() {
        assert!(parse_names(b"INFO: no tasks\n").is_err());
        assert!(parse_names(b"").is_err());
    }
}
