// ============================================================
// 理论训练考核系统 — Rust 数据模型
// ============================================================
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bank {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub question_count: u32,
    pub wrong_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub bank_id: String,
    pub stem: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub options: Vec<String>,
    #[serde(deserialize_with = "deserialize_answer")]
    pub answer: serde_json::Value,
    pub explanation: String,
    pub times_attempted: u32,
    pub times_correct: u32,
    pub last_attempted: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeRecord {
    pub id: Option<i64>,
    pub question_id: String,
    pub bank_id: String,
    pub user_answer: String,
    pub is_correct: bool,
    pub mode: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: u32,
    pub failed: u32,
    pub skipped: u32,
    pub overwritten: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCheckResult {
    pub duplicate_count: u32,
    pub duplicate_stems: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceBankPreview {
    pub name: String,
    pub question_count: u32,
    pub created_at: String,
    pub name_conflict: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub path: String,
    pub size_bytes: u64,
    pub bank_count: u32,
    pub question_count: u32,
    pub record_count: u32,
    pub banks: Vec<SourceBankPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDatabasePreview {
    pub file_path: String,
    pub size_bytes: u64,
    pub bank_count: u32,
    pub question_count: u32,
    pub record_count: u32,
    pub banks: Vec<SourceBankPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReplaceResult {
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub banks_added: u32,
    pub banks_merged: u32,
    pub banks_skipped: u32,
    pub questions_added: u32,
    pub questions_skipped: u32,
    pub questions_overwritten: u32,
    pub errors: Vec<String>,
}

/// 自定义反序列化：支持字符串和数组两种格式
fn deserialize_answer<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match &value {
        serde_json::Value::String(_) | serde_json::Value::Array(_) => Ok(value),
        _ => Ok(serde_json::Value::String(value.to_string())),
    }
}
