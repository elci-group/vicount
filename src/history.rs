use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Maximum number of recent entries to keep in memory.
const MAX_HISTORY: usize = 1000;

/// A single persisted input entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryEntry {
    text: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    timestamp: DateTime<Utc>,
}

/// Return the path to Vicount's data directory, creating it if necessary.
fn data_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("vicount");
    fs::create_dir_all(&dir).with_context(|| format!("creating data dir {}", dir.display()))?;
    Ok(dir)
}

/// Path to the newline-delimited history file.
pub fn history_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("history.jsonl"))
}

/// Load the most recent `limit` history entries from disk.
pub fn load_history(limit: usize) -> Vec<String> {
    let path = match history_path() {
        Ok(p) => p,
        Err(e) => {
            warn!("cannot determine history path: {e}");
            return Vec::new();
        }
    };

    if !path.exists() {
        debug!("no history file at {}", path.display());
        return Vec::new();
    }

    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            warn!("cannot open history file {}: {e}", path.display());
            return Vec::new();
        }
    };

    let reader = BufReader::new(file);
    let mut entries: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                warn!("error reading history line: {e}");
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<HistoryEntry>(&line) {
            Ok(entry) => entries.push(entry.text),
            Err(e) => warn!("malformed history entry: {e}"),
        }
    }

    // Keep only the most recent `limit` entries.
    if entries.len() > limit {
        entries.drain(..entries.len() - limit);
    }
    entries
}

/// Persist a single input entry to disk, appending to the history file.
pub fn append_history(text: &str) -> Result<()> {
    let path = history_path()?;
    let entry = HistoryEntry {
        text: text.to_string(),
        timestamp: Utc::now(),
    };
    let line = serde_json::to_string(&entry)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening history file {} for append", path.display()))?;

    writeln!(file, "{line}")
        .with_context(|| format!("writing history entry to {}", path.display()))?;

    debug!("appended history entry to {}", path.display());
    Ok(())
}

/// Trim the on-disk history file to the most recent `limit` entries.
pub fn trim_history(limit: usize) -> Result<()> {
    let path = history_path()?;
    if !path.exists() {
        return Ok(());
    }

    let history = load_history(usize::MAX);
    if history.len() <= limit {
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("truncating history file {}", path.display()))?;

    // Rebuild entries with fresh timestamps; we only store text, so use now.
    for text in history.iter().skip(history.len() - limit) {
        let entry = HistoryEntry {
            text: text.clone(),
            timestamp: Utc::now(),
        };
        writeln!(file, "{}", serde_json::to_string(&entry)?)
            .with_context(|| format!("writing trimmed history to {}", path.display()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn with_temp_history<F>(f: F)
    where
        F: FnOnce(),
    {
        let tmp = env::temp_dir().join(format!("vicount-history-test-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        // Override data_dir via environment if supported, otherwise rely on the
        // test-only helper below.
        let _guard = env::var("XDG_DATA_HOME").ok();
        env::set_var("XDG_DATA_HOME", &tmp);
        f();
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn append_and_load_roundtrip() {
        with_temp_history(|| {
            let _ = fs::remove_file(history_path().unwrap());
            append_history("hello").unwrap();
            append_history("world").unwrap();
            let loaded = load_history(10);
            assert_eq!(loaded, vec!["hello", "world"]);
        });
    }

    #[test]
    fn load_respects_limit() {
        with_temp_history(|| {
            let _ = fs::remove_file(history_path().unwrap());
            for i in 0..10 {
                append_history(&format!("entry {i}")).unwrap();
            }
            let loaded = load_history(3);
            assert_eq!(loaded.len(), 3);
            assert_eq!(loaded[0], "entry 7");
            assert_eq!(loaded[2], "entry 9");
        });
    }

    #[test]
    fn trim_history_drops_old_entries() {
        with_temp_history(|| {
            let _ = fs::remove_file(history_path().unwrap());
            for i in 0..5 {
                append_history(&format!("entry {i}")).unwrap();
            }
            trim_history(2).unwrap();
            let loaded = load_history(10);
            assert_eq!(loaded, vec!["entry 3", "entry 4"]);
        });
    }
}
