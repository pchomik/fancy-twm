//! Virtual desktop operations — thin wrappers over [`crate::platform`].
//!
//! All platform-specific details (Win10 vs Win11 API differences) are
//! handled inside `platform.rs`; this module contains only the
//! user-facing desktop navigation logic.

use crate::platform;

/// Moves the foreground window to the next virtual desktop (if one exists).
pub fn move_active_window_to_next_virtual_desktop() {
    let Some(hwnd) = platform::get_foreground_window() else {
        return;
    };
    if let Ok((count, current)) = platform::get_desktop_info()
        && current + 1 < count {
            let _ = platform::move_window_to_desktop_by_index(hwnd, current + 1);
        }
}

/// Moves the foreground window to the previous virtual desktop (if one exists).
pub fn move_active_window_to_prev_virtual_desktop() {
    let Some(hwnd) = platform::get_foreground_window() else {
        return;
    };
    if let Ok((_, current)) = platform::get_desktop_info()
        && current > 0 {
            let _ = platform::move_window_to_desktop_by_index(hwnd, current - 1);
        }
}

/// Moves the foreground window to the virtual desktop at `target_index`.
pub fn move_active_window_to_virtual_desktop(target_index: &str) {
    let Some(hwnd) = platform::get_foreground_window() else {
        return;
    };
    if let Ok(target) = target_index.parse::<usize>()
        && let Ok((count, current)) = platform::get_desktop_info()
            && target != current && target < count {
                let _ = platform::move_window_to_desktop_by_index(hwnd, target);
            }
}

/// Switches the view to the next virtual desktop.
pub fn switch_to_next_virtual_desktop() {
    if let Ok((count, current)) = platform::get_desktop_info()
        && current + 1 < count {
            let _ = platform::switch_to_desktop_by_index(current + 1);
        }
}

/// Switches the view to the previous virtual desktop.
pub fn switch_to_prev_virtual_desktop() {
    if let Ok((_, current)) = platform::get_desktop_info()
        && current > 0 {
            let _ = platform::switch_to_desktop_by_index(current - 1);
        }
}

/// Switches the view to the virtual desktop at `target_index`.
pub fn switch_to_virtual_desktop(target_index: &str) {
    if let Ok(target) = target_index.parse::<usize>()
        && let Ok((count, current)) = platform::get_desktop_info()
            && target != current && target < count {
                let _ = platform::switch_to_desktop_by_index(target);
            }
}
