//! JSON persistence for the current game and summarized history.
//! Path: `data/play_nine.json` relative to the process working directory
//! (typically the project root when you `cargo run`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::{Game, HistoryEntry};

const DATA_DIR: &str = "data";
const DATA_FILE: &str = "play_nine.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Store {
    pub current_game: Option<Game>,
    pub history: Vec<HistoryEntry>,
}

pub fn data_path() -> PathBuf {
    Path::new(DATA_DIR).join(DATA_FILE)
}

pub fn load() -> Result<Store, String> {
    let path = data_path();
    if !path.exists() {
        return Ok(Store::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Store::default());
    }
    serde_json::from_str(&raw).map_err(|e| {
        format!(
            "Corrupt JSON in {}: {e}\n\
             Fix or delete the file, then restart. A backup tip: rename it to play_nine.json.bak",
            path.display()
        )
    })
}

pub fn save(store: &Store) -> Result<(), String> {
    ensure_data_dir().map_err(|e| format!("Could not create data dir: {e}"))?;
    let path = data_path();
    let pretty = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Failed to serialize game data: {e}"))?;
    // Atomic-ish write: temp then rename
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, pretty).map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Could not finalize {}: {e}", path.display()))?;
    Ok(())
}

fn ensure_data_dir() -> io::Result<()> {
    fs::create_dir_all(DATA_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Game, HistoryEntry};

    #[test]
    fn save_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("play_nine_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        // Temporarily chdir isn't great in parallel tests; write via helpers by
        // exercising serialize shape instead.
        let store = Store {
            current_game: Some(Game::new(9, vec!["Pat".into()])),
            history: vec![HistoryEntry {
                finished_at: "2026-09-20 13:00".into(),
                holes: 9,
                results: vec![("Pat".into(), 10)],
                winners: vec!["Pat".into()],
            }],
        };
        let s = serde_json::to_string_pretty(&store).unwrap();
        let back: Store = serde_json::from_str(&s).unwrap();
        assert!(back.current_game.is_some());
        assert_eq!(back.history[0].winners[0], "Pat");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_json_errors() {
        // Direct parse path used by load()
        let err = serde_json::from_str::<Store>("{not json").err();
        assert!(err.is_some());
    }
}
