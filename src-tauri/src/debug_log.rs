// Debug NDJSON logger (session c82dbd)
use serde_json::json;
use std::io::Write;

pub fn agent_log(location: &str, message: &str, hypothesis_id: &str, data: serde_json::Value) {
    let log_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("debug-c82dbd.log"));

    let Some(log_path) = log_path else {
        return;
    };

    let entry = json!({
        "sessionId": "c82dbd",
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "runId": "import-excel-kylin"
    });

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let _ = writeln!(file, "{}", entry);
    }
}
