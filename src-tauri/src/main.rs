#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = switcher_core::run() {
        eprintln!("启动失败：{err}");
        std::process::exit(1);
    }
}
