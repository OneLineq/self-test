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
    tauri::Builder::default()
        .setup(|app| {
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
            // File picker commands
            commands::file_picker::pick_file,
            commands::file_picker::save_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
