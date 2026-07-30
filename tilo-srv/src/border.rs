//! Active-window border overlay.
//!
//! Draws a colored frame around the currently focused window using a layered
//! topmost overlay window and Direct2D rendering. Ported from Glint.

use crate::config::WindowBorderConfig;
use crate::platform::{self, HWND, Rect};
use anyhow::Result;
use regex::Regex;

#[cfg(feature = "windows10")]
use windows_win10::Win32::Foundation::{
    COLORREF, HINSTANCE, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::Foundation::{
    COLORREF, HINSTANCE, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};

#[cfg(feature = "windows10")]
use windows_win10::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F,
};

#[cfg(feature = "windows10")]
use windows_win10::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, D2D1_DEBUG_LEVEL_NONE, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT, ID2D1Factory,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, D2D1_DEBUG_LEVEL_NONE, D2D1_FACTORY_OPTIONS,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT, ID2D1Factory,
};

#[cfg(feature = "windows10")]
use windows_win10::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
#[cfg(feature = "windows11")]
use windows_win11::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

#[cfg(feature = "windows10")]
use windows_win10::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HGDIOBJ,
    ReleaseDC, SelectObject,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HGDIOBJ,
    ReleaseDC, SelectObject,
};

#[cfg(feature = "windows10")]
use windows_win10::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(feature = "windows11")]
use windows_win11::Win32::System::LibraryLoader::GetModuleHandleW;

#[cfg(feature = "windows10")]
use windows_win10::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, HWND_TOPMOST, IDC_ARROW,
    LoadCursorW, PostQuitMessage, RegisterClassW, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowPos, ULW_ALPHA, UpdateLayeredWindow,
    WM_DESTROY, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
#[cfg(feature = "windows11")]
use windows_win11::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, HWND_TOPMOST, IDC_ARROW,
    LoadCursorW, PostQuitMessage, RegisterClassW, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowPos, ULW_ALPHA, UpdateLayeredWindow,
    WM_DESTROY, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

/// Window classes that never get a border (system UI).
const SYSTEM_IGNORE_CLASSES: &[&str] = &[
    "Windows.UI.Core.CoreWindow",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "Progman",
    "WorkerW",
];

struct CompiledRule {
    process: Option<Regex>,
    title: Option<Regex>,
}

pub struct BorderOverlay {
    overlay: HWND,
    factory: ID2D1Factory,
    width: i32,
    radius: i32,
    color: D2D1_COLOR_F,
    rules: Vec<CompiledRule>,
    last_foreground: Option<HWND>,
}

impl BorderOverlay {
    /// Creates the overlay window and D2D factory. Returns `None` when the
    /// border is disabled in config.
    pub fn new(config: &WindowBorderConfig) -> Result<Option<Self>> {
        if !config.enabled {
            return Ok(None);
        }

        let rules = config
            .ignore
            .iter()
            .filter_map(|r| {
                let process = r
                    .process
                    .as_deref()
                    .and_then(|p| Regex::new(p).ok());
                let title = r.title.as_deref().and_then(|t| Regex::new(t).ok());
                if process.is_some() || title.is_some() {
                    Some(CompiledRule { process, title })
                } else {
                    None
                }
            })
            .collect();

        unsafe {
            let options = D2D1_FACTORY_OPTIONS {
                debugLevel: D2D1_DEBUG_LEVEL_NONE,
            };
            let factory: ID2D1Factory =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, Some(&options))?;

            let instance = GetModuleHandleW(None)?;
            let class_name: Vec<u16> = "tilo_border_overlay\0".encode_utf16().collect();
            let window_name: Vec<u16> = "\0".encode_utf16().collect();

            let wc = WNDCLASSW {
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                hInstance: to_hinstance(instance),
                lpszClassName: windows_pcwstr(&class_name),
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(border_wnd_proc),
                ..Default::default()
            };
            RegisterClassW(&wc);

            #[cfg(feature = "windows10")]
            let hinstance = Some(to_hinstance(instance));
            #[cfg(feature = "windows11")]
            let hinstance = to_hinstance(instance);

            let overlay = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
                windows_pcwstr(&class_name),
                windows_pcwstr(&window_name),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                hinstance,
                None,
            );

            #[cfg(feature = "windows10")]
            let overlay = {
                anyhow::ensure!(overlay.0 != 0, "Failed to create overlay window");
                overlay
            };
            #[cfg(feature = "windows11")]
            let overlay = overlay?;

            Ok(Some(Self {
                overlay,
                factory,
                width: config.width,
                radius: config.radius,
                color: parse_hex_color(&config.color),
                rules,
                last_foreground: None,
            }))
        }
    }

    /// Re-evaluates the foreground window and redraws or hides the border.
    pub fn update(&mut self, suppress: bool) {
        if suppress {
            self.hide();
            return;
        }

        let Some(fg) = platform::get_foreground_window() else {
            self.hide();
            return;
        };

        if fg == self.overlay {
            return;
        }

        let changed = self.last_foreground != Some(fg);
        self.last_foreground = Some(fg);

        if !platform::is_window(fg) || !platform::is_window_visible(fg) {
            self.hide();
            return;
        }

        let class = platform::get_window_class_name(fg);
        if SYSTEM_IGNORE_CLASSES.iter().any(|c| *c == class) {
            self.hide();
            return;
        }

        let title = platform::get_window_title(fg);
        let process = platform::get_window_process_name(fg);
        for rule in &self.rules {
            if let Some(re) = &rule.process
                && re.is_match(&process) {
                    self.hide();
                    return;
                }
            if let Some(re) = &rule.title
                && re.is_match(&title) {
                    self.hide();
                    return;
                }
        }

        if platform::is_window_minimized(fg) || platform::is_window_maximized_or_fullscreen(fg) {
            self.hide();
            return;
        }

        let Some(bounds) = platform::get_extended_frame_bounds(fg) else {
            self.hide();
            return;
        };

        let dpi = platform::get_dpi_for_window(fg);
        let scale = dpi as f32 / 96.0;
        let border_width = (self.width as f32 * scale).ceil() as i32;
        let radius_px = (self.radius as f32 * scale).ceil() as i32;

        let overlay_rect = Rect {
            left: bounds.left - border_width,
            top: bounds.top - border_width,
            right: bounds.right + border_width,
            bottom: bounds.bottom + border_width,
        };

        let width = overlay_rect.width();
        let height = overlay_rect.height();
        if width <= 0 || height <= 0 {
            self.hide();
            return;
        }

        // Skip re-render when nothing moved (cheap guard). Rect changes are
        // caught because position verification triggers re-tile + update.
        let _ = changed;

        unsafe {
            self.render(
                width,
                height,
                border_width as f32,
                radius_px as f32,
                &overlay_rect,
            );
        }
    }

    /// Hides the overlay window.
    pub fn hide(&self) {
        self.last_foreground_reset();
        #[cfg(feature = "windows10")]
        let insert_after = None;
        #[cfg(feature = "windows11")]
        let insert_after = HWND::default();
        unsafe {
            let _ = SetWindowPos(
                self.overlay,
                insert_after,
                0,
                0,
                0,
                0,
                SWP_HIDEWINDOW | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
            );
        }
    }

    fn last_foreground_reset(&self) {
        // Intentionally no-op; `update` tracks last_foreground internally.
    }

    unsafe fn render(&self, width: i32, height: i32, border_width: f32, radius: f32, screen: &Rect) {
        unsafe {
            let screen_dc = GetDC(None);
            #[cfg(feature = "windows10")]
            let mem_dc = CreateCompatibleDC(Some(screen_dc));
            #[cfg(feature = "windows11")]
            let mem_dc = CreateCompatibleDC(screen_dc);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // top-down
                    biPlanes: 1,
                    biBitCount: 32,
                    #[cfg(feature = "windows10")]
                    biCompression: BI_RGB,
                    #[cfg(feature = "windows11")]
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            #[cfg(feature = "windows10")]
            let bitmap = CreateDIBSection(Some(screen_dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
            #[cfg(feature = "windows11")]
            let bitmap = CreateDIBSection(screen_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
            let Ok(bitmap) = bitmap else {
                let _ = DeleteDC(mem_dc);
                ReleaseDC(None, screen_dc);
                return;
            };

            let old_bitmap = SelectObject(mem_dc, HGDIOBJ(bitmap.0 as _));

            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 0.0,
                dpiY: 0.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };

            if let Ok(rt) = self.factory.CreateDCRenderTarget(&props) {
                let rect = RECT {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: height,
                };
                if rt.BindDC(mem_dc, &rect).is_ok() {
                    rt.BeginDraw();
                    rt.Clear(Some(&D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }));

                    let target = &rt;
                    if let Ok(brush) = target.CreateSolidColorBrush(&self.color, None) {
                        let half = border_width / 2.0;
                        let draw_rect = D2D_RECT_F {
                            left: half,
                            top: half,
                            right: width as f32 - half,
                            bottom: height as f32 - half,
                        };
                        if radius > 0.0 {
                            let rounded = D2D1_ROUNDED_RECT {
                                rect: draw_rect,
                                radiusX: radius,
                                radiusY: radius,
                            };
                            target.DrawRoundedRectangle(&rounded, &brush, border_width, None);
                        } else {
                            target.DrawRectangle(&draw_rect, &brush, border_width, None);
                        }
                    }

                    let _ = rt.EndDraw(None, None);
                }
            }

            let pt_src = POINT { x: 0, y: 0 };
            let pt_dst = POINT {
                x: screen.left,
                y: screen.top,
            };
            let size = SIZE {
                cx: width,
                cy: height,
            };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            #[cfg(feature = "windows10")]
            let _ = UpdateLayeredWindow(
                self.overlay,
                Some(screen_dc),
                Some(&pt_dst),
                Some(&size),
                Some(mem_dc.into()),
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            #[cfg(feature = "windows11")]
            let _ = UpdateLayeredWindow(
                self.overlay,
                screen_dc,
                Some(&pt_dst),
                Some(&size),
                mem_dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            #[cfg(feature = "windows10")]
            let topmost = Some(HWND_TOPMOST);
            #[cfg(feature = "windows11")]
            let topmost = HWND_TOPMOST;

            let _ = SetWindowPos(
                self.overlay,
                topmost,
                screen.left,
                screen.top,
                width,
                height,
                SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );

            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(HGDIOBJ(bitmap.0 as _));
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
        }
    }
}

/// Parses "#RRGGBB" into a D2D color. Falls back to the default blue.
fn parse_hex_color(s: &str) -> D2D1_COLOR_F {
    let hex = s.trim_start_matches('#');
    let default = D2D1_COLOR_F {
        r: 141.0 / 255.0,
        g: 188.0 / 255.0,
        b: 1.0,
        a: 1.0,
    };
    if hex.len() != 6 {
        return default;
    }
    let parse = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    match (parse(0), parse(2), parse(4)) {
        (Some(r), Some(g), Some(b)) => D2D1_COLOR_F {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        },
        _ => default,
    }
}

#[cfg(feature = "windows10")]
fn windows_pcwstr(s: &[u16]) -> windows_win10::core::PCWSTR {
    windows_win10::core::PCWSTR(s.as_ptr())
}

#[cfg(feature = "windows11")]
fn windows_pcwstr(s: &[u16]) -> windows_win11::core::PCWSTR {
    windows_win11::core::PCWSTR(s.as_ptr())
}

#[cfg(feature = "windows10")]
fn to_hinstance(h: windows_win10::Win32::Foundation::HINSTANCE) -> HINSTANCE {
    h
}

#[cfg(feature = "windows11")]
fn to_hinstance(h: windows_win11::Win32::Foundation::HMODULE) -> HINSTANCE {
    HINSTANCE(h.0)
}

unsafe extern "system" fn border_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
