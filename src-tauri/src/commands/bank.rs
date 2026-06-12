// ============================================================
// 刷题助手 — 题库管理 Commands
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
    })
}

#[tauri::command]
pub fn list_banks(state: tauri::State<DbState>) -> Result<Vec<Bank>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT b.id, b.name, b.created_at, COUNT(q.id) as q_count
             FROM banks b LEFT JOIN questions q ON b.id = q.bank_id
             GROUP BY b.id ORDER BY b.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let banks = stmt
        .query_map([], |row| {
            Ok(Bank {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                question_count: row.get(3)?,
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
            "SELECT b.id, b.name, b.created_at, COUNT(q.id) as q_count
             FROM banks b LEFT JOIN questions q ON b.id = q.bank_id
             WHERE b.id = ?1 GROUP BY b.id",
        )
        .map_err(|e| e.to_string())?;

    let bank = stmt
        .query_row(rusqlite::params![id], |row| {
            Ok(Bank {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                question_count: row.get(3)?,
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
