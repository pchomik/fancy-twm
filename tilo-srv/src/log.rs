//! Minimal file logger for diagnosing tiling/tag logic.
//!
//! Writes timestamped lines to `%USERPROFILE%\.config\tilo\tilosrv.log`.
//! Enabled via the `TILOSRV_LOG` environment variable (set to `1`).
//! When disabled, all logging calls are effectively no-ops (cheap check).

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static ENABLED: AtomicBool = AtomicBool::new(false);
static LOG_PATH: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Initializes the logger. Call once at startup.
pub fn init() {
    let enabled = std::env::var("TILOSRV_LOG")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    ENABLED.store(enabled, Ordering::Relaxed);

    if enabled {
        if let Some(dir) = dirs::home_dir().map(|p| p.join(".config").join("tilo")) {
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join("tilosrv.log");
            // Truncate on start so each run gets a fresh log.
            let _ = std::fs::write(&path, "");
            let _ = LOG_PATH.set(path);
        }
    }
}

/// Returns whether logging is enabled.
#[inline]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Appends a single line to the log file with a timestamp.
pub fn write(line: &str) {
    if !enabled() {
        return;
    }
    let Some(path) = LOG_PATH.get() else {
        return;
    };

    // Simple timestamp: milliseconds since process start would need a start
    // instant; instead use system time formatted as HH:MM:SS.mmm.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = now.as_secs();
    let millis = now.subsec_millis();
    let secs_of_day = total_secs % 86400;
    let h = secs_of_day / 3600;
    let m = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{h:02}:{m:02}:{s:02}.{millis:03}] {line}");
    }
}

/// Convenience macro: `log!("format {}", args)`.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        if $crate::log::enabled() {
            $crate::log::write(&format!($($arg)*));
        }
    };
}
