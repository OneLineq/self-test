// ============================================================
// 刷题助手 — Rust 数据模型
// ============================================================
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bank {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub question_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub bank_id: String,
    pub stem: String,
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
