// ============================================================
// 理论训练考核系统 — 数据库导入 / 导出 / 合并
// ============================================================
use crate::db::DbState;
use crate::models::{
    DatabaseInfo, ImportReplaceResult, MergeResult, SourceBankPreview, SourceDatabasePreview,
};
use chrono::Local;
use rusqlite::{backup::Backup, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

fn resolve_file_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let cleaned = trimmed
        .strip_prefix("file:///")
        .or_else(|| trimmed.strip_prefix("file://"))
        .or_else(|| trimmed.strip_prefix("file:/"))
        .unwrap_or(trimmed);
    let path_str = cleaned.trim().to_string();
    let p = Path::new(&path_str);
    let metadata = std::fs::metadata(p)
        .map_err(|e| format!("无法访问文件 '{}': {}", path_str, e))?;
    if !metadata.is_file() {
        return Err(format!("路径不是文件: '{}'", path_str));
    }
    if metadata.len() == 0 {
        return Err(format!("文件为空: '{}'", path_str));
    }
    Ok(path_str)
}

fn validate_sqlite_file(path: &str) -> Result<(PathBuf, u64), String> {
    let resolved = resolve_file_path(path)?;
    let pb = PathBuf::from(&resolved);
    let meta = std::fs::metadata(&pb).map_err(|e| e.to_string())?;
    let size = meta.len();
    let bytes = std::fs::read(&pb).map_err(|e| e.to_string())?;
    if bytes.len() < 16 || &bytes[0..16] != b"SQLite format 3\0" {
        return Err("不是有效的 SQLite 数据库文件".into());
    }
    let conn = Connection::open(&pb).map_err(|e| format!("无法打开数据库: {}", e))?;
    let has_banks: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='banks'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);
    let has_questions: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='questions'",
            [],
            |row| row.get::<_, i32>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);
    if !has_banks || !has_questions {
        return Err("数据库缺少必要的表（banks / questions）".into());
    }
    Ok((pb, size))
}

fn load_existing_bank_names(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT name FROM banks")
        .map_err(|e| e.to_string())?;
    let names = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(names)
}

fn fetch_banks_from_sql(
    conn: &Connection,
    sql: &str,
    existing_names: &HashSet<String>,
) -> Result<Vec<SourceBankPreview>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let banks = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(SourceBankPreview {
                name: name.clone(),
                question_count: row.get(1)?,
                created_at: row.get(2)?,
                name_conflict: existing_names.contains(&name),
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(banks)
}

fn count_table(conn: &Connection, table: &str) -> Result<u32, String> {
    let sql = format!("SELECT COUNT(*) FROM {}", table);
    conn.query_row(&sql, [], |row| row.get::<_, u32>(0))
        .map_err(|e| e.to_string())
}

fn unique_bank_name(conn: &Connection, base: &str) -> Result<String, String> {
    let candidate = format!("{}(导入)", base);
    if !bank_name_exists(conn, &candidate)? {
        return Ok(candidate);
    }
    for n in 2..100 {
        let name = format!("{}(导入{})", base, n);
        if !bank_name_exists(conn, &name)? {
            return Ok(name);
        }
    }
    Ok(format!("{}(导入{})", base, Uuid::new_v4().to_string().split('-').next().unwrap_or("x")))
}

fn bank_name_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM banks WHERE name = ?1",
            rusqlite::params![name],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

fn find_bank_id_by_name(conn: &Connection, name: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT id FROM banks WHERE name = ?1",
        rusqlite::params![name],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_database_info(state: tauri::State<DbState>) -> Result<DatabaseInfo, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let path = state.db_path.to_string_lossy().to_string();
    let size_bytes = std::fs::metadata(&state.db_path)
        .map_err(|e| e.to_string())?
        .len();
    let existing = load_existing_bank_names(&conn)?;
    let banks = fetch_banks_from_sql(
        &conn,
        "SELECT b.name, COUNT(q.id), b.created_at
         FROM banks b LEFT JOIN questions q ON b.id = q.bank_id
         GROUP BY b.id ORDER BY b.created_at DESC",
        &existing,
    )?;
    Ok(DatabaseInfo {
        path,
        size_bytes,
        bank_count: banks.len() as u32,
        question_count: count_table(&conn, "questions")?,
        record_count: count_table(&conn, "practice_records")?,
        banks,
    })
}

#[tauri::command]
pub fn preview_source_database(
    state: tauri::State<DbState>,
    file_path: String,
) -> Result<SourceDatabasePreview, String> {
    let (pb, size_bytes) = validate_sqlite_file(&file_path)?;
    let main_conn = state.conn.lock().map_err(|e| e.to_string())?;
    let existing = load_existing_bank_names(&main_conn)?;
    drop(main_conn);

    let src_conn = Connection::open(&pb).map_err(|e| e.to_string())?;
    let banks = fetch_banks_from_sql(
        &src_conn,
        "SELECT b.name, COUNT(q.id), b.created_at
         FROM banks b LEFT JOIN questions q ON b.id = q.bank_id
         GROUP BY b.id ORDER BY b.created_at DESC",
        &existing,
    )?;

    Ok(SourceDatabasePreview {
        file_path: pb.to_string_lossy().to_string(),
        size_bytes,
        bank_count: banks.len() as u32,
        question_count: count_table(&src_conn, "questions")?,
        record_count: count_table(&src_conn, "practice_records")?,
        banks,
    })
}

#[tauri::command]
pub fn export_database(state: tauri::State<DbState>, save_path: String) -> Result<String, String> {
    let dest = resolve_file_path(&save_path)?;
    std::fs::copy(&state.db_path, &dest).map_err(|e| format!("导出失败: {}", e))?;
    Ok(dest)
}

#[tauri::command]
pub fn import_database(
    state: tauri::State<DbState>,
    file_path: String,
) -> Result<ImportReplaceResult, String> {
    let (pb, _) = validate_sqlite_file(&file_path)?;
    let ts = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = state
        .db_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!(
            "{}.bak.{}",
            state
                .db_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("quiz_app.db"),
            ts
        ));

    std::fs::copy(&state.db_path, &backup_path)
        .map_err(|e| format!("备份当前数据库失败: {}", e))?;

    let src_conn = Connection::open(&pb).map_err(|e| e.to_string())?;
    let mut main_conn = state.conn.lock().map_err(|e| e.to_string())?;
    let backup = Backup::new(&src_conn, &mut main_conn).map_err(|e| e.to_string())?;
    backup
        .run_to_completion(100, Duration::from_millis(50), None)
        .map_err(|e| format!("导入替换失败: {}", e))?;

    Ok(ImportReplaceResult {
        backup_path: backup_path.to_string_lossy().to_string(),
    })
}

struct SrcBank {
    id: String,
    name: String,
    created_at: String,
}

struct SrcQuestion {
    id: String,
    bank_id: String,
    stem: String,
    q_type: String,
    options: String,
    answer: String,
    explanation: String,
    times_attempted: u32,
    times_correct: u32,
    last_attempted: Option<String>,
}

struct SrcRecord {
    question_id: String,
    bank_id: String,
    user_answer: String,
    is_correct: i32,
    mode: String,
    timestamp: String,
}

struct SrcProgress {
    bank_id: String,
    current_index: i32,
    selected_answer: String,
    updated_at: String,
}

fn load_src_banks(conn: &Connection) -> Result<Vec<SrcBank>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, created_at FROM src.banks ORDER BY created_at")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(SrcBank {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn load_src_questions(conn: &Connection, bank_id: &str) -> Result<Vec<SrcQuestion>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, bank_id, stem, type, options, answer, explanation,
                    times_attempted, times_correct, last_attempted
             FROM src.questions WHERE bank_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![bank_id], |row| {
            Ok(SrcQuestion {
                id: row.get(0)?,
                bank_id: row.get(1)?,
                stem: row.get(2)?,
                q_type: row.get(3)?,
                options: row.get(4)?,
                answer: row.get(5)?,
                explanation: row.get(6)?,
                times_attempted: row.get(7)?,
                times_correct: row.get(8)?,
                last_attempted: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn load_src_records(conn: &Connection, bank_id: &str) -> Result<Vec<SrcRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT question_id, bank_id, user_answer, is_correct, mode, timestamp
             FROM src.practice_records WHERE bank_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![bank_id], |row| {
            Ok(SrcRecord {
                question_id: row.get(0)?,
                bank_id: row.get(1)?,
                user_answer: row.get(2)?,
                is_correct: row.get(3)?,
                mode: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

fn load_src_progress(conn: &Connection, bank_id: &str) -> Result<Option<SrcProgress>, String> {
    conn.query_row(
        "SELECT bank_id, current_index, selected_answer, updated_at
         FROM src.practice_progress WHERE bank_id = ?1",
        rusqlite::params![bank_id],
        |row| {
            Ok(SrcProgress {
                bank_id: row.get(0)?,
                current_index: row.get(1)?,
                selected_answer: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn copy_bank_as_new(
    conn: &Connection,
    bank: &SrcBank,
    new_name: &str,
    questions: &[SrcQuestion],
    records: &[SrcRecord],
    progress: Option<&SrcProgress>,
) -> Result<(u32, HashMap<String, String>), String> {
    let new_bank_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO banks (id, name, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![new_bank_id, new_name, bank.created_at],
    )
    .map_err(|e| e.to_string())?;

    let mut id_map: HashMap<String, String> = HashMap::new();
    let mut added = 0u32;

    for q in questions {
        let new_qid = Uuid::new_v4().to_string();
        id_map.insert(q.id.clone(), new_qid.clone());
        conn.execute(
            "INSERT INTO questions (id, bank_id, stem, type, options, answer, explanation,
                                    times_attempted, times_correct, last_attempted)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                new_qid,
                new_bank_id,
                q.stem,
                q.q_type,
                q.options,
                q.answer,
                q.explanation,
                q.times_attempted,
                q.times_correct,
                q.last_attempted,
            ],
        )
        .map_err(|e| e.to_string())?;
        added += 1;
    }

    for r in records {
        if let Some(new_qid) = id_map.get(&r.question_id) {
            conn.execute(
                "INSERT INTO practice_records (question_id, bank_id, user_answer, is_correct, mode, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    new_qid,
                    new_bank_id,
                    r.user_answer,
                    r.is_correct,
                    r.mode,
                    r.timestamp,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    if let Some(p) = progress {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO practice_progress (bank_id, current_index, selected_answer, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                new_bank_id,
                p.current_index,
                p.selected_answer,
                p.updated_at,
            ],
        );
    }

    Ok((added, id_map))
}

fn merge_questions_into_existing(
    conn: &Connection,
    target_bank_id: &str,
    questions: &[SrcQuestion],
    records: &[SrcRecord],
    duplicate_strategy: &str,
) -> Result<(u32, u32, u32), String> {
    let mut added = 0u32;
    let mut skipped = 0u32;
    let mut overwritten = 0u32;
    let mut id_map: HashMap<String, String> = HashMap::new();

    for q in questions {
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM questions WHERE bank_id = ?1 AND stem = ?2",
                rusqlite::params![target_bank_id, q.stem],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some(eid) = existing_id {
            match duplicate_strategy {
                "overwrite" => {
                    conn.execute(
                        "UPDATE questions SET type=?1, options=?2, answer=?3, explanation=?4,
                         times_attempted=?5, times_correct=?6, last_attempted=?7 WHERE id=?8",
                        rusqlite::params![
                            q.q_type,
                            q.options,
                            q.answer,
                            q.explanation,
                            q.times_attempted,
                            q.times_correct,
                            q.last_attempted,
                            eid,
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    id_map.insert(q.id.clone(), eid);
                    overwritten += 1;
                }
                "append" => {
                    let new_qid = Uuid::new_v4().to_string();
                    conn.execute(
                        "INSERT INTO questions (id, bank_id, stem, type, options, answer, explanation,
                                                times_attempted, times_correct, last_attempted)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        rusqlite::params![
                            new_qid,
                            target_bank_id,
                            q.stem,
                            q.q_type,
                            q.options,
                            q.answer,
                            q.explanation,
                            q.times_attempted,
                            q.times_correct,
                            q.last_attempted,
                        ],
                    )
                    .map_err(|e| e.to_string())?;
                    id_map.insert(q.id.clone(), new_qid);
                    added += 1;
                }
                _ => {
                    id_map.insert(q.id.clone(), eid);
                    skipped += 1;
                }
            }
        } else {
            let new_qid = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO questions (id, bank_id, stem, type, options, answer, explanation,
                                        times_attempted, times_correct, last_attempted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    new_qid,
                    target_bank_id,
                    q.stem,
                    q.q_type,
                    q.options,
                    q.answer,
                    q.explanation,
                    q.times_attempted,
                    q.times_correct,
                    q.last_attempted,
                ],
            )
            .map_err(|e| e.to_string())?;
            id_map.insert(q.id.clone(), new_qid);
            added += 1;
        }
    }

    for r in records {
        if let Some(new_qid) = id_map.get(&r.question_id) {
            conn.execute(
                "INSERT INTO practice_records (question_id, bank_id, user_answer, is_correct, mode, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    new_qid,
                    target_bank_id,
                    r.user_answer,
                    r.is_correct,
                    r.mode,
                    r.timestamp,
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok((added, skipped, overwritten))
}

#[tauri::command]
pub fn merge_database(
    state: tauri::State<DbState>,
    file_path: String,
    bank_conflict_strategy: String,
    duplicate_strategy: Option<String>,
) -> Result<MergeResult, String> {
    let (pb, _) = validate_sqlite_file(&file_path)?;
    let dup_strategy = duplicate_strategy.as_deref().unwrap_or("skip");

    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute("ATTACH DATABASE ?1 AS src", rusqlite::params![pb.to_string_lossy().to_string()])
        .map_err(|e| format!("挂载源数据库失败: {}", e))?;

    let merge_out = (|| -> Result<MergeResult, String> {
        conn.execute_batch("BEGIN")
            .map_err(|e| e.to_string())?;

        let mut merge_result = MergeResult {
            banks_added: 0,
            banks_merged: 0,
            banks_skipped: 0,
            questions_added: 0,
            questions_skipped: 0,
            questions_overwritten: 0,
            errors: Vec::new(),
        };

        let src_banks = load_src_banks(&conn)?;

        for bank in src_banks {
            let questions = load_src_questions(&conn, &bank.id)?;
            let records = load_src_records(&conn, &bank.id)?;
            let progress = load_src_progress(&conn, &bank.id)?;

            let existing_id = find_bank_id_by_name(&conn, &bank.name)?;

            if existing_id.is_some() && bank_conflict_strategy == "merge_into_existing" {
                let target_id = existing_id.unwrap();
                match merge_questions_into_existing(
                    &conn,
                    &target_id,
                    &questions,
                    &records,
                    dup_strategy,
                ) {
                    Ok((a, s, o)) => {
                        merge_result.banks_merged += 1;
                        merge_result.questions_added += a;
                        merge_result.questions_skipped += s;
                        merge_result.questions_overwritten += o;
                    }
                    Err(e) => merge_result.errors.push(format!("合并题库「{}」失败: {}", bank.name, e)),
                }
            } else if existing_id.is_some() {
                let new_name = unique_bank_name(&conn, &bank.name)?;
                match copy_bank_as_new(&conn, &bank, &new_name, &questions, &records, progress.as_ref()) {
                    Ok((q_added, _)) => {
                        merge_result.banks_added += 1;
                        merge_result.questions_added += q_added;
                    }
                    Err(e) => merge_result.errors.push(format!("导入题库「{}」失败: {}", bank.name, e)),
                }
            } else {
                match copy_bank_as_new(&conn, &bank, &bank.name, &questions, &records, progress.as_ref()) {
                    Ok((q_added, _)) => {
                        merge_result.banks_added += 1;
                        merge_result.questions_added += q_added;
                    }
                    Err(e) => merge_result.errors.push(format!("导入题库「{}」失败: {}", bank.name, e)),
                }
            }
        }

        conn.execute_batch("COMMIT")
            .map_err(|e| e.to_string())?;
        Ok(merge_result)
    })();

    if merge_out.is_err() {
        let _ = conn.execute_batch("ROLLBACK");
    }
    let _ = conn.execute("DETACH DATABASE src", []);

    merge_out
}
