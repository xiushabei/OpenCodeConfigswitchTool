#![forbid(unsafe_code)]

pub mod apply;
pub mod backup;
pub mod commands;
pub mod chat;
pub mod chat_commands;
mod config_import;
pub mod engine;
pub mod error;
pub mod export;
pub mod import;
pub mod jsonc;
pub mod model;
pub mod models;
pub mod omo_patch;
pub mod paths;
pub mod restore;
pub mod service;
pub mod startup;
pub mod storage;
pub mod targets;
pub mod validate;

pub use engine::Engine;
pub use error::AppError;
pub use model::{
    ApplyResult, ExportEnvelope, Preset, PresetsFile, Settings, Snapshot, Target, TargetPaths,
    TargetState, Theme,
};

/// Discovers the project root, then starts the desktop window and command handler.
///
/// # Errors
///
/// Returns the error when the project root cannot be discovered, the webview data directory
/// cannot be created, or the Tauri runtime fails to start.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = paths::discover_root_from_env()?;
    let engine = engine::Engine::new(root.clone());
    engine.initialize_startup();

    let webview_dir = root.join(model::WEBVIEW_DIR);
    std::fs::create_dir_all(&webview_dir)?;

    tauri::Builder::default()
        .manage(engine)
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("OMO Config Switcher")
            .inner_size(1100.0, 760.0)
            .min_inner_size(860.0, 600.0)
            .data_directory(webview_dir)
            .build()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::save_preset,
            commands::delete_preset,
            commands::save_settings,
            commands::apply_preset,
            commands::export_preset,
            commands::restore_startup,
            commands::read_import_file,
            commands::normalize_import_content,
            commands::import_current_configs,
            commands::list_models,
            commands::patch_omo_model,
            chat_commands::discover_chat_databases,
            chat_commands::list_chat_sessions,
            chat_commands::read_chat_session,
            chat_commands::preview_chat_delete,
            chat_commands::delete_chat_session
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
