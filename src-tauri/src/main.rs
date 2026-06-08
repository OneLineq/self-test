// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;
mod debug_log;
mod models;

use tauri::Manager;

use db::DbState;
use std::sync::Mutex;

fn main() {
    // 在麒麟 V10 等 Linux 平台上，WebKitGTK 的 GPU 硬件加速在部分笔记本
    // 显卡驱动上会导致 BadMatch X11 错误和白屏，禁用合成模式以强制软件渲染。
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    tauri::Builder::default()
        .setup(|app| {
            // Windows: 便携模式，数据库放在 exe 同目录，拷文件夹即可迁移
            // Linux: 遵循 XDG 规范，放在 ~/.local/share/...
            #[cfg(target_os = "windows")]
            let app_dir = std::env::current_exe()
                .expect("Failed to get executable path")
                .parent()
                .expect("Failed to get executable directory")
                .to_path_buf();
            #[cfg(not(target_os = "windows"))]
            let app_dir = app
                .path_resolver()
                .app_data_dir()
                .expect("Failed to resolve app data dir");
            let conn = db::init_db(&app_dir);
            app.manage(DbState(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Bank commands
            commands::bank::create_bank,
            commands::bank::list_banks,
            commands::bank::get_bank,
            commands::bank::delete_bank,
            commands::bank::rename_bank,
            // Question commands
            commands::question::list_questions,
            commands::question::add_question,
            commands::question::update_question,
            commands::question::delete_question,
            commands::question::import_questions,
            commands::question::export_questions,
            commands::question::preview_excel,
            commands::question::get_sheet_names,
            commands::question::batch_set_type,
            commands::question::check_duplicate_stems,
            commands::question::batch_delete_questions,
            commands::question::clear_bank_questions,
            // Practice commands
            commands::practice::get_practice_questions,
            commands::practice::record_practice,
            commands::practice::get_practice_stats,
            commands::practice::get_practice_records,
            commands::practice::get_global_stats,
            commands::practice::get_practice_memory,
            commands::practice::clear_practice_memory,
            commands::practice::get_question_memory,
            commands::practice::clear_question_memory,
            commands::practice::save_practice_progress,
            commands::practice::load_practice_progress,
            commands::practice::clear_practice_progress,
            // 错题管理
            commands::practice::mark_question_wrong,
            commands::practice::remove_from_wrong,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
