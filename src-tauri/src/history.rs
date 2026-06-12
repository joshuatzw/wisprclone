use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub timestamp: i64, // Unix seconds
    pub text: String,
}

const FILE_NAME: &str = "history.json";

pub fn load(data_dir: &Path) -> Vec<HistoryEntry> {
    let path = data_dir.join(FILE_NAME);
    let Ok(bytes) = fs::read(&path) else {
        return vec![];
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

pub fn save(data_dir: &Path, entries: &[HistoryEntry]) {
    if let Ok(json) = serde_json::to_vec_pretty(entries) {
        fs::create_dir_all(data_dir).ok();
        fs::write(data_dir.join(FILE_NAME), json).ok();
    }
}

pub fn push(entries: &mut Vec<HistoryEntry>, text: String, data_dir: &Path) -> HistoryEntry {
    let id = entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let entry = HistoryEntry { id, timestamp, text };
    entries.insert(0, entry.clone());
    save(data_dir, entries);
    entry
}
