// ============================================================
// 理论训练考核系统 — 题库管理 Commands
// ============================================================
use crate::db::DbState;
use crate::models::Bank;
use chrono::Local;
use rusqlite::OptionalExtension;
use uuid::Uuid;

#[tauri::command]
pub fn create_bank(state: tauri::State<DbState>, name: String) -> Result<Bank, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    conn.execute(
        "INSERT INTO banks (id, name, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![id, name, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(Bank {
        id,
        name,
        created_at: now,
        question_count: 0,
        wrong_count: 0,
    })
}

#[tauri::command]
pub fn list_banks(state: tauri::State<DbState>) -> Result<Vec<Bank>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT b.id, b.name, b.created_at,
                    COALESCE(qs.q_count, 0),
                    COALESCE(ws.w_count, 0)
             FROM banks b
             LEFT JOIN (
                 SELECT bank_id, COUNT(*) AS q_count FROM questions GROUP BY bank_id
             ) qs ON qs.bank_id = b.id
             LEFT JOIN (
                 SELECT bank_id, COUNT(DISTINCT question_id) AS w_count
                 FROM practice_records WHERE is_correct = 0
                 GROUP BY bank_id
             ) ws ON ws.bank_id = b.id
             ORDER BY b.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let banks = stmt
        .query_map([], |row| {
            Ok(Bank {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                question_count: row.get(3)?,
                wrong_count: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(banks)
}

#[tauri::command]
pub fn get_bank(state: tauri::State<DbState>, id: String) -> Result<Option<Bank>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT b.id, b.name, b.created_at,
                    (SELECT COUNT(*) FROM questions WHERE bank_id = b.id),
                    (SELECT COUNT(DISTINCT question_id) FROM practice_records
                     WHERE bank_id = b.id AND is_correct = 0)
             FROM banks b WHERE b.id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let bank = stmt
        .query_row(rusqlite::params![id], |row| {
            Ok(Bank {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                question_count: row.get(3)?,
                wrong_count: row.get(4)?,
            })
        })
        .optional()
        .map_err(|e| e.to_string())?;

    Ok(bank)
}

#[tauri::command]
pub fn delete_bank(state: tauri::State<DbState>, id: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM practice_records WHERE bank_id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM questions WHERE bank_id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM banks WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rename_bank(state: tauri::State<DbState>, id: String, name: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE banks SET name = ?1 WHERE id = ?2",
        rusqlite::params![name, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}






