// ============================================================
// 刷题助手 — 文件选择 Command
// ============================================================
// 麒麟 V10 上 GTK 对话框与 Tauri 主循环冲突导致卡死，
// 改用外部进程方式打开文件对话框。
// 优先级: python3(tkinter) → zenity → kdialog

use std::process::Command;

fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// 打开文件选择对话框
#[tauri::command]
pub fn pick_file(filters: Option<Vec<String>>) -> Result<Option<String>, String> {
    if is_linux() {
        linux_pick_file()
    } else {
        Ok(windows_pick_file(filters))
    }
}

/// 打开保存文件对话框
#[tauri::command]
pub fn save_file(default_name: String) -> Result<Option<String>, String> {
    if is_linux() {
        linux_save_file(&default_name)
    } else {
        Ok(windows_save_file(default_name))
    }
}

// ==================== Linux ====================

/// 过滤掉 stderr 中的桌面环境噪音（AT-SPI、dbus 等），
/// 只保留真正的错误内容。
fn filter_stderr_noise(stderr: &str) -> String {
    stderr
        .lines()
        .filter(|line| {
            // 跳过 AT-SPI / dbind 等 dbus 噪音
            !line.contains("AT-SPI:")
                && !line.contains("dbind-WARNING")
                && !line.contains("org.a11y.Bus")
                && !line.contains("org.freedesktop.DBus.Error")
                && !line.contains("ServiceUnknown")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("执行 '{}' 失败: {}", cmd, e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        // 过滤已知的桌面环境噪音后，检查是否还有真正的错误
        let real_err = filter_stderr_noise(&stderr);
        if stdout.is_empty() && real_err.is_empty() {
            // stderr 全是桌面噪音 + stdout 空 = 用户取消了对话框
            Ok(String::new())
        } else {
            // 有真正的错误信息
            let msg = if real_err.is_empty() { &stderr } else { &real_err };
            Err(format!("'{}' 返回错误: {}", cmd, msg))
        }
    } else if stdout.is_empty() {
        // exit code 0 但 stdout 空 → tkinter 被用户关闭
        Ok(String::new())
    } else {
        Ok(stdout)
    }
}

/// 用 python3 tkinter 打开文件对话框
fn python_tkinter_file(title: &str, save: bool, default_name: Option<&str>) -> Result<String, String> {
    let default = default_name.unwrap_or("");

    let script = format!(
        r#"import tkinter as tk
from tkinter import filedialog as fd
r = tk.Tk()
r.withdraw()
r.attributes('-topmost', True)
if {save}:
    p = fd.asksaveasfilename(title="{title}", initialfile="{default}", defaultextension=".xlsx",
                             filetypes=[("Excel 文件", "*.xlsx *.xls *.xlsb *.ods *.et"), ("所有文件", "*.*")])
else:
    p = fd.askopenfilename(title="{title}",
                           filetypes=[("Excel 文件", "*.xlsx *.xls *.xlsb *.ods *.et"), ("所有文件", "*.*")])
if p:
    print(p)
r.destroy()
"#
    );
    run_cmd("python3", &["-c", &script])
}

/// 用 zenity 打开文件对话框
fn python_zenity(title: &str, save: bool, default_name: Option<&str>) -> Result<String, String> {
    let mut args = vec!["--file-selection", "--title", title];
    if save {
        args.push("--save");
        if let Some(name) = default_name {
            args.push("--filename");
            args.push(name);
        }
    }
    run_cmd("zenity", &args)
}

fn linux_pick_file() -> Result<Option<String>, String> {
    // 优先用 python3 tkinter（最兼容 Ubuntu/Kylin 桌面）
    let r = python_tkinter_file("选择表格文件", false, None)
        .or_else(|_| python_zenity("选择表格文件", false, None));

    match r {
        Ok(p) if p.is_empty() => Ok(None),  // 用户取消
        Ok(p) => Ok(Some(p)),
        Err(msg) => Err(format!("文件对话框均不可用: {}", msg)),
    }
}

fn linux_save_file(default_name: &str) -> Result<Option<String>, String> {
    let r = python_tkinter_file("保存为", true, Some(default_name))
        .or_else(|_| python_zenity("保存为", true, Some(default_name)));

    match r {
        Ok(p) if p.is_empty() => Ok(None),
        Ok(p) => Ok(Some(p)),
        Err(msg) => Err(format!("保存对话框均不可用: {}", msg)),
    }
}

// ==================== Windows ====================

fn windows_pick_file(filters: Option<Vec<String>>) -> Option<String> {
    let mut dialog = tauri::api::dialog::blocking::FileDialogBuilder::new();
    if let Some(exts) = filters {
        let refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
        dialog = dialog.add_filter("支持的表格文件", &refs);
    } else {
        dialog = dialog.add_filter("支持的表格文件", &["xlsx", "xls", "xlsb", "ods", "et"]);
    }
    dialog.pick_file().map(|p| p.to_string_lossy().to_string())
}

fn windows_save_file(default_name: String) -> Option<String> {
    tauri::api::dialog::blocking::FileDialogBuilder::new()
        .set_file_name(&default_name)
        .add_filter("Excel 文件", &["xlsx"])
        .save_file()
        .map(|p| p.to_string_lossy().to_string())
}
