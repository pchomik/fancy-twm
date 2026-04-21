#[cfg(feature = "windows10")]
use windows_win10::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

#[cfg(feature = "windows11")]
use windows_win11::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

#[cfg(feature = "windows10")]
use winvd_win10::{get_current_desktop, get_desktops, move_window_to_desktop};

#[cfg(feature = "windows11")]
use winvd_win11::{get_current_desktop, get_desktops, move_window_to_desktop};

#[cfg(feature = "windows10")]
pub fn move_active_window_to_next_virtual_desktop() {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 != 0 {
            if let (Ok(desktops), Ok(current)) = (get_desktops(), get_current_desktop()) {
                if let Some(current_index) = desktops.iter().position(|d| d == &current) {
                    if current_index < desktops.len() - 1 {
                        let next_desktop = &desktops[current_index + 1];
                        let _ = move_window_to_desktop(hwnd.0 as u32, next_desktop);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "windows10")]
pub fn move_active_window_to_virtual_desktop(target_index: &String) {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 != 0 {
            if let (Ok(desktops), Ok(current)) = (get_desktops(), get_current_desktop()) {
                if let Some(current_index) = desktops.iter().position(|d| d == &current) {
                    if let Ok(target_index) = target_index.parse::<usize>() {
                        if target_index != current_index && target_index < desktops.len() {
                            let target_desktop = &desktops[target_index];
                            let _ = move_window_to_desktop(hwnd.0 as u32, target_desktop);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "windows11")]
pub fn move_active_window_to_next_virtual_desktop() {
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_invalid() {
            if let (Ok(desktops), Ok(current)) = (get_desktops(), get_current_desktop()) {
                if let Some(current_index) = desktops.iter().position(|d| d == &current) {
                    if current_index < desktops.len() - 1 {
                        let next_desktop = &desktops[current_index + 1];
                        let _ = move_window_to_desktop(next_desktop.clone(), &hwnd);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "windows11")]
pub fn move_active_window_to_virtual_desktop(target_index: &String) {
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_invalid() {
            if let (Ok(desktops), Ok(current)) = (get_desktops(), get_current_desktop()) {
                if let Some(current_index) = desktops.iter().position(|d| d == &current) {
                    if let Ok(target_index) = target_index.parse::<usize>() {
                        if target_index != current_index && target_index < desktops.len() {
                            let target_desktop = &desktops[target_index];
                            let _ = move_window_to_desktop(target_desktop.clone(), &hwnd);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "windows10")]
pub fn move_active_window_to_prev_virtual_desktop() {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 != 0 {
            if let (Ok(desktops), Ok(current)) = (get_desktops(), get_current_desktop()) {
                if let Some(current_index) = desktops.iter().position(|d| d == &current) {
                    if current_index > 0 {
                        let prev_desktop = &desktops[current_index - 1];
                        let _ = move_window_to_desktop(hwnd.0 as u32, prev_desktop);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "windows11")]
pub fn move_active_window_to_prev_virtual_desktop() {
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_invalid() {
            if let (Ok(desktops), Ok(current)) = (get_desktops(), get_current_desktop()) {
                if let Some(current_index) = desktops.iter().position(|d| d == &current) {
                    if current_index > 0 {
                        let prev_desktop = &desktops[current_index - 1];
                        let _ = move_window_to_desktop(prev_desktop.clone(), &hwnd);
                    }
                }
            }
        }
    }
}
