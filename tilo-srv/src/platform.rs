//! Platform abstraction layer — hides Windows 10 / Windows 11 API differences.
//!
//! All Win32 and winvd calls go through this module so the rest of the
//! codebase remains platform-agnostic.

use anyhow::{Context, Result};
use std::sync::OnceLock;
use std::sync::mpsc;

// ── Re-export the correct HWND type ──────────────────────────────────
#[cfg(feature = "windows10")]
pub use windows_win10::Win32::Foundation::HWND;
#[cfg(feature = "windows11")]
pub use windows_win11::Win32::Foundation::HWND;

// ── Foundation types ─────────────────────────────────────────────────
#[cfg(feature = "windows10")]
use windows_win10::Win32::Foundation::{BOOL, CloseHandle, LPARAM, RECT};
#[cfg(feature = "windows11")]
use windows_win11::Win32::Foundation::{BOOL, CloseHandle, LPARAM, RECT};

#[cfg(feature = "windows10")]
use windows_win10::core::PWSTR;
#[cfg(feature = "windows11")]
use windows_win11::core::PWSTR;

// ── Win32 imports ────────────────────────────────────────────────────
#[cfg(feature = "windows10")]
use windows_win10::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GWL_EXSTYLE, GWL_STYLE, GetForegroundWindow, GetMessageW,
    GetParent, GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed, MSG, PM_REMOVE,
    PeekMessageW, SWP_NOACTIVATE, SWP_NOSENDCHANGING, SWP_NOZORDER, SetWindowPos, TranslateMessage,
    WM_QUIT, WS_EX_TOOLWINDOW, WS_THICKFRAME,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EnumWindows, GWL_EXSTYLE, GWL_STYLE, GetForegroundWindow, GetMessageW,
    GetParent, GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, IsZoomed, MSG, PM_REMOVE,
    PeekMessageW, SWP_NOACTIVATE, SWP_NOSENDCHANGING, SWP_NOZORDER, SetWindowPos, TranslateMessage,
    WM_QUIT, WS_EX_TOOLWINDOW, WS_THICKFRAME,
};

// ── GDI / Monitor APIs ───────────────────────────────────────────────
#[cfg(feature = "windows10")]
use windows_win10::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
#[cfg(feature = "windows10")]
use windows_win10::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MONITORINFOEXW, MonitorFromWindow,
};
#[cfg(feature = "windows10")]
use windows_win10::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
    SetProcessDpiAwarenessContext,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
#[cfg(feature = "windows11")]
use windows_win11::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MONITORINFOEXW, MonitorFromWindow,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForMonitor, MDT_EFFECTIVE_DPI,
    SetProcessDpiAwarenessContext,
};

// ── COM for IVirtualDesktopManager ───────────────────────────────────
#[cfg(feature = "windows10")]
use windows_win10::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
#[cfg(feature = "windows11")]
use windows_win11::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};

#[cfg(feature = "windows10")]
use windows_win10::Win32::UI::Shell::IVirtualDesktopManager;
#[cfg(feature = "windows11")]
use windows_win11::Win32::UI::Shell::IVirtualDesktopManager;

/// CLSID_VirtualDesktopManager = {AA509086-5CA9-4C25-8F95-589D3C07B48A}
#[cfg(feature = "windows10")]
const CLSID_VIRTUAL_DESKTOP_MANAGER: windows_win10::core::GUID =
    windows_win10::core::GUID::from_u128(0xAA509086_5CA9_4C25_8F95_589D3C07B48A);
#[cfg(feature = "windows11")]
const CLSID_VIRTUAL_DESKTOP_MANAGER: windows_win11::core::GUID =
    windows_win11::core::GUID::from_u128(0xAA509086_5CA9_4C25_8F95_589D3C07B48A);

// ── Threading / process APIs ─────────────────────────────────────────
#[cfg(feature = "windows10")]
use windows_win10::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

// ── WinEvent hook APIs ───────────────────────────────────────────────
#[cfg(feature = "windows10")]
use windows_win10::Win32::UI::Accessibility::SetWinEventHook;
#[cfg(feature = "windows11")]
use windows_win11::Win32::UI::Accessibility::SetWinEventHook;

// WinEvent constants (not always exposed as named items in older windows crates).
const EVENT_SYSTEM_MOVESIZEEND: u32 = 0x000B;
const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
const EVENT_SYSTEM_MINIMIZESTART: u32 = 0x0016;
const EVENT_SYSTEM_MINIMIZEEND: u32 = 0x0017;
const EVENT_OBJECT_CREATE: u32 = 0x8000;
const EVENT_OBJECT_DESTROY: u32 = 0x8001;
const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

// ── winvd imports ────────────────────────────────────────────────────
#[cfg(feature = "windows10")]
use winvd_win10::{get_current_desktop, get_desktops, go_to_desktop, move_window_to_desktop};
#[cfg(feature = "windows11")]
use winvd_win11::{get_current_desktop, get_desktops, move_window_to_desktop, switch_desktop};

// ── Foreground window ────────────────────────────────────────────────

/// Sets the process DPI awareness to Per-Monitor V2.
///
/// Without this, Windows applies DPI virtualization: when a window is moved
/// between monitors with different DPI, the system rescales the window after
/// `SetWindowPos`, causing a visible double-resize (correct → wrong → correct).
/// With PMv2, `SetWindowPos` coordinates are physical pixels on every monitor
/// and the system performs no additional rescaling — the window is positioned
/// and sized exactly once.
///
/// Must be called before any window is created or positioned.
pub fn set_process_dpi_awareness() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// Returns the foreground window, or `None` when there is no valid one.
pub fn get_foreground_window() -> Option<HWND> {
    unsafe {
        let hwnd = GetForegroundWindow();

        #[cfg(feature = "windows10")]
        {
            if hwnd.0 != 0 {
                return Some(hwnd);
            }
        }

        #[cfg(feature = "windows11")]
        {
            if !hwnd.is_invalid() {
                return Some(hwnd);
            }
        }

        None
    }
}

// ── Virtual desktop helpers ──────────────────────────────────────────

/// Returns `(desktop_count, current_desktop_index)` in a single enumeration.
pub fn get_desktop_info() -> Result<(usize, usize)> {
    let desktops = get_desktops()
        .map_err(|e| anyhow::anyhow!("Failed to enumerate virtual desktops: {e:?}"))?;
    let current = get_current_desktop()
        .map_err(|e| anyhow::anyhow!("Failed to get current virtual desktop: {e:?}"))?;
    let count = desktops.len();
    let index = desktops
        .iter()
        .position(|d| d == &current)
        .context("Current desktop not found in desktop list")?;
    Ok((count, index))
}

/// Moves `hwnd` to the virtual desktop at `index`.
pub fn move_window_to_desktop_by_index(hwnd: HWND, index: usize) -> Result<()> {
    let desktops = get_desktops()
        .map_err(|e| anyhow::anyhow!("Failed to enumerate virtual desktops: {e:?}"))?;
    anyhow::ensure!(
        index < desktops.len(),
        "Desktop index {index} out of range ({})",
        desktops.len()
    );

    #[cfg(feature = "windows10")]
    {
        move_window_to_desktop(hwnd.0 as u32, &desktops[index])
            .map_err(|e| anyhow::anyhow!("Failed to move window to desktop {index}: {e:?}"))?;
    }

    #[cfg(feature = "windows11")]
    {
        move_window_to_desktop(desktops[index].clone(), &hwnd)
            .map_err(|e| anyhow::anyhow!("Failed to move window to desktop {index}: {e:?}"))?;
    }

    Ok(())
}

/// Switches the active virtual desktop to the one at `index`.
pub fn switch_to_desktop_by_index(index: usize) -> Result<()> {
    let desktops = get_desktops()
        .map_err(|e| anyhow::anyhow!("Failed to enumerate virtual desktops: {e:?}"))?;
    anyhow::ensure!(
        index < desktops.len(),
        "Desktop index {index} out of range ({})",
        desktops.len()
    );

    #[cfg(feature = "windows10")]
    {
        go_to_desktop(&desktops[index])
            .map_err(|e| anyhow::anyhow!("Failed to switch to desktop {index}: {e:?}"))?;
    }

    #[cfg(feature = "windows11")]
    {
        switch_desktop(desktops[index].clone())
            .map_err(|e| anyhow::anyhow!("Failed to switch to desktop {index}: {e:?}"))?;
    }

    Ok(())
}

// ── Windows message pump ─────────────────────────────────────────────

/// Pumps the Win32 message queue (required for tray icon and COM).
///
/// Returns `false` when `WM_QUIT` is received, signalling the main loop
/// to exit.
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

// ── Rect ─────────────────────────────────────────────────────────────

/// Platform-independent rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    fn from_win_rect(r: &RECT) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

// ── MonitorInfo ──────────────────────────────────────────────────────

/// Information about a single display monitor.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub handle: HMONITOR,
    /// Full monitor rectangle (including taskbar area).
    pub rect: Rect,
    /// Usable work area (excluding taskbar).
    pub work_area: Rect,
    /// Device name, e.g. `\\.\DISPLAY1`.
    pub name: String,
}

/// Horizontal direction for cross-monitor navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

// ── Monitor enumeration ──────────────────────────────────────────────

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprc_clip: *mut RECT,
    dw_data: LPARAM,
) -> BOOL {
    unsafe {
        let monitors = &mut *(dw_data.0 as *mut Vec<MonitorInfo>);

        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(
            hmonitor,
            &mut info as *mut MONITORINFOEXW as *mut MONITORINFO,
        )
        .as_bool()
        {
            let name_end = info
                .szDevice
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(info.szDevice.len());
            let name = String::from_utf16_lossy(&info.szDevice[..name_end]);

            monitors.push(MonitorInfo {
                handle: hmonitor,
                rect: Rect::from_win_rect(&info.monitorInfo.rcMonitor),
                work_area: Rect::from_win_rect(&info.monitorInfo.rcWork),
                name,
            });
        }

        BOOL(1) // continue enumeration
    }
}

/// Enumerates all display monitors, sorted left-to-right by x coordinate.
pub fn enum_monitors() -> Result<Vec<MonitorInfo>> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();
    let ptr = &mut monitors as *mut Vec<MonitorInfo>;

    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(monitor_enum_proc),
            LPARAM(ptr as isize),
        );
    }

    monitors.sort_by_key(|m| m.rect.left);
    Ok(monitors)
}

/// Returns monitor information for the monitor containing `hwnd`.
pub fn get_monitor_for_window(hwnd: HWND) -> Option<MonitorInfo> {
    unsafe {
        let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);

        let mut info: MONITORINFOEXW = std::mem::zeroed();
        info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;

        if GetMonitorInfoW(
            hmonitor,
            &mut info as *mut MONITORINFOEXW as *mut MONITORINFO,
        )
        .as_bool()
        {
            let name_end = info
                .szDevice
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(info.szDevice.len());
            let name = String::from_utf16_lossy(&info.szDevice[..name_end]);

            Some(MonitorInfo {
                handle: hmonitor,
                rect: Rect::from_win_rect(&info.monitorInfo.rcMonitor),
                work_area: Rect::from_win_rect(&info.monitorInfo.rcWork),
                name,
            })
        } else {
            None
        }
    }
}

/// Returns the effective DPI for a monitor.
///
/// Falls back to 96 (100% scaling) if the query fails.
pub fn get_monitor_dpi(handle: HMONITOR) -> u32 {
    unsafe {
        let mut dpi_x: u32 = 96;
        let mut dpi_y: u32 = 96;
        let _ = GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        dpi_x
    }
}

/// Returns the adjacent monitor in the given direction, if one exists.
pub fn get_adjacent_monitor(current: &MonitorInfo, direction: Direction) -> Option<MonitorInfo> {
    let monitors = enum_monitors().ok()?;
    let current_idx = monitors.iter().position(|m| m.handle == current.handle)?;

    match direction {
        Direction::Left => current_idx.checked_sub(1).map(|i| monitors[i].clone()),
        Direction::Right => {
            if current_idx + 1 < monitors.len() {
                Some(monitors[current_idx + 1].clone())
            } else {
                None
            }
        }
    }
}

// ── Virtual Desktop Tracker ──────────────────────────────────────────

/// Tracks the currently active virtual desktop by polling.
///
/// Call [`check_for_changes`](Self::check_for_changes) on each main-loop
/// iteration to detect desktop switches.
pub struct VirtualDesktopTracker {
    current_index: usize,
    desktop_count: usize,
}

impl VirtualDesktopTracker {
    /// Creates a tracker, detecting the current desktop on start.
    pub fn new() -> Result<Self> {
        let (count, index) = get_desktop_info()?;
        Ok(Self {
            current_index: index,
            desktop_count: count,
        })
    }

    /// Polls for a desktop change. Returns `Some(new_index)` if the active
    /// desktop changed since the last check.
    pub fn check_for_changes(&mut self) -> Option<usize> {
        if let Ok((count, index)) = get_desktop_info() {
            self.desktop_count = count;
            if index != self.current_index {
                self.current_index = index;
                return Some(index);
            }
        }
        None
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn desktop_count(&self) -> usize {
        self.desktop_count
    }
}

// ── Window enumeration ───────────────────────────────────────────────

struct EnumWindowsContext {
    windows: Vec<HWND>,
    manager: IVirtualDesktopManager,
}

unsafe extern "system" fn enum_windows_on_vd_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let ctx = &mut *(lparam.0 as *mut EnumWindowsContext);

        if IsWindowVisible(hwnd).as_bool() {
            #[cfg(feature = "windows10")]
            let on_current = ctx
                .manager
                .IsWindowOnCurrentVirtualDesktop(hwnd)
                .map(|b| b.as_bool())
                .unwrap_or(false);

            #[cfg(feature = "windows11")]
            let on_current = ctx
                .manager
                .IsWindowOnCurrentVirtualDesktop(hwnd)
                .map(|b| b.as_bool())
                .unwrap_or(false);

            if on_current {
                ctx.windows.push(hwnd);
            }
        }

        BOOL(1) // continue enumeration
    }
}

/// Returns all visible windows on the current virtual desktop.
pub fn get_windows_on_current_vd() -> Result<Vec<HWND>> {
    unsafe {
        let manager: IVirtualDesktopManager =
            CoCreateInstance(&CLSID_VIRTUAL_DESKTOP_MANAGER, None, CLSCTX_ALL)
                .map_err(|e| anyhow::anyhow!("Failed to create VirtualDesktopManager: {e}"))?;

        let mut ctx = EnumWindowsContext {
            windows: Vec::new(),
            manager,
        };

        let ctx_ptr = &mut ctx as *mut EnumWindowsContext;
        let _ = EnumWindows(Some(enum_windows_on_vd_proc), LPARAM(ctx_ptr as isize));

        Ok(ctx.windows)
    }
}

/// Returns whether a single window is on the current virtual desktop.
pub fn is_window_on_current_vd(hwnd: HWND) -> bool {
    unsafe {
        let manager: IVirtualDesktopManager =
            match CoCreateInstance(&CLSID_VIRTUAL_DESKTOP_MANAGER, None, CLSCTX_ALL) {
                Ok(m) => m,
                Err(_) => return false,
            };

        #[cfg(feature = "windows10")]
        {
            manager
                .IsWindowOnCurrentVirtualDesktop(hwnd)
                .map(|b| b.as_bool())
                .unwrap_or(false)
        }

        #[cfg(feature = "windows11")]
        {
            manager
                .IsWindowOnCurrentVirtualDesktop(hwnd)
                .map(|b| b.as_bool())
                .unwrap_or(false)
        }
    }
}

// ── Window information helpers ───────────────────────────────────────

/// Returns the window title text.
pub fn get_window_title(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf: Vec<u16> = vec![0; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

/// Returns the owning process's executable name (e.g. `notepad.exe`).
pub fn get_window_process_name(hwnd: HWND) -> String {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return String::new();
        }

        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
        let Ok(process) = process else {
            return String::new();
        };

        let mut buf: Vec<u16> = vec![0; 1024];
        let mut size = buf.len() as u32;

        #[cfg(feature = "windows10")]
        let query_ok = {
            use windows_win10::Win32::System::Threading::PROCESS_NAME_FORMAT;
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_FORMAT(0),
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            )
            .as_bool()
        };
        #[cfg(feature = "windows11")]
        let query_ok = {
            use windows_win11::Win32::System::Threading::PROCESS_NAME_FORMAT;
            QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_FORMAT(0),
                PWSTR(buf.as_mut_ptr()),
                &mut size,
            )
            .is_ok()
        };

        let name = if query_ok {
            let full = String::from_utf16_lossy(&buf[..size as usize]);
            full.rsplit(['\\', '/']).next().unwrap_or(&full).to_string()
        } else {
            String::new()
        };

        let _ = CloseHandle(process);
        name
    }
}

/// Returns the window's outer rectangle in screen coordinates (including
/// invisible DWM shadow borders).
pub fn get_window_rect(hwnd: HWND) -> Option<Rect> {
    unsafe {
        let mut rect = RECT::default();

        #[cfg(feature = "windows10")]
        let ok = GetWindowRect(hwnd, &mut rect).as_bool();
        #[cfg(feature = "windows11")]
        let ok = GetWindowRect(hwnd, &mut rect).is_ok();

        if ok {
            Some(Rect::from_win_rect(&rect))
        } else {
            None
        }
    }
}

/// Returns the window's *visible* rectangle (excluding invisible DWM shadow
/// borders) in screen coordinates.
///
/// Falls back to `get_window_rect` if the DWM query fails.
pub fn get_visible_window_rect(hwnd: HWND) -> Option<Rect> {
    unsafe {
        let mut rect = RECT::default();
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut RECT as *mut _,
            std::mem::size_of::<RECT>() as u32,
        );

        #[cfg(feature = "windows10")]
        let ok = hr.is_ok();
        #[cfg(feature = "windows11")]
        let ok = hr.is_ok();

        if ok {
            Some(Rect::from_win_rect(&rect))
        } else {
            get_window_rect(hwnd)
        }
    }
}

/// Whether the window is minimized (iconic).
pub fn is_window_minimized(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).as_bool() }
}

/// Whether tiling must preserve the window's current geometry.
///
/// Windows exposes maximized state directly. Borderless fullscreen windows
/// are detected by matching their outer rectangle to the full monitor area.
pub fn is_window_maximized_or_fullscreen(hwnd: HWND) -> bool {
    if unsafe { IsZoomed(hwnd).as_bool() } {
        return true;
    }

    let Some(window_rect) = get_window_rect(hwnd) else {
        return false;
    };
    let Some(monitor) = get_monitor_for_window(hwnd) else {
        return false;
    };

    window_rect == monitor.rect
}

/// Whether the window still exists.
pub fn is_window(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd).as_bool() }
}

/// Whether the window is visible.
pub fn is_window_visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() }
}

/// Returns the window's class name (e.g. `CabinetWClass`).
pub fn get_window_class_name(hwnd: HWND) -> String {
    #[cfg(feature = "windows10")]
    use windows_win10::Win32::UI::WindowsAndMessaging::GetClassNameW;
    #[cfg(feature = "windows11")]
    use windows_win11::Win32::UI::WindowsAndMessaging::GetClassNameW;

    unsafe {
        let mut buf: Vec<u16> = vec![0; 256];
        let len = GetClassNameW(hwnd, &mut buf);
        if len <= 0 {
            return String::new();
        }
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

/// Returns the DPI of the display the window is on. Falls back to 96.
pub fn get_dpi_for_window(hwnd: HWND) -> u32 {
    #[cfg(feature = "windows10")]
    use windows_win10::Win32::UI::HiDpi::GetDpiForWindow;
    #[cfg(feature = "windows11")]
    use windows_win11::Win32::UI::HiDpi::GetDpiForWindow;

    unsafe {
        let dpi = GetDpiForWindow(hwnd);
        if dpi == 0 { 96 } else { dpi }
    }
}

/// Returns the visible frame bounds (DWM extended frame bounds), falling
/// back to the outer window rect on failure.
pub fn get_extended_frame_bounds(hwnd: HWND) -> Option<Rect> {
    get_visible_window_rect(hwnd)
}

/// Whether a window should be considered for tiling.
///
/// A tileable window is visible, not minimized, has no owner, is not a
/// tool window, and is resizable (`WS_THICKFRAME`), maximized, or fullscreen.
pub fn is_window_tileable(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        if IsIconic(hwnd).as_bool() {
            return false;
        }
        // Skip child/owned windows.
        #[cfg(feature = "windows10")]
        {
            if GetParent(hwnd).0 != 0 {
                return false;
            }
        }
        #[cfg(feature = "windows11")]
        {
            if GetParent(hwnd).map(|p| !p.0.is_null()).unwrap_or(false) {
                return false;
            }
        }
        // Skip tool windows (e.g. floating palettes).
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex_style & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }
        // Keep maximized and borderless fullscreen windows tracked even when
        // they temporarily lack the normal resizable-window style.
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        style & WS_THICKFRAME.0 != 0 || is_window_maximized_or_fullscreen(hwnd)
    }
}

// ── Window manipulation ──────────────────────────────────────────────

/// Returns the invisible border thickness (DWM shadow borders) of a window.
///
/// Windows renders invisible borders around most windows for the drop-shadow
/// effect. `GetWindowRect` includes these borders, but they are not visible.
/// This function computes the difference between the outer rect (from
/// `GetWindowRect`) and the visible rect (from `DWMWA_EXTENDED_FRAME_BOUNDS`)
/// so callers can compensate when positioning windows.
///
/// Returns `(left, top, right, bottom)` border widths in pixels.
fn get_invisible_borders(hwnd: HWND) -> (i32, i32, i32, i32) {
    unsafe {
        let mut outer = RECT::default();
        #[cfg(feature = "windows10")]
        let outer_ok = GetWindowRect(hwnd, &mut outer).as_bool();
        #[cfg(feature = "windows11")]
        let outer_ok = GetWindowRect(hwnd, &mut outer).is_ok();

        if !outer_ok {
            return (0, 0, 0, 0);
        }

        let mut visible = RECT::default();
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut visible as *mut RECT as *mut _,
            std::mem::size_of::<RECT>() as u32,
        );

        #[cfg(feature = "windows10")]
        let dwm_ok = hr.is_ok();
        #[cfg(feature = "windows11")]
        let dwm_ok = hr.is_ok();

        if !dwm_ok {
            return (0, 0, 0, 0);
        }

        (
            visible.left - outer.left,
            visible.top - outer.top,
            outer.right - visible.right,
            outer.bottom - visible.bottom,
        )
    }
}

/// Moves and resizes a window without changing its z-order or activation.
///
/// Compensates for invisible DWM shadow borders so the *visible* portion of
/// the window matches the requested rectangle exactly. Without this, the top
/// gap appears smaller than the others because Windows renders asymmetric
/// invisible borders (typically 0px top, ~7px on other sides).
///
/// `SWP_NOSENDCHANGING` suppresses `WM_WINDOWPOSCHANGING`/`WM_WINDOWPOSCHANGED`
/// messages so the target window does not perform its own intermediate resize
/// in response to the move — this prevents a visible double-resize when moving
/// between monitors with different DPI settings.
pub fn set_window_pos(hwnd: HWND, rect: Rect) -> bool {
    let (bl, bt, br, bb) = get_invisible_borders(hwnd);

    let adjusted = Rect {
        left: rect.left - bl,
        top: rect.top - bt,
        right: rect.right + br,
        bottom: rect.bottom + bb,
    };

    unsafe {
        #[cfg(feature = "windows10")]
        {
            SetWindowPos(
                hwnd,
                None,
                adjusted.left,
                adjusted.top,
                adjusted.width(),
                adjusted.height(),
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
            )
            .as_bool()
        }
        #[cfg(feature = "windows11")]
        {
            SetWindowPos(
                hwnd,
                None,
                adjusted.left,
                adjusted.top,
                adjusted.width(),
                adjusted.height(),
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING,
            )
            .is_ok()
        }
    }
}

// ── WinEvent hook ────────────────────────────────────────────────────

/// High-level window lifecycle events surfaced by the WinEvent hook.
///
/// Uses a raw pointer wrapper so the event can be sent across threads (HWND is
/// `*mut c_void` which is not `Send`/`Sync` in windows 0.58).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    Created(WindowHandle),
    Destroyed(WindowHandle),
    Minimized(WindowHandle),
    Restored(WindowHandle),
    Moved(WindowHandle),
    ForegroundChanged(WindowHandle),
}

/// A thread-safe wrapper around a raw window handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowHandle(pub usize);

impl WindowHandle {
    pub fn from_hwnd(hwnd: HWND) -> Self {
        #[cfg(feature = "windows10")]
        {
            Self(hwnd.0 as usize)
        }
        #[cfg(feature = "windows11")]
        {
            Self(hwnd.0 as usize)
        }
    }

    pub fn to_hwnd(self) -> HWND {
        #[cfg(feature = "windows10")]
        {
            HWND(self.0 as isize)
        }
        #[cfg(feature = "windows11")]
        {
            HWND(self.0 as *mut _)
        }
    }
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

static EVENT_SENDER: OnceLock<mpsc::Sender<WindowEvent>> = OnceLock::new();

#[cfg(feature = "windows10")]
type WinEventHook = windows_win10::Win32::UI::Accessibility::HWINEVENTHOOK;
#[cfg(feature = "windows11")]
type WinEventHook = windows_win11::Win32::UI::Accessibility::HWINEVENTHOOK;

#[cfg(feature = "windows10")]
unsafe extern "system" fn win_event_proc(
    _hook: WinEventHook,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    handle_win_event(event, hwnd, id_object);
}

#[cfg(feature = "windows11")]
unsafe extern "system" fn win_event_proc(
    _hook: WinEventHook,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    handle_win_event(event, hwnd, id_object);
}

fn handle_win_event(event: u32, hwnd: HWND, id_object: i32) {
    // Only care about the window object itself (OBJID_WINDOW == 0).
    if id_object != 0 {
        return;
    }
    let Some(sender) = EVENT_SENDER.get() else {
        return;
    };

    let handle = WindowHandle::from_hwnd(hwnd);

    let evt = if event == EVENT_OBJECT_CREATE {
        WindowEvent::Created(handle)
    } else if event == EVENT_OBJECT_DESTROY {
        WindowEvent::Destroyed(handle)
    } else if event == EVENT_SYSTEM_MINIMIZESTART {
        WindowEvent::Minimized(handle)
    } else if event == EVENT_SYSTEM_MINIMIZEEND {
        WindowEvent::Restored(handle)
    } else if event == EVENT_SYSTEM_MOVESIZEEND {
        WindowEvent::Moved(handle)
    } else if event == EVENT_SYSTEM_FOREGROUND {
        WindowEvent::ForegroundChanged(handle)
    } else {
        return;
    };

    let _ = sender.send(evt);
}

/// Spawns a dedicated thread that installs global WinEvent hooks and runs a
/// message loop to deliver window lifecycle events.
///
/// Returns a receiver that should be polled on the main loop. The hook
/// callbacks require a message loop, which runs on the spawned thread.
pub fn start_window_event_listener() -> Result<mpsc::Receiver<WindowEvent>> {
    let (tx, rx) = mpsc::channel();
    EVENT_SENDER
        .set(tx)
        .map_err(|_| anyhow::anyhow!("Window event listener already started"))?;

    std::thread::Builder::new()
        .name("tilosrv-winevent".to_string())
        .spawn(|| unsafe {
            let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;

            let h1 = SetWinEventHook(
                EVENT_OBJECT_CREATE,
                EVENT_OBJECT_DESTROY,
                None,
                Some(win_event_proc),
                0,
                0,
                flags,
            );
            let h2 = SetWinEventHook(
                EVENT_SYSTEM_MINIMIZESTART,
                EVENT_SYSTEM_MOVESIZEEND,
                None,
                Some(win_event_proc),
                0,
                0,
                flags,
            );

            let h3 = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(win_event_proc),
                0,
                0,
                flags,
            );

            if h1.is_invalid() || h2.is_invalid() || h3.is_invalid() {
                eprintln!("Failed to install WinEvent hooks");
                return;
            }

            run_event_message_loop();
        })
        .context("Failed to spawn WinEvent listener thread")?;

    Ok(rx)
}

/// Runs a blocking Win32 message loop (call on a dedicated thread).
///
/// Required so that out-of-context WinEvent hooks can deliver callbacks.
pub fn run_event_message_loop() {
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
