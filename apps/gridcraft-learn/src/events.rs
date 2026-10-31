//! Local-only, append-only JSONL action log for later offline analysis.
//!
//! Events are written next to the progress file, in a sibling `events/` folder, one file per
//! local day (`YYYY-MM-DD.jsonl`), one JSON object per line. The log is best-effort: an
//! unwritable directory never breaks practice, and nothing is ever sent over the network.
//!
//! Privacy: the log deliberately records *what the learner did* (keys, clicks, typed text) and
//! stays on this machine. It is off-switchable and disabled whenever there is no persistent
//! progress path (tests, ephemeral runs), so tests never touch the user's home.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Daily files are pruned oldest-first once the `events/` folder grows past this.
pub const MAX_LOG_BYTES: u64 = 200 * 1024 * 1024;

/// A wall-clock stamp for the local action log (never used for scheduling).
#[expect(clippy::disallowed_methods, reason = "the wall clock belongs to the native-only desktop shell")]
fn unix_now() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default()
}

fn mix(x: u64) -> u64 {
    let x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    let x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// A session id that distinguishes runs started from the same install; not a secret.
fn new_session_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = unix_now().as_nanos() as u64;
    let pid = u64::from(std::process::id());
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    mix(nanos ^ (pid << 32) ^ c.wrapping_mul(0x9e37_79b9_7f4a_7c15))
}

/// Days since 1970-01-01 → civil date (Howard Hinnant's `civil_from_days`, UTC).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn day_stamp(ms: u64) -> String {
    let days = (ms / 1000) as i64 / 86_400;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// A local JSONL writer tied to one progress file's directory.
pub struct EventLog {
    /// `Some` when a persistent path exists; `None` disables the log entirely.
    dir: Option<PathBuf>,
    enabled: bool,
    session: u64,
    screen: String,
    max_bytes: u64,
}

impl EventLog {
    /// Builds from the same path as `progress.json`; events live in its sibling `events/` folder.
    /// With no persistent path (`None`) the log is inert, so tests never write to the user's home.
    pub fn new(progress: Option<&Path>, enabled: bool) -> Self {
        Self::with_limit(progress, enabled, MAX_LOG_BYTES)
    }

    /// Like [`EventLog::new`] but with a custom rotation cap (used by tests).
    pub fn with_limit(progress: Option<&Path>, enabled: bool, max_bytes: u64) -> Self {
        let dir = progress.and_then(Path::parent).filter(|d| !d.as_os_str().is_empty()).map(|d| d.join("events"));
        Self { dir, enabled, session: new_session_id(), screen: String::new(), max_bytes }
    }

    pub fn session_id(&self) -> u64 {
        self.session
    }

    /// Turns writing on or off at runtime (the settings checkbox); the target directory is kept.
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled && self.dir.is_some()
    }

    /// The screen label attached to subsequent events.
    pub fn set_screen(&mut self, screen: &str) {
        self.screen.clear();
        self.screen.push_str(screen);
    }

    /// Appends one line. Base keys (`ts`, `session`, `screen`, `kind`) win over `fields`.
    pub fn log(&mut self, kind: &str, fields: serde_json::Value) {
        if !self.enabled {
            return;
        }
        let Some(dir) = self.dir.clone() else { return };
        let ms = unix_now().as_millis() as u64;
        let mut obj = match fields {
            serde_json::Value::Object(map) => map,
            other => {
                let mut m = serde_json::Map::new();
                m.insert("fields".into(), other);
                m
            }
        };
        obj.insert("ts".into(), serde_json::Value::from(ms));
        obj.insert("session".into(), serde_json::Value::from(self.session));
        obj.insert("screen".into(), serde_json::Value::from(self.screen.clone()));
        obj.insert("kind".into(), serde_json::Value::from(kind));
        let line = serde_json::Value::Object(obj).to_string();
        let file = dir.join(format!("{}.jsonl", day_stamp(ms)));
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
            let _ = f.write_all(line.as_bytes());
            let _ = f.write_all(b"\n");
        }
        self.prune();
    }

    /// Appends one answer record to `mistakes.jsonl` next to the progress file: wrong answers
    /// and later corrections, kept apart from the action log so they can be reviewed to adapt
    /// the course. Same rules as [`EventLog::log`]: local only, inert without a progress path.
    pub fn mistake(&self, fields: serde_json::Value) {
        if !self.enabled {
            return;
        }
        let Some(file) = self.dir.as_ref().and_then(|d| d.parent()).map(|p| p.join("mistakes.jsonl")) else { return };
        let ms = unix_now().as_millis() as u64;
        let mut obj = match fields {
            serde_json::Value::Object(map) => map,
            other => {
                let mut m = serde_json::Map::new();
                m.insert("fields".into(), other);
                m
            }
        };
        obj.insert("ts".into(), serde_json::Value::from(ms));
        obj.insert("day".into(), serde_json::Value::from(day_stamp(ms)));
        obj.insert("session".into(), serde_json::Value::from(self.session));
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
            let _ = f.write_all(serde_json::Value::Object(obj).to_string().as_bytes());
            let _ = f.write_all(b"\n");
        }
    }

    /// Removes the oldest daily files while the folder is over the cap.
    pub(crate) fn prune(&self) {
        self.prune_to(self.max_bytes);
    }

    /// `prune` with an explicit byte cap, so tests need no 200 MiB fixtures.
    pub(crate) fn prune_to(&self, max_bytes: u64) {
        let Some(dir) = self.dir.as_ref() else { return };
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        let mut files: Vec<(PathBuf, u64)> = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().is_some_and(|x| x == "jsonl")
                && let Ok(meta) = e.metadata()
                && meta.is_file()
            {
                files.push((path, meta.len()));
            }
        }
        files.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));
        let mut total: u64 = files.iter().map(|(_, len)| *len).sum();
        for (path, len) in &files {
            if total <= max_bytes {
                break;
            }
            if std::fs::remove_file(path).is_ok() {
                total = total.saturating_sub(*len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scratch(name: &str) -> PathBuf {
        let root = Path::new("/home/y/.hermes/cache/scratch/opencode");
        let base = if root.is_dir() { root.to_path_buf() } else { std::env::temp_dir() };
        base.join(format!("events-{name}-{}", std::process::id()))
    }

    #[test]
    fn writes_valid_jsonl_and_disabled_writes_nothing() {
        let dir = scratch("write");
        std::fs::create_dir_all(&dir).unwrap();
        let progress = dir.join("progress.json");

        let mut log = EventLog::with_limit(Some(&progress), true, MAX_LOG_BYTES);
        log.set_screen("Lesson");
        log.log("check", json!({"passed": true, "seconds": 1.5}));

        let events = dir.join("events");
        let files: Vec<_> = std::fs::read_dir(&events).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(files.len(), 1);
        let text = std::fs::read_to_string(&files[0]).unwrap();
        let value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(value["kind"], "check");
        assert_eq!(value["screen"], "Lesson");
        assert_eq!(value["passed"], true);
        assert!(value["ts"].as_u64().unwrap() > 0);
        assert!(value["session"].as_u64().unwrap() > 0);

        let off_dir = scratch("off");
        std::fs::create_dir_all(&off_dir).unwrap();
        let mut off = EventLog::with_limit(Some(&off_dir.join("progress.json")), false, MAX_LOG_BYTES);
        off.log("ignored", json!({}));
        assert!(!off_dir.join("events").exists());

        let mut none = EventLog::new(None, true);
        none.log("ignored", json!({}));
        assert!(!none.is_enabled());

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&off_dir);
    }

    #[test]
    fn prune_removes_the_oldest_files_past_the_cap() {
        let dir = scratch("prune");
        let events = dir.join("events");
        std::fs::create_dir_all(&events).unwrap();
        for name in ["2020-01-01", "2020-01-02", "2020-01-03"] {
            std::fs::write(events.join(format!("{name}.jsonl")), vec![b'x'; 100]).unwrap();
        }
        let log = EventLog::with_limit(Some(&dir.join("progress.json")), false, 250);
        log.prune_to(250);
        assert!(!events.join("2020-01-01.jsonl").exists(), "oldest file must be pruned first");
        assert!(events.join("2020-01-02.jsonl").exists());
        assert!(events.join("2020-01-03.jsonl").exists());

        log.prune_to(150);
        assert!(!events.join("2020-01-02.jsonl").exists());
        assert!(events.join("2020-01-03.jsonl").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
