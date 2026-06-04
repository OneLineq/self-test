// ============================================================
// 刷题助手 — 文件选择 Command
// ============================================================
// 使用 Tauri 内置对话框 API，调用原生 GTK 文件对话框，
// 不依赖 python3 / zenity / kdialog 等外部命令。
//
// 不设置文件过滤器，让所有文件可见（导入命令内部会校验文件类型）。

/// 打开文件选择对话框
#[tauri::command]
pub fn pick_file(window: tauri::Window, _filters: Option<Vec<String>>) -> Result<Option<String>, String> {
    let result = tauri::api::dialog::blocking::FileDialogBuilder::new()
        .set_parent(&window)
        .set_title("选择表格文件")
        .pick_file();

    match result {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

/// 打开保存文件对话框
#[tauri::command]
pub fn save_file(window: tauri::Window, default_name: String) -> Result<Option<String>, String> {
    let result = tauri::api::dialog::blocking::FileDialogBuilder::new()
        .set_parent(&window)
        .set_title("保存为")
        .set_file_name(&default_name)
        .save_file();

    match result {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}
