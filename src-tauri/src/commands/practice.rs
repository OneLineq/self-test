// ============================================================
// 刷题助手 — 练习模式 Commands
// ============================================================
use crate::db::DbState;
use crate::models::{PracticeRecord, Question};
use chrono::Local;

/// 获取练习题目
/// mode: sequential (顺序), random (随机), wrong (错题), exam (模拟考试)
/// question_types: 可选，按题型过滤（如 ["single","multiple"]）
/// limit: 可选，限定题目数量（combined 模式用）
#[tauri::command]
pub fn get_practice_questions(
    state: tauri::State<DbState>,
    bank_id: String,
    mode: String,
    question_types: Option<Vec<String>>,
    limit: Option<u32>,
) -> Result<Vec<Question>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    // 构建 WHERE + ORDER BY 子句
    let (order_clause, filter_suffix) = match mode.as_str() {
        "wrong" => ("ORDER BY q.rowid", " AND pr.is_correct = 0"),
        "random" => ("ORDER BY RANDOM()", ""),
        _ => ("ORDER BY rowid", ""), // sequential / exam
    };

    // 题型过滤子句
    let type_filter = if let Some(ref types) = question_types {
        if !types.is_empty() {
            let placeholders: Vec<String> = (0..types.len()).map(|i| format!("?{}", i + 2)).collect();
            format!(" AND type IN ({})", placeholders.join(","))
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // LIMIT 子句
    let limit_clause = if let Some(_l) = limit {
        " LIMIT ?100"
    } else {
        ""
    };

    if mode.as_str() == "wrong" {
        let query = format!(
            "SELECT DISTINCT q.id, q.bank_id, q.stem, q.type, q.options, q.answer,
                    q.explanation, q.times_attempted, q.times_correct, q.last_attempted
             FROM questions q
             INNER JOIN practice_records pr ON q.id = pr.question_id
             WHERE q.bank_id = ?1{}{}{}",
            filter_suffix, type_filter, limit_clause
        );

        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let questions = bind_and_query(&mut stmt, &bank_id, &question_types, &limit)?;
        Ok(questions)
    } else {
        let query = format!(
            "SELECT id, bank_id, stem, type, options, answer, explanation,
                    times_attempted, times_correct, last_attempted
             FROM questions WHERE bank_id = ?1{}{} {}",
            type_filter, limit_clause, order_clause
        );

        let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
        let questions = bind_and_query(&mut stmt, &bank_id, &question_types, &limit)?;
        Ok(questions)
    }
}

/// 绑定参数并执行查询
fn bind_and_query(
    stmt: &mut rusqlite::Statement,
    bank_id: &str,
    question_types: &Option<Vec<String>>,
    limit: &Option<u32>,
) -> Result<Vec<Question>, String> {
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    params.push(Box::new(bank_id.to_string()));

    // 添加题型参数
    if let Some(types) = question_types {
        for t in types {
            params.push(Box::new(t.clone()));
        }
    }

    // 添加 limit 参数
    if let Some(l) = limit {
        params.push(Box::new(*l));
    }

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let questions = stmt
        .query_map(params_refs.as_slice(), |row| {
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

/// 记录一次做题
#[tauri::command]
pub fn record_practice(
    state: tauri::State<DbState>,
    question_id: String,
    bank_id: String,
    user_answer: String,
    is_correct: bool,
    mode: String,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        "INSERT INTO practice_records (question_id, bank_id, user_answer, is_correct, mode, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![question_id, bank_id, user_answer, is_correct as i32, mode, now],
    )
    .map_err(|e| e.to_string())?;

    if is_correct {
        conn.execute(
            "UPDATE questions SET times_attempted = times_attempted + 1,
             times_correct = times_correct + 1, last_attempted = ?1 WHERE id = ?2",
            rusqlite::params![now, question_id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE questions SET times_attempted = times_attempted + 1,
             last_attempted = ?1 WHERE id = ?2",
            rusqlite::params![now, question_id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 获取某题库的做题统计
#[tauri::command]
pub fn get_practice_stats(
    state: tauri::State<DbState>,
    bank_id: String,
) -> Result<serde_json::Value, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    let total: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM questions WHERE bank_id = ?1",
            rusqlite::params![bank_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let attempted: u32 = conn
        .query_row(
            "SELECT COUNT(DISTINCT question_id) FROM practice_records WHERE bank_id = ?1",
            rusqlite::params![bank_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let total_correct: u32 = conn
        .query_row(
            "SELECT COALESCE(SUM(times_correct), 0) FROM questions WHERE bank_id = ?1",
            rusqlite::params![bank_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let total_attempts: u32 = conn
        .query_row(
            "SELECT COALESCE(SUM(times_attempted), 0) FROM questions WHERE bank_id = ?1",
            rusqlite::params![bank_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let wrong_count: u32 = conn
        .query_row(
            "SELECT COUNT(DISTINCT question_id) FROM practice_records
             WHERE bank_id = ?1 AND is_correct = 0",
            rusqlite::params![bank_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "total": total,
        "attempted": attempted,
        "total_correct": total_correct,
        "total_attempts": total_attempts,
        "wrong_count": wrong_count,
    }))
}

/// 获取全局统计（Dashboard 用）
#[tauri::command]
pub fn get_global_stats(
    state: tauri::State<DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

    let total_banks: u32 = conn
        .query_row("SELECT COUNT(*) FROM banks", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let total_questions: u32 = conn
        .query_row("SELECT COUNT(*) FROM questions", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let total_records: u32 = conn
        .query_row("SELECT COUNT(*) FROM practice_records", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    let total_correct: u32 = conn
        .query_row(
            "SELECT COALESCE(SUM(is_correct), 0) FROM practice_records",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let today_records: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM practice_records WHERE timestamp LIKE ?1",
            rusqlite::params![format!("{}%", today)],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let wrong_count: u32 = conn
        .query_row(
            "SELECT COUNT(DISTINCT question_id) FROM practice_records WHERE is_correct = 0",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // 各题库统计
    let mut stmt = conn
        .prepare(
            "SELECT b.id, b.name,
                    COUNT(DISTINCT q.id) as q_count,
                    COUNT(DISTINCT pr.question_id) as attempted,
                    COALESCE(SUM(CASE WHEN pr.is_correct = 1 THEN 1 ELSE 0 END), 0) as correct,
                    COUNT(pr.id) as total_pr
             FROM banks b
             LEFT JOIN questions q ON b.id = q.bank_id
             LEFT JOIN practice_records pr ON b.id = pr.bank_id
             GROUP BY b.id
             ORDER BY b.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let bank_stats: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "question_count": row.get::<_, u32>(2)?,
                "attempted": row.get::<_, u32>(3)?,
                "correct": row.get::<_, u32>(4)?,
                "total_practice": row.get::<_, u32>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(serde_json::json!({
        "total_banks": total_banks,
        "total_questions": total_questions,
        "total_practice": total_records,
        "total_correct": total_correct,
        "today_practice": today_records,
        "wrong_count": wrong_count,
        "banks": bank_stats,
    }))
}

/// 获取做题历史记录
#[tauri::command]
pub fn get_practice_records(
    state: tauri::State<DbState>,
    bank_id: String,
    limit: Option<u32>,
) -> Result<Vec<PracticeRecord>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(100);

    let mut stmt = conn
        .prepare(
            "SELECT id, question_id, bank_id, user_answer, is_correct, mode, timestamp
             FROM practice_records WHERE bank_id = ?1
             ORDER BY timestamp DESC LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;

    let records = stmt
        .query_map(rusqlite::params![bank_id, limit], |row| {
            Ok(PracticeRecord {
                id: Some(row.get(0)?),
                question_id: row.get(1)?,
                bank_id: row.get(2)?,
                user_answer: row.get(3)?,
                is_correct: row.get::<_, i32>(4)? != 0,
                mode: row.get(5)?,
                timestamp: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(records)
}
