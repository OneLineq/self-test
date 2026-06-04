// ============================================================
// 刷题助手 — 题目管理 Commands（含 Excel 导入导出）
// ============================================================
use crate::db::DbState;
use crate::models::{DuplicateCheckResult, ImportResult, Question};
use calamine::{Reader, Xls, Xlsx};
use std::io::Cursor;

/// 标准化文件路径并校验文件是否存在
/// 处理常见问题：file:// 前缀、多余空白、编码问题等
fn resolve_file_path(raw: &str) -> Result<String, String> {
    // 去除首尾空白
    let trimmed = raw.trim();

    // 去除 file:// 或 file:/// 前缀（某些 GTK 后端的特殊行为）
    let cleaned = trimmed
        .strip_prefix("file:///")
        .or_else(|| trimmed.strip_prefix("file://"))
        .or_else(|| trimmed.strip_prefix("file:/"))
        .unwrap_or(trimmed);

    // 统一路径分隔符（保险），并确保不以空白结尾
    let path_str = cleaned.trim().to_string();

    let p = std::path::Path::new(&path_str);

    // 检查文件是否存在并获取元数据
    let metadata = std::fs::metadata(p)
        .map_err(|e| format!("无法访问文件 '{}' (原始路径: '{}'): {}", path_str, raw, e))?;

    if !metadata.is_file() {
        return Err(format!("路径不是文件: '{}' (原始路径: '{}')", path_str, raw));
    }

    let file_size = metadata.len();
    if file_size == 0 {
        return Err(format!("文件为空: '{}' (原始路径: '{}')", path_str, raw));
    }

    // 路径来自 Tauri 原生对话框，已经是正确路径，无需 canonicalize
    //（麒麟 V10 上 canonicalize + to_string_lossy 对中文路径会返回损坏数据）
    Ok(path_str)
}

/// 读取文件到内存，校验魔数，然后通过 Cursor 传给 calamine 解析
/// 绕过麒麟 V10 上 calamine 通过 FFI 路径打开文件可能出现的异常
macro_rules! open_spreadsheet {
    ($path:expr, $wb:ident, $body:block) => {{
        let _resolved = resolve_file_path(&$path)?;
        let p = std::path::Path::new(&_resolved);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        // 先把文件整个读到内存，再通过 Cursor 传给 calamine
        let file_bytes = std::fs::read(&_resolved)
            .map_err(|e| format!("读取文件失败 '{}': {}", _resolved, e))?;

        // 先检测文件魔数（不依赖扩展名）
        let first_4: [u8; 4] = if file_bytes.len() >= 4 {
            [file_bytes[0], file_bytes[1], file_bytes[2], file_bytes[3]]
        } else {
            return Err(format!("文件为空或损坏 '{}'", _resolved));
        };

        // 如果魔数不匹配标准签名，检查是否为 WPS 私有格式
        let is_ole2 = first_4 == [0xd0, 0xcf, 0x11, 0xe0];
        let is_zip  = first_4 == [0x50, 0x4b, 0x03, 0x04];
        let is_wps  = first_4 == [0xf5, 0x14, 0x00, 0x00];

        if !is_ole2 && !is_zip {
            let first_hex: String = file_bytes.iter().take(16).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ");

            let msg = if is_wps {
                format!(
                    "文件是 WPS 表格私有格式，不能直接导入。\n\
                     请在 WPS 中点击「另存为」，在「保存类型」中选择【Microsoft Excel 工作簿(.xlsx)】\n\
                     （不要使用默认的「WPS 表格」格式），保存后再导入。\n\
                     文件头: [{}]",
                    first_hex
                )
            } else {
                format!(
                    "文件格式无法识别 '{}'：文件头为 [{}]，不是标准 Excel 格式。\n\
                     请用 WPS 打开后另存为「Microsoft Excel 工作簿(.xlsx)」(非 WPS 表格格式)。",
                    _resolved, first_hex
                )
            };

            return Err(msg);
        }

        // 根据实际魔数选择解析器，而非扩展名
        // 这可以处理 WPS 将 .xlsx 名称用在非 ZIP 文件上的情况
        let cursor = Cursor::new(file_bytes);

        if is_ole2 {
            let mut $wb: Xls<Cursor<Vec<u8>>> = Xls::new(cursor)
                .map_err(|e| format!("无法解析文件 '{}': {}", _resolved, e))?;
            $body
        } else {
            let mut $wb: Xlsx<Cursor<Vec<u8>>> = Xlsx::new(cursor)
                .map_err(|e| format!("无法解析文件 '{}': {}", _resolved, e))?;
            $body
        }
    }};
}
use rust_xlsxwriter::Workbook;
use uuid::Uuid;

#[tauri::command]
pub fn list_questions(
    state: tauri::State<DbState>,
    bank_id: String,
) -> Result<Vec<Question>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, bank_id, stem, type, options, answer, explanation,
                    times_attempted, times_correct, last_attempted
             FROM questions WHERE bank_id = ?1 ORDER BY rowid",
        )
        .map_err(|e| e.to_string())?;

    let questions = stmt
        .query_map(rusqlite::params![bank_id], |row| {
            let options_str: String = row.get(4)?;
            let answer_str: String = row.get(5)?;
            Ok(Question {
                id: row.get(0)?,
                bank_id: row.get(1)?,
                stem: row.get(2)?,
                r#type: row.get(3)?,
                options: serde_json::from_str(&options_str).unwrap_or_default(),
                answer: serde_json::from_str(&answer_str).unwrap_or(serde_json::Value::String(
                    String::new(),
                )),
                explanation: row.get(6)?,
                times_attempted: row.get(7)?,
                times_correct: row.get(8)?,
                last_attempted: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(questions)
}

#[tauri::command]
pub fn add_question(
    state: tauri::State<DbState>,
    bank_id: String,
    stem: String,
    r#type: String,
    options: Vec<String>,
    answer: serde_json::Value,
    explanation: String,
) -> Result<Question, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let options_json = serde_json::to_string(&options).map_err(|e| e.to_string())?;
    let answer_json = serde_json::to_string(&answer).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO questions (id, bank_id, stem, type, options, answer, explanation)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, bank_id, stem, r#type, options_json, answer_json, explanation],
    )
    .map_err(|e| e.to_string())?;

    Ok(Question {
        id,
        bank_id,
        stem,
        r#type,
        options,
        answer,
        explanation,
        times_attempted: 0,
        times_correct: 0,
        last_attempted: None,
    })
}

#[tauri::command]
pub fn update_question(
    state: tauri::State<DbState>,
    id: String,
    stem: String,
    r#type: String,
    options: Vec<String>,
    answer: serde_json::Value,
    explanation: String,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let options_json = serde_json::to_string(&options).map_err(|e| e.to_string())?;
    let answer_json = serde_json::to_string(&answer).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE questions SET stem=?1, type=?2, options=?3, answer=?4, explanation=?5 WHERE id=?6",
        rusqlite::params![stem, r#type, options_json, answer_json, explanation, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_question(state: tauri::State<DbState>, id: String) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM questions WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM practice_records WHERE question_id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 导入 Excel 题目（智能列映射）
/// duplicate_strategy: None/"append"=全部追加, "overwrite"=覆盖, "skip"=跳过
#[tauri::command]
pub fn import_questions(
    state: tauri::State<DbState>,
    bank_id: String,
    file_path: String,
    sheet_name: String,
    stem_col: usize,
    type_col: Option<usize>,
    option_start_col: usize,
    option_count: usize,
    answer_col: usize,
    explanation_col: Option<usize>,
    force_type: Option<String>,
    duplicate_strategy: Option<String>,
) -> Result<ImportResult, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    // 根据扩展名自动选择 Xls/Xlsx 读取器
    let range = open_spreadsheet!(&file_path, workbook, {
        workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| format!("读取工作表 '{}' 失败: {}", sheet_name, e))
    })?;

    let rows = range.rows();
    if rows.len() < 2 {
        return Err("文件至少需要标题行和一行数据".into());
    }

    let mut success = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;
    let mut overwritten = 0u32;
    let mut errors: Vec<String> = Vec::new();

    // 跳过标题行（第一行）
    for (row_idx, row) in rows.enumerate().skip(1) {
        if row.is_empty() {
            continue;
        }

        // 读取题干
        let stem = match row.get(stem_col) {
            Some(cell) => cell.to_string().trim().to_string(),
            None => {
                failed += 1;
                errors.push(format!("第 {} 行：题干列为空", row_idx + 1));
                continue;
            }
        };
        if stem.is_empty() {
            failed += 1;
            errors.push(format!("第 {} 行：题干为空", row_idx + 1));
            continue;
        }

        // 读取题型（如果指定了统一题型则覆盖）
        let q_type = if let Some(ref ft) = force_type {
            if !ft.is_empty() {
                ft.clone()
            } else {
                String::new()
            }
        } else {
            match type_col {
                Some(col) => row.get(col).map(|c| c.to_string().trim().to_string()).unwrap_or_default(),
                None => String::new(),
            }
        };

        // 读取选项
        let mut options: Vec<String> = Vec::new();
        for i in 0..option_count {
            let col = option_start_col + i;
            let opt = row
                .get(col)
                .map(|c| c.to_string().trim().to_string())
                .unwrap_or_default();
            if !opt.is_empty() {
                let letter = ((b'A' + i as u8) as char).to_string();
                options.push(format!("{}. {}", letter, opt));
            }
        }

        // 读取答案
        let answer_raw = match row.get(answer_col) {
            Some(cell) => cell.to_string().trim().to_string(),
            None => {
                failed += 1;
                errors.push(format!("第 {} 行：答案列为空", row_idx + 1));
                continue;
            }
        };

        let answer: serde_json::Value = {
            let upper = answer_raw.to_uppercase();
            // 判断是否为多选（如 "ABC" 或 "A,B,C"）
            let cleaned = upper.replace(',', "").replace('，', "").replace(' ', "");
            if cleaned.len() > 1
                && cleaned.chars().all(|c| c.is_ascii_uppercase())
            {
                let arr: Vec<serde_json::Value> = cleaned
                    .chars()
                    .map(|c| serde_json::Value::String(c.to_string()))
                    .collect();
                serde_json::Value::Array(arr)
            } else if cleaned.len() == 1 && cleaned.chars().all(|c| c.is_ascii_uppercase()) {
                serde_json::Value::String(cleaned)
            } else {
                serde_json::Value::String(answer_raw.to_string())
            }
        };

        // 读取解析
        let explanation = match explanation_col {
            Some(col) => row.get(col).map(|c| c.to_string().trim().to_string()).unwrap_or_default(),
            None => String::new(),
        };

        // 生成选项 JSON
        let options_json = serde_json::to_string(&options).map_err(|e| e.to_string())?;
        let answer_json = serde_json::to_string(&answer).map_err(|e| e.to_string())?;

        // 检查重复 + 按策略处理
        let strategy = duplicate_strategy.as_deref().unwrap_or("append");

        if strategy == "overwrite" || strategy == "skip" {
            // 查询是否已存在相同题目的题目
            let existing: Option<String> = conn
                .query_row(
                    "SELECT id FROM questions WHERE bank_id=?1 AND stem=?2",
                    rusqlite::params![bank_id, stem],
                    |row| row.get(0),
                )
                .ok();

            if let Some(existing_id) = existing {
                if strategy == "skip" {
                    skipped += 1;
                    continue;
                }
                // overwrite: 更新已有题目
                match conn.execute(
                    "UPDATE questions SET type=?1, options=?2, answer=?3, explanation=?4 WHERE id=?5",
                    rusqlite::params![q_type, options_json, answer_json, explanation, existing_id],
                ) {
                    Ok(_) => overwritten += 1,
                    Err(e) => {
                        failed += 1;
                        errors.push(format!("第 {} 行：更新失败 - {}", row_idx + 1, e));
                    }
                }
                continue;
            }
        }

        // 无重复（或策略为 append）→ 插入
        let id = Uuid::new_v4().to_string();
        match conn.execute(
            "INSERT INTO questions (id, bank_id, stem, type, options, answer, explanation)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id, bank_id, stem, q_type, options_json, answer_json, explanation
            ],
        ) {
            Ok(_) => success += 1,
            Err(e) => {
                failed += 1;
                errors.push(format!("第 {} 行：插入失败 - {}", row_idx + 1, e));
            }
        }
    }

    Ok(ImportResult {
        success,
        failed,
        skipped,
        overwritten,
        errors,
    })
}

/// 导出题目为 Excel（xlsx 格式），返回文件路径
#[tauri::command]
pub fn export_questions(
    state: tauri::State<DbState>,
    bank_id: String,
    save_path: String,
) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT id, bank_id, stem, type, options, answer, explanation
             FROM questions WHERE bank_id = ?1 ORDER BY rowid",
        )
        .map_err(|e| e.to_string())?;

    let questions: Vec<Question> = stmt
        .query_map(rusqlite::params![bank_id], |row| {
            let options_str: String = row.get(4)?;
            let answer_str: String = row.get(5)?;
            Ok(Question {
                id: row.get(0)?,
                bank_id: row.get(1)?,
                stem: row.get(2)?,
                r#type: row.get(3)?,
                options: serde_json::from_str(&options_str).unwrap_or_default(),
                answer: serde_json::from_str(&answer_str).unwrap_or_default(),
                explanation: row.get(6)?,
                times_attempted: 0,
                times_correct: 0,
                last_attempted: None,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // 找出最大选项数
    let max_options = questions
        .iter()
        .map(|q| q.options.len())
        .max()
        .unwrap_or(4);

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    // 写表头
    let mut col = 0u16;
    sheet.write(0, col, "题型").map_err(|e| e.to_string())?;
    col += 1;
    sheet.write(0, col, "题干").map_err(|e| e.to_string())?;
    col += 1;
    for i in 0..max_options {
        let letter = ((b'A' + i as u8) as char).to_string();
        sheet
            .write(0, col, format!("选项{}", letter))
            .map_err(|e| e.to_string())?;
        col += 1;
    }
    sheet.write(0, col, "答案").map_err(|e| e.to_string())?;
    col += 1;
    sheet
        .write(0, col, "解析")
        .map_err(|e| e.to_string())?;

    // 写数据
    for (row_idx, q) in questions.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        let mut col = 0u16;

        sheet.write(row, col, &q.r#type).map_err(|e| e.to_string())?;
        col += 1;
        sheet.write(row, col, &q.stem).map_err(|e| e.to_string())?;
        col += 1;

        for i in 0..max_options {
            let opt = q.options.get(i).map(|s| s.as_str()).unwrap_or("");
            // 去掉 "A. " 前缀
            let opt_clean = if opt.len() > 3 && opt.as_bytes().get(1) == Some(&b'.') {
                &opt[3..]
            } else {
                opt
            };
            sheet
                .write(row, col, opt_clean)
                .map_err(|e| e.to_string())?;
            col += 1;
        }

        let answer_str = match &q.answer {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(","),
            _ => String::new(),
        };
        sheet
            .write(row, col, &answer_str)
            .map_err(|e| e.to_string())?;
        col += 1;
        sheet
            .write(row, col, &q.explanation)
            .map_err(|e| e.to_string())?;
    }

    let data = workbook.save_to_buffer().map_err(|e| e.to_string())?;
    std::fs::write(&save_path, data).map_err(|e| e.to_string())?;

    Ok(save_path)
}

/// 获取 Excel 文件中所有工作表的名称
#[tauri::command]
pub fn get_sheet_names(file_path: String) -> Result<Vec<String>, String> {
    open_spreadsheet!(&file_path, workbook, {
        let names = workbook.sheet_names().to_vec();
        if names.is_empty() {
            return Err("文件中没有工作表".into());
        }
        Ok(names)
    })
}

/// 预览 Excel 文件的列标题和示例数据（用于列映射）
fn preview_excel_sync(file_path: &str, sheet_name: &str) -> Result<serde_json::Value, String> {
    let range = open_spreadsheet!(file_path, workbook, {
        workbook
            .worksheet_range(sheet_name)
            .map_err(|e| format!("读取工作表 '{}' 失败: {}", sheet_name, e))
    })?;

    let rows: Vec<Vec<String>> = range
        .rows()
        .map(|row| row.iter().map(|c| c.to_string().trim().to_string()).collect())
        .collect();

    let headers: Vec<String> = rows.first().cloned().unwrap_or_default();
    let preview_rows: Vec<Vec<String>> = rows.iter().skip(1).take(3).cloned().collect();

    Ok(serde_json::json!({
        "headers": headers,
        "preview": preview_rows,
        "total_rows": rows.len().saturating_sub(1),
    }))
}

#[tauri::command]
pub async fn preview_excel(
    file_path: String,
    sheet_name: String,
) -> Result<serde_json::Value, String> {
    let f = file_path.clone();
    let s = sheet_name.clone();

    let result = tauri::async_runtime::spawn_blocking(move || preview_excel_sync(&f, &s))
        .await
        .map_err(|e| format!("预览任务失败: {}", e))?;

    result
}

/// 检测 Excel 文件中与题库已有题目重复的题干
#[tauri::command]
pub fn check_duplicate_stems(
    state: tauri::State<DbState>,
    bank_id: String,
    file_path: String,
    sheet_name: String,
    stem_col: usize,
) -> Result<DuplicateCheckResult, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    // 读取 Excel
    let range = open_spreadsheet!(&file_path, workbook, {
        workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| format!("读取工作表 '{}' 失败: {}", sheet_name, e))
    })?;

    let rows = range.rows();
    if rows.len() < 2 {
        return Err("文件至少需要标题行和一行数据".into());
    }

    let mut duplicate_stems: Vec<String> = Vec::new();

    for row in rows.skip(1) {
        if row.is_empty() {
            continue;
        }
        let stem = match row.get(stem_col) {
            Some(cell) => cell.to_string().trim().to_string(),
            None => continue,
        };
        if stem.is_empty() {
            continue;
        }

        // 查询是否已存在
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM questions WHERE bank_id=?1 AND stem=?2",
                rusqlite::params![bank_id, stem],
                |row| row.get::<_, i32>(0),
            )
            .unwrap_or(0)
            > 0;

        if exists {
            duplicate_stems.push(stem);
        }
    }

    let count = duplicate_stems.len() as u32;
    // 只返回前 20 个重复题干（避免数据过大）
    if duplicate_stems.len() > 20 {
        duplicate_stems.truncate(20);
        duplicate_stems.push(format!("……以及另外 {} 道", count - 20));
    }

    Ok(DuplicateCheckResult {
        duplicate_count: count,
        duplicate_stems,
    })
}

/// 批量设置题目题型
#[tauri::command]
pub fn batch_set_type(
    state: tauri::State<DbState>,
    question_ids: Vec<String>,
    q_type: String,
) -> Result<u32, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for id in &question_ids {
        match conn.execute(
            "UPDATE questions SET type=?1 WHERE id=?2",
            rusqlite::params![q_type, id],
        ) {
            Ok(rows) => count += rows as u32,
            Err(e) => return Err(format!("更新题目 {} 失败: {}", id, e)),
        }
    }
    Ok(count)
}

/// 批量删除题目
#[tauri::command]
pub fn batch_delete_questions(
    state: tauri::State<DbState>,
    question_ids: Vec<String>,
) -> Result<u32, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut count = 0u32;
    for id in &question_ids {
        match conn.execute("DELETE FROM questions WHERE id=?1", rusqlite::params![id]) {
            Ok(n) => {
                count += n as u32;
                let _ = conn.execute(
                    "DELETE FROM practice_records WHERE question_id=?1",
                    rusqlite::params![id],
                );
            }
            Err(e) => return Err(format!("删除题目 {} 失败: {}", id, e)),
        }
    }
    Ok(count)
}

/// 清空题库中所有题目
#[tauri::command]
pub fn clear_bank_questions(
    state: tauri::State<DbState>,
    bank_id: String,
) -> Result<u32, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    // 先清理练习记录（否则外键约束会阻止删除题目）
    let _ = conn.execute(
        "DELETE FROM practice_records WHERE bank_id=?1",
        rusqlite::params![bank_id],
    );
    let count = conn
        .execute("DELETE FROM questions WHERE bank_id=?1", rusqlite::params![bank_id])
        .map_err(|e| format!("清空题库失败: {}", e))? as u32;
    Ok(count)
}
