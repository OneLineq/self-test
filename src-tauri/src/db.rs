// ============================================================
// 刷题助手 — SQLite 数据库初始化与管理
// ============================================================
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct DbState(pub Mutex<Connection>);

pub fn init_db(app_dir: &Path) -> Connection {
    std::fs::create_dir_all(app_dir).ok();
    let db_path = app_dir.join("quiz_app.db");
    let conn = Connection::open(&db_path).expect("Failed to open database");

    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS banks (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            created_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS questions (
            id              TEXT PRIMARY KEY,
            bank_id         TEXT NOT NULL,
            stem            TEXT NOT NULL,
            type            TEXT DEFAULT '',
            options         TEXT NOT NULL DEFAULT '[]',
            answer          TEXT NOT NULL DEFAULT '\"\"',
            explanation     TEXT DEFAULT '',
            times_attempted INTEGER DEFAULT 0,
            times_correct   INTEGER DEFAULT 0,
            last_attempted  TEXT,
            FOREIGN KEY (bank_id) REFERENCES banks(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS practice_records (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            question_id TEXT NOT NULL,
            bank_id     TEXT NOT NULL,
            user_answer TEXT NOT NULL,
            is_correct  INTEGER NOT NULL,
            mode        TEXT NOT NULL,
            timestamp   TEXT NOT NULL,
            FOREIGN KEY (question_id) REFERENCES questions(id),
            FOREIGN KEY (bank_id) REFERENCES banks(id)
        );

        CREATE INDEX IF NOT EXISTS idx_questions_bank ON questions(bank_id);
        CREATE INDEX IF NOT EXISTS idx_practice_question ON practice_records(question_id);
        CREATE INDEX IF NOT EXISTS idx_practice_bank ON practice_records(bank_id);
        ",
    )
    .expect("Failed to create tables");

    conn
}
