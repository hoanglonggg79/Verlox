use crate::installed::verlox_root;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_HISTORY: usize = 20;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub path: String,
}

pub fn history_file() -> PathBuf {
    verlox_root().join("download_history.json")
}

pub fn load() -> Vec<HistoryEntry> {
    std::fs::read_to_string(history_file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(entries: &[HistoryEntry]) -> Result<(), String> {
    std::fs::create_dir_all(verlox_root()).map_err(|e| e.to_string())?;
    let capped: Vec<&HistoryEntry> = entries.iter().take(MAX_HISTORY).collect();
    let json = serde_json::to_string_pretty(&capped).map_err(|e| e.to_string())?;
    std::fs::write(history_file(), json).map_err(|e| e.to_string())
}

pub fn push(hash: &str, path: &str) -> Result<(), String> {
    let mut entries = load();
    entries.retain(|e| e.hash != hash);
    entries.insert(
        0,
        HistoryEntry {
            hash: hash.to_string(),
            time: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            path: path.to_string(),
        },
    );
    entries.truncate(MAX_HISTORY);
    save(&entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_via_string() {
        let entries = vec![HistoryEntry {
            hash: "version-eb4648e9148d440c".into(),
            time: "2026-11-05 11:38".into(),
            path: "C:\\Versions\\version-eb4648e9148d440c".into(),
        }];
        let json = serde_json::to_string(&entries).unwrap();
        let parsed: Vec<HistoryEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].hash, "version-eb4648e9148d440c");
        assert!(serde_json::from_str::<Vec<HistoryEntry>>(
            "[{\"hash\":\"version-abc123\",\"time\":\"\",\"path\":\"\"}]"
        )
        .is_ok());
    }
}
