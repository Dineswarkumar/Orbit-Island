use tauri::WebviewWindow;

#[cfg(target_os = "windows")]
pub fn enforce_frameless(window: &WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMNCRP_DISABLED, DWMWA_NCRENDERING_POLICY,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetWindowLongW, SetWindowPos, SetWindowTextW, GWL_EXSTYLE, GWL_STYLE,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_BORDER,
        WS_CAPTION, WS_DLGFRAME, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_STATICEDGE,
        WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
    };

    let _ = window.set_decorations(false);
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;

    unsafe {
        // 1. Clear window title text on the native Win32 HWND so the OS never paints "ORBIT Island"
        let empty_title: [u16; 1] = [0];
        SetWindowTextW(hwnd.0 as HWND, empty_title.as_ptr());

        // 2. Strip all decorative non-client styles and enforce WS_POPUP
        let current_style = GetWindowLongW(hwnd.0 as HWND, GWL_STYLE);
        let stripped_style = (current_style | WS_POPUP as i32)
            & !(WS_CAPTION as i32
                | WS_THICKFRAME as i32
                | WS_MINIMIZEBOX as i32
                | WS_MAXIMIZEBOX as i32
                | WS_SYSMENU as i32
                | WS_BORDER as i32
                | WS_DLGFRAME as i32);
        if current_style != stripped_style {
            SetWindowLongW(hwnd.0 as HWND, GWL_STYLE, stripped_style);
        }

        // 3. Strip all 3D / dialog frame extended styles
        let current_ex_style = GetWindowLongW(hwnd.0 as HWND, GWL_EXSTYLE);
        let stripped_ex_style = current_ex_style
            & !(WS_EX_WINDOWEDGE as i32
                | WS_EX_CLIENTEDGE as i32
                | WS_EX_STATICEDGE as i32
                | WS_EX_DLGMODALFRAME as i32);
        if current_ex_style != stripped_ex_style {
            SetWindowLongW(hwnd.0 as HWND, GWL_EXSTYLE, stripped_ex_style);
        }

        // 4. Instruct DWM to completely disable non-client rendering (no caption, frame, or title strip)
        let nc_policy = DWMNCRP_DISABLED;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as HWND,
            DWMWA_NCRENDERING_POLICY as u32,
            &nc_policy as *const _ as *const _,
            std::mem::size_of_val(&nc_policy) as u32,
        );

        let corner_pref = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as HWND,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            &corner_pref as *const _ as *const _,
            std::mem::size_of_val(&corner_pref) as u32,
        );

        // Explicitly suppress native Windows 11 DWM window frame borders
        // 34 = DWMWA_BORDER_COLOR, 35 = DWMWA_CAPTION_COLOR, 36 = DWMWA_TEXT_COLOR
        // 0xFFFFFFFE = DWMWA_COLOR_NONE (no border rendered by DWM)
        let color_none = 0xFFFFFFFEu32;
        let _ = DwmSetWindowAttribute(
            hwnd.0 as HWND,
            34,
            &color_none as *const _ as *const _,
            std::mem::size_of_val(&color_none) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd.0 as HWND,
            35,
            &color_none as *const _ as *const _,
            std::mem::size_of_val(&color_none) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd.0 as HWND,
            36,
            &color_none as *const _ as *const _,
            std::mem::size_of_val(&color_none) as u32,
        );

        // 5. Notify the window of the frame change ONCE during startup initialization
        SetWindowPos(
            hwnd.0 as HWND,
            0 as HWND,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_NOACTIVATE,
        );

        // 6. Install window subclass dynamically to suppress non-client painting, deactivation borders, and GDI background erase
        install_island_subclass(hwnd.0 as HWND);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn install_island_subclass(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
    unsafe {
        let comctl32 = LoadLibraryA(b"comctl32.dll\0".as_ptr());
        if !comctl32.is_null() {
            let set_subclass_fn = GetProcAddress(comctl32, b"SetWindowSubclass\0".as_ptr());
            if let Some(set_subclass) = set_subclass_fn {
                type SetWindowSubclassFn = unsafe extern "system" fn(
                    windows_sys::Win32::Foundation::HWND,
                    unsafe extern "system" fn(
                        windows_sys::Win32::Foundation::HWND,
                        u32,
                        usize,
                        isize,
                        usize,
                        usize,
                    ) -> isize,
                    usize,
                    usize,
                ) -> i32;
                let func: SetWindowSubclassFn = std::mem::transmute(set_subclass);
                const SUBCLASS_ID: usize = 0x08B17; // "ORBIT"
                func(hwnd, island_subclass_proc, SUBCLASS_ID, 0);
            }
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn island_subclass_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _uid_subclass: usize,
    _ref_data: usize,
) -> isize {
    const WM_NCACTIVATE: u32 = 0x0086;
    const WM_NCPAINT: u32 = 0x0085;
    const WM_ERASEBKGND: u32 = 0x0014;

    match msg {
        // Prevent Windows from rendering default active/inactive title bar and caption frame upon focus changes
        WM_NCACTIVATE => 1,
        // Suppress any native window non-client frame borders or GDI frame repainting
        WM_NCPAINT => 0,
        // Suppress GDI background erasing (which paints opaque black/gray over transparent swapchains)
        WM_ERASEBKGND => 1,
        _ => unsafe {
            use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
            let comctl32 = GetModuleHandleA(b"comctl32.dll\0".as_ptr());
            if !comctl32.is_null() {
                let def_fn = GetProcAddress(comctl32, b"DefSubclassProc\0".as_ptr());
                if let Some(def_proc) = def_fn {
                    type DefSubclassProcFn = unsafe extern "system" fn(
                        windows_sys::Win32::Foundation::HWND,
                        u32,
                        usize,
                        isize,
                    ) -> isize;
                    let func: DefSubclassProcFn = std::mem::transmute(def_proc);
                    return func(hwnd, msg, wparam, lparam);
                }
            }
            windows_sys::Win32::UI::WindowsAndMessaging::DefWindowProcW(hwnd, msg, wparam, lparam)
        },
    }
}

#[cfg(not(target_os = "windows"))]
pub fn enforce_frameless(window: &WebviewWindow) -> Result<(), String> {
    let _ = window.set_decorations(false);
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn position_island_window<R: tauri::Runtime>(window: &WebviewWindow<R>, position: &str) -> Result<(), String> {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let monitor = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Primary monitor not found".to_string())?;
    let scale = monitor.scale_factor();
    let screen_width = monitor.size().width as f64 / scale;

    // Stable 400x188 canvas positioned at top-left, top-center, or top-right (y = 10.0)
    let x = match position {
        "top-left" => 24.0,
        "top-right" => (screen_width - 400.0 - 24.0).max(0.0),
        _ => (screen_width - 400.0) / 2.0,
    };
    let y = 10.0;

    let x_px = (x * scale).round() as i32;
    let y_px = (y * scale).round() as i32;
    let width_px = (400.0 * scale).round() as i32;
    let height_px = (188.0 * scale).round() as i32;

    let pos_result = unsafe {
        SetWindowPos(
            hwnd.0 as HWND,
            0 as HWND,
            x_px,
            y_px,
            width_px,
            height_px,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
    if pos_result == 0 {
        return Err("Windows failed to update island window position and size".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn position_island_window<R: tauri::Runtime>(window: &WebviewWindow<R>, position: &str) -> Result<(), String> {
    let monitor = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Primary monitor not found".to_string())?;
    let scale = monitor.scale_factor();
    let screen_width = monitor.size().width as f64 / scale;
    let x = match position {
        "top-left" => 24.0,
        "top-right" => (screen_width - 400.0 - 24.0).max(0.0),
        _ => (screen_width - 400.0) / 2.0,
    };
    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y: 10.0 }))
        .map_err(|e| e.to_string())?;
    window
        .set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: 400.0,
            height: 188.0,
        }))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn init_island_window(window: &WebviewWindow, position: &str) -> Result<(), String> {
    enforce_frameless(window)?;
    position_island_window(window, position)?;
    apply_island_region(window, "compact")?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn init_island_window(window: &WebviewWindow, position: &str) -> Result<(), String> {
    enforce_frameless(window)?;
    position_island_window(window, position)?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn apply_island_region(window: &WebviewWindow, state: &str) -> Result<(), String> {
    // CRITICAL: DO NOT call enforce_frameless, set_decorations, or SetWindowPos(SWP_FRAMECHANGED) here.
    // Non-client frame styles are established once at startup in init_island_window.
    // Calling frame recalculation during runtime transitions causes DWM to momentarily expose
    // the Win32 non-client caption bar and a bright horizontal strip along the top edge.

    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateRoundRectRgn, DeleteObject, HGDIOBJ, HRGN, SetWindowRgn,
    };

    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let monitor = window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Primary monitor not found".to_string())?;
    let scale = monitor.scale_factor();

    // Give a 4px breathing clearance around the visual 3D shell so that:
    // 1. DirectComposition/WebView2 renders smooth subpixel anti-aliased 3D borders without
    //    1-bit GDI Bresenham clipping chewing into the capsule rim.
    // 2. 3D perspective tilt (rotateX/rotateY), spring overshoot, and outer 3D rim depth
    //    never get guillotined at the boundary, while keeping outside space and corners click-through.
    let (x1_f, y1_f, x2_f, y2_f, corner_f) = match state {
        "compact" | "compact_alert" => (126.0, 0.0, 274.0, 48.0, 46.0),
        "notification" => (36.0, 0.0, 364.0, 74.0, 58.0),
        "expanded" => (16.0, 0.0, 384.0, 180.0, 56.0),
        _ => return Err(format!("Unsupported island state for region: {state}")),
    };

    let x1 = (x1_f * scale).round() as i32;
    let y1 = (y1_f * scale).round() as i32;
    let x2 = (x2_f * scale).round() as i32 + 1;
    let y2 = (y2_f * scale).round() as i32 + 1;
    let corner_diameter = (corner_f * scale).round() as i32;

    let region = unsafe {
        CreateRoundRectRgn(
            x1,
            y1,
            x2,
            y2,
            corner_diameter,
            corner_diameter,
        )
    };
    if region.is_null() {
        return Err("Windows failed to create the island window region".to_string());
    }

    // SetWindowRgn with bRedraw = 1 so Windows informs DWM to invalidate the desktop area
    // exposed by shrinking the region. Non-client repainting and GDI background erases
    // are safely suppressed by island_subclass_proc, so no GDI flash or title artifact occurs.
    let result = unsafe { SetWindowRgn(hwnd.0 as HWND, region as HRGN, 1) };
    if result == 0 {
        unsafe { DeleteObject(region as HGDIOBJ) };
        return Err("Windows failed to update the island window region".to_string());
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn apply_island_region(_window: &WebviewWindow, _state: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_island_geometry(window: &WebviewWindow, width: f64, _height: f64) -> Result<(), String> {
    let state = if width > 310.0 {
        "expanded"
    } else if width > 200.0 {
        "notification"
    } else {
        "compact"
    };
    apply_island_region(window, state)
}

#[cfg(not(target_os = "windows"))]
pub fn set_island_geometry(window: &WebviewWindow, _width: f64, _height: f64) -> Result<(), String> {
    enforce_frameless(window)?;
    Ok(())
}

/// Asks the Windows shell whether a fullscreen app (D3D game, browser/video fullscreen,
/// presentation) currently owns the foreground. Chromium-based browsers report
/// `QUNS_BUSY` when a tab/video goes fullscreen, which is the most reliable signal.
#[cfg(target_os = "windows")]
fn shell_reports_fullscreen_app() -> bool {
    use windows_sys::Win32::UI::Shell::SHQueryUserNotificationState;

    // QUNS_BUSY = 2, QUNS_RUNNING_D3D_FULL_SCREEN = 3, QUNS_PRESENTATION_MODE = 4
    let mut state: i32 = 0;
    let hr = unsafe { SHQueryUserNotificationState(&mut state) };
    hr >= 0 && matches!(state, 2 | 3 | 4)
}

#[cfg(target_os = "windows")]
pub fn is_foreground_fullscreen(island_hwnd: isize) -> bool {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowLongW, GetWindowRect, GWL_STYLE, WS_CAPTION,
        WS_THICKFRAME,
    };

    if island_hwnd == 0 {
        return false;
    }

    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.is_null() || fg_hwnd == (island_hwnd as HWND) {
            return false;
        }

        // Filter out Windows shell desktop windows
        let mut class_name = [0u16; 256];
        let len = GetClassNameW(fg_hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
        if len > 0 {
            let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
            if class_str == "Progman" || class_str == "WorkerW" || class_str == "Shell_TrayWnd" {
                return false;
            }
        }

        // Signal 1: the shell itself says a fullscreen app is running.
        if shell_reports_fullscreen_app() {
            return true;
        }

        // Signal 2: geometry. A normal window (even maximized) keeps its title bar / sizing
        // frame, so it is NOT fullscreen media. NOTE: we must not treat WS_MAXIMIZE as
        // "normal": Chromium keeps WS_MAXIMIZE while a video is fullscreen and only strips
        // the caption and frame styles.
        let style = GetWindowLongW(fg_hwnd, GWL_STYLE) as u32;
        let has_caption = (style & (WS_CAPTION as u32)) == (WS_CAPTION as u32);
        let has_frame = (style & (WS_THICKFRAME as u32)) != 0;
        if has_caption || has_frame {
            return false;
        }

        let mut fg_rect: RECT = std::mem::zeroed();
        if GetWindowRect(fg_hwnd, &mut fg_rect) == 0 {
            return false;
        }

        let hmon = MonitorFromWindow(island_hwnd as HWND, MONITOR_DEFAULTTONEAREST);
        if hmon.is_null() {
            return false;
        }

        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmon, &mut mi) == 0 {
            return false;
        }

        let mon = mi.rcMonitor;
        fg_rect.left <= mon.left
            && fg_rect.top <= mon.top
            && fg_rect.right >= mon.right
            && fg_rect.bottom >= mon.bottom
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_foreground_fullscreen(_island_hwnd: isize) -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn set_window_topmost<R: tauri::Runtime>(window: &WebviewWindow<R>, topmost: bool) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE,
        SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST,
    };
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            // Re-issuing HWND_TOPMOST on an already-topmost window moves it to the top of the
            // topmost band, i.e. ABOVE popup menus that were created after it. Skip the call
            // when the state already matches so right-click menus stay on top of the island.
            let is_topmost = (GetWindowLongW(hwnd.0 as HWND, GWL_EXSTYLE) as u32
                & (WS_EX_TOPMOST as u32))
                != 0;
            if is_topmost == topmost {
                return;
            }
            let insert_after = if topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
            SetWindowPos(
                hwnd.0 as HWND,
                insert_after,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_window_topmost<R: tauri::Runtime>(window: &WebviewWindow<R>, topmost: bool) {
    let _ = window.set_always_on_top(topmost);
}

pub fn ensure_window_on_screen<R: tauri::Runtime>(
    window: &WebviewWindow<R>,
    position: &str,
) -> Result<(), String> {
    let monitor = match window.current_monitor().map_err(|e| e.to_string())? {
        Some(m) => m,
        None => match window.primary_monitor().map_err(|e| e.to_string())? {
            Some(m) => m,
            None => return Err("No monitor found".to_string()),
        },
    };

    let mon_pos = monitor.position();
    let mon_size = monitor.size();

    let win_pos = window.outer_position().map_err(|e| e.to_string())?;

    let min_x = mon_pos.x;
    let max_x = mon_pos.x + mon_size.width as i32;
    let min_y = mon_pos.y;
    let max_y = mon_pos.y + mon_size.height as i32;

    if win_pos.x < min_x || win_pos.x >= max_x || win_pos.y < min_y || win_pos.y >= max_y {
        position_island_window(window, position)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_check_handles_zero_hwnd_safely() {
        assert!(!is_foreground_fullscreen(0));
    }
}
