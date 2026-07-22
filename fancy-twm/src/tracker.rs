//! Window tracking — maintains the ordered set of tracked (tileable) windows
//! per virtual desktop, driven by WinEvent hooks and an optional periodic scan.

use crate::config::AppConfig;
use crate::platform::{self, HWND, WindowEvent};
use anyhow::Result;
use regex::Regex;
use std::sync::mpsc;
use std::time::Instant;

/// A compiled ignore rule.
struct CompiledIgnoreRule {
    process: Option<Regex>,
    title: Option<Regex>,
}

/// Tracks the set of tileable windows on the current virtual desktop.
///
/// Windows are kept in a stable order that reflects their tiling position.
/// New windows are appended at the end; the tiling engine is responsible for
/// the "new window takes first area, others shift" semantics by reordering.
pub struct WindowTracker {
    /// Ordered list of currently tracked windows.
    windows: Vec<HWND>,
    /// Compiled ignore rules from configuration.
    ignore_rules: Vec<CompiledIgnoreRule>,
    /// Receiver for WinEvent-driven window events.
    event_rx: mpsc::Receiver<WindowEvent>,
    /// Whether the periodic scan fallback is enabled.
    scan_enabled: bool,
    /// Interval between periodic scans.
    scan_interval_ms: u64,
    /// When the last periodic scan ran.
    last_scan: Instant,
}

impl WindowTracker {
    /// Creates a tracker and installs the global WinEvent hook.
    pub fn new(config: &AppConfig) -> Result<Self> {
        let ignore_rules = config
            .ignore
            .iter()
            .map(|rule| {
                Ok(CompiledIgnoreRule {
                    process: rule
                        .process
                        .as_deref()
                        .map(Regex::new)
                        .transpose()
                        .map_err(|e| anyhow::anyhow!("Invalid process regex: {e}"))?,
                    title: rule
                        .title
                        .as_deref()
                        .map(Regex::new)
                        .transpose()
                        .map_err(|e| anyhow::anyhow!("Invalid title regex: {e}"))?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let event_rx = platform::start_window_event_listener()?;

        let mut tracker = Self {
            windows: Vec::new(),
            ignore_rules,
            event_rx,
            scan_enabled: config.scan.enabled,
            scan_interval_ms: config.scan.interval_ms,
            last_scan: Instant::now()
                .checked_sub(std::time::Duration::from_millis(config.scan.interval_ms))
                .unwrap_or_else(Instant::now),
        };

        // Initial population.
        tracker.full_scan();

        Ok(tracker)
    }

    /// Returns whether a window matches any ignore rule (match if ANY
    /// specified field matches).
    fn is_ignored(&self, hwnd: HWND) -> bool {
        let process = platform::get_window_process_name(hwnd);
        let title = platform::get_window_title(hwnd);

        for rule in &self.ignore_rules {
            let process_match = rule
                .process
                .as_ref()
                .map(|re| re.is_match(&process))
                .unwrap_or(false);
            let title_match = rule
                .title
                .as_ref()
                .map(|re| re.is_match(&title))
                .unwrap_or(false);

            if process_match || title_match {
                return true;
            }
        }

        false
    }

    /// Whether a window should be tracked (tileable + not ignored).
    ///
    /// Does NOT check virtual desktop membership — callers must ensure the
    /// window is on the current VD before calling this.
    fn is_trackable(&self, hwnd: HWND) -> bool {
        platform::is_window_tileable(hwnd) && !self.is_ignored(hwnd)
    }

    /// Rebuilds the tracked window list from scratch.
    ///
    /// Preserves already-tracked windows that are still valid (exist, on the
    /// current VD, tileable, not ignored) and discovers any new tileable
    /// windows on the current VD.
    fn full_scan(&mut self) {
        // Keep existing windows that are still valid.
        let mut ordered: Vec<HWND> = self
            .windows
            .iter()
            .copied()
            .filter(|&hwnd| {
                platform::is_window(hwnd)
                    && platform::is_window_on_current_vd(hwnd)
                    && platform::is_window_tileable(hwnd)
                    && !self.is_ignored(hwnd)
            })
            .collect();

        // Discover tileable windows on the current VD.
        if let Ok(all) = platform::get_windows_on_current_vd() {
            for hwnd in all {
                if !ordered.contains(&hwnd)
                    && platform::is_window_tileable(hwnd)
                    && !self.is_ignored(hwnd)
                {
                    ordered.push(hwnd);
                }
            }
        }

        crate::log!(
            "tracker.full_scan: {} windows: {:?}",
            ordered.len(),
            ordered.iter().map(|w| w.0 as usize).collect::<Vec<_>>()
        );
        self.windows = ordered;
        self.last_scan = Instant::now();
    }

    /// Removes a window from tracking if present.
    fn remove(&mut self, hwnd: HWND) {
        self.windows.retain(|w| *w != hwnd);
    }

    /// Re-validates tracked windows and discovers new ones on the current VD.
    ///
    /// Equivalent to a full scan: keeps windows that are still valid (exist,
    /// tileable, not ignored, on current VD) and discovers new tileable
    /// windows.
    pub fn refresh(&mut self) {
        self.full_scan();
    }

    /// Adds a window to tracking if it is on the current VD, tileable, not
    /// ignored, and not already present.
    fn add(&mut self, hwnd: HWND) {
        if self.windows.contains(&hwnd) {
            return;
        }
        if platform::is_window_on_current_vd(hwnd) && self.is_trackable(hwnd) {
            self.windows.push(hwnd);
        }
    }

    /// Polls for window events and runs the periodic scan when due.
    ///
    /// Returns `true` if the tracked window set changed and a re-tile is
    /// needed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;

        // Drain all pending WinEvents.
        while let Ok(evt) = self.event_rx.try_recv() {
            match evt {
                WindowEvent::Created(handle) => {
                    self.add(handle.to_hwnd());
                    changed = true;
                }
                WindowEvent::Restored(handle) => {
                    self.add(handle.to_hwnd());
                    changed = true;
                }
                WindowEvent::Destroyed(handle) | WindowEvent::Minimized(handle) => {
                    let hwnd = handle.to_hwnd();
                    if self.windows.contains(&hwnd) {
                        self.remove(hwnd);
                        changed = true;
                    }
                }
                WindowEvent::Moved(_) => {
                    // Position changes are handled by the periodic position
                    // check, not by re-tiling.
                }
            }
        }

        // Periodic scan fallback (catches events the hook may miss and
        // windows that moved to/from the current virtual desktop).
        if self.scan_enabled
            && self.last_scan.elapsed().as_millis() >= self.scan_interval_ms as u128
        {
            let before = self.windows.clone();
            self.full_scan();
            if self.windows != before {
                changed = true;
            }
        }

        changed
    }

    /// Returns the current ordered list of tracked windows.
    pub fn windows(&self) -> &[HWND] {
        &self.windows
    }

    /// Replaces the tracked window order (used by the tiling engine to
    /// persist its computed ordering).
    pub fn set_order(&mut self, windows: Vec<HWND>) {
        self.windows = windows;
    }

    /// Drops a specific window from tracking and returns whether it changed.
    pub fn contains(&self, hwnd: HWND) -> bool {
        self.windows.contains(&hwnd)
    }
}
