use anyhow::Result;

#[cfg(feature = "windows10")]
use windows_win10::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};

#[cfg(feature = "windows11")]
use windows_win11::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage, WM_QUIT,
};

// This solution can be replaced by inputbot crate which replace global-hotkeys
// Crate inputbot registers SetWindowsHookExW hook and runs own message loop.
// Is able to support keyboard and mouse. Is heavier solution. Consider that later.
pub fn pump_windows_messages() -> Result<bool> {
    let mut message = MSG::default();

    unsafe {
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).into() {
            if message.message == WM_QUIT {
                return Ok(false);
            }

            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    Ok(true)
}
