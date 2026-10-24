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
    apply_island_region_sized(window, state, None)
}

/// Same as `apply_island_region`, but for compact-family states an optional logical pill width
/// makes the native hit region hug the visible capsule (centered on the 400px window) instead of
/// using a fixed slab. This keeps wide pills (REC + privacy light + call timer) from being clipped
/// and stops narrow pills from blocking clicks on empty space beside them.
#[cfg(target_os = "windows")]
pub fn apply_island_region_sized(
    window: &WebviewWindow,
    state: &str,
    compact_width: Option<f64>,
) -> Result<(), String> {
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
    // Dynamic compact region: pill width + 8px breathing (4px each side), centered at x=200.
    let dynamic_compact = compact_width.map(|w| {
        let half = (w.clamp(60.0, 380.0) / 2.0) + 4.0;
        (200.0 - half, 0.0, 200.0 + half, 48.0, 46.0)
    });

    let (x1_f, y1_f, x2_f, y2_f, corner_f) = match state {
        "suppressed" => (0.0, 0.0, 0.0, 0.0, 0.0),
        "compact" | "compact_alert" => dynamic_compact.unwrap_or((110.0, 0.0, 290.0, 48.0, 46.0)),
        "compact_dual" => dynamic_compact.unwrap_or((68.0, 0.0, 332.0, 48.0, 46.0)),
        "notification" => (36.0, 0.0, 364.0, 74.0, 56.0),
        "notification_hover" => (10.0, 0.0, 390.0, 132.0, 56.0),
        "expanded" => (16.0, 0.0, 384.0, 180.0, 56.0),
        _ => return Err(format!("Unsupported island state for region: {state}")),
    };

    let (x1, y1, x2, y2, corner_diameter) = if state == "suppressed" {
        (0, 0, 0, 0, 0)
    } else {
        (
            (x1_f * scale).round() as i32,
            (y1_f * scale).round() as i32,
            (x2_f * scale).round() as i32 + 1,
            (y2_f * scale).round() as i32 + 1,
            (corner_f * scale).round() as i32,
        )
    };

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

#[cfg(not(target_os = "windows"))]
pub fn apply_island_region_sized(
    _window: &WebviewWindow,
    _state: &str,
    _compact_width: Option<f64>,
) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_island_geometry(window: &WebviewWindow, width: f64, _height: f64) -> Result<(), String> {
    let state = if width > 330.0 {
        "expanded"
    } else if width > 280.0 {
        "notification"
    } else if width > 200.0 {
        "compact_dual"
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

#[cfg(target_os = "windows")]
pub fn force_window_topmost<R: tauri::Runtime>(window: &WebviewWindow<R>, topmost: bool) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
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
pub fn force_window_topmost<R: tauri::Runtime>(window: &WebviewWindow<R>, topmost: bool) {
    let _ = window.set_always_on_top(topmost);
}

#[cfg(target_os = "windows")]
pub fn set_window_capture_stealth<R: tauri::Runtime>(
    window: &WebviewWindow<R>,
    hide_from_capture: bool,
) {
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::SetWindowDisplayAffinity;

    // WDA_NONE = 0x00000000, WDA_EXCLUDEFROMCAPTURE = 0x00000011 (Windows 10 2004+ / Windows 11)
    const WDA_NONE: u32 = 0x00000000;
    const WDA_EXCLUDEFROMCAPTURE: u32 = 0x00000011;

    if let Ok(hwnd) = window.hwnd() {
        let affinity = if hide_from_capture {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        };
        unsafe {
            let _ = SetWindowDisplayAffinity(hwnd.0 as HWND, affinity);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_window_capture_stealth<R: tauri::Runtime>(
    _window: &WebviewWindow<R>,
    _hide_from_capture: bool,
) {
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

#[cfg(target_os = "windows")]
fn is_generic_token(token: &str) -> bool {
    matches!(
        token,
        "microsoft"
            | "windows"
            | "system"
            | "program"
            | "files"
            | "apps"
            | "app"
            | "desktop"
            | "notification"
            | "application"
            | "host"
            | "runtime"
            | "service"
            | "local"
            | "user"
            | "package"
            | "common"
    )
}

#[cfg(target_os = "windows")]
fn extract_core_app_token(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    if lower.is_empty() {
        return String::new();
    }

    for known in [
        "brave", "chrome", "msedge", "edge", "firefox", "opera", "vivaldi", "arc",
        "spotify", "vlc", "whatsapp", "telegram", "discord", "slack", "teams", "zoom", "webex", "skype",
        "outlook", "code", "snippingtool", "screensketch", "camera", "soundrecorder", "calculator",
        "notepad", "terminal", "powershell", "youtube", "youtubemusic",
    ] {
        if lower.contains(known) {
            return if known == "edge" {
                "msedge".to_string()
            } else if known == "screensketch" {
                "snippingtool".to_string()
            } else if known == "youtubemusic" {
                "youtube".to_string()
            } else {
                known.to_string()
            };
        }
    }

    // If it's a file path, extract the file stem
    if lower.contains('\\') || lower.contains('/') {
        let clean = lower.replace('/', "\\");
        if let Some(filename) = clean.split('\\').last() {
            let stem = filename.trim_end_matches(".exe");
            if !stem.is_empty() && stem.len() >= 3 && !is_generic_token(stem) {
                return stem.to_string();
            }
        }
    }

    // If it's an AUMID package name (e.g. "Microsoft.WindowsCamera_8wekyb3d8bbwe!App")
    let without_app = lower.split('!').next().unwrap_or(&lower);
    let package_prefix = without_app.split('_').next().unwrap_or(without_app);

    // Get the last component of a dot-separated name (e.g. "Microsoft.WindowsCamera" -> "windowscamera")
    if let Some(last_dot) = package_prefix.split('.').last() {
        let candidate = last_dot.trim();
        if candidate.len() >= 3 && !is_generic_token(candidate) {
            return candidate.to_string();
        }
    }

    // Fallback: pick the first non-generic token with length >= 3
    for token in lower.split(['.', '\\', '/', '_', ' ', '-']) {
        let t = token.trim().trim_end_matches(".exe");
        if t.len() >= 3 && !is_generic_token(t) {
            return t.to_string();
        }
    }

    String::new()
}

#[cfg(target_os = "windows")]
unsafe fn force_foreground_window(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    #[link(name = "user32")]
    unsafe extern "system" {
        fn AttachThreadInput(id_attach: u32, id_attach_to: u32, f_attach: i32) -> i32;
    }

    unsafe {
        let fg_hwnd = GetForegroundWindow();
        let fg_thread = if !fg_hwnd.is_null() {
            GetWindowThreadProcessId(fg_hwnd, std::ptr::null_mut())
        } else {
            0
        };
        let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
        let cur_thread = GetCurrentThreadId();

        let attached_fg = if fg_thread != 0 && fg_thread != cur_thread {
            AttachThreadInput(cur_thread, fg_thread, 1) != 0
        } else {
            false
        };
        let attached_target = if target_thread != 0 && target_thread != cur_thread {
            AttachThreadInput(cur_thread, target_thread, 1) != 0
        } else {
            false
        };

        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        } else {
            ShowWindow(hwnd, SW_SHOW);
        }
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);

        if attached_target {
            AttachThreadInput(cur_thread, target_thread, 0);
        }
        if attached_fg {
            AttachThreadInput(cur_thread, fg_thread, 0);
        }
    }
}

fn is_common_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "and"
            | "for"
            | "new"
            | "you"
            | "are"
            | "was"
            | "has"
            | "have"
            | "from"
            | "with"
            | "your"
            | "this"
            | "that"
            | "message"
            | "messages"
            | "notification"
            | "alert"
            | "sent"
            | "just"
            | "now"
    )
}

/// Uses Windows UI Automation (IUIAutomation) to find and select the exact browser tab
/// matching `title_hint` or `core_name_token` inside `hwnd`.
#[cfg(target_os = "windows")]
fn try_activate_browser_tab_via_uia(
    hwnd: windows_sys::Win32::Foundation::HWND,
    title_hint: Option<&str>,
    core_id_token: &str,
    core_name_token: &str,
) -> bool {
    use windows::Win32::Foundation::HWND as WinHwnd;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationInvokePattern,
        IUIAutomationLegacyIAccessiblePattern, IUIAutomationSelectionItemPattern,
        TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_InvokePatternId,
        UIA_LegacyIAccessiblePatternId, UIA_SelectionItemPatternId, UIA_TabItemControlTypeId,
    };
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, IsIconic, SetCursorPos, ShowWindow, SW_RESTORE,
    };

    const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
    const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn mouse_event(dw_flags: u32, dx: i32, dy: i32, dw_data: u32, dw_extra_info: usize);
    }

    let hint_lower = title_hint.unwrap_or("").trim().to_lowercase();
    let hint_tokens: Vec<&str> = hint_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3 && !is_common_stop_word(w))
        .collect();

    let is_browser_token = |tok: &str| {
        matches!(
            tok,
            "chrome" | "msedge" | "edge" | "brave" | "firefox" | "opera" | "vivaldi" | "arc" | "browser"
        )
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(automation) =
            CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        else {
            return false;
        };

        let Ok(root_el) = automation.ElementFromHandle(WinHwnd(hwnd as *mut _)) else {
            return false;
        };

        let prop_var = VARIANT::from(UIA_TabItemControlTypeId.0 as i32);
        let Ok(condition) =
            automation.CreatePropertyCondition(UIA_ControlTypePropertyId, &prop_var)
        else {
            return false;
        };

        let tab_list = match root_el.FindAll(TreeScope_Descendants, &condition) {
            Ok(list) => list,
            Err(_) => return false,
        };

        let count = tab_list.Length().unwrap_or(0);
        if count <= 0 {
            return false;
        }

        let mut best_idx: i32 = -1;
        let mut best_tab_score: i32 = 0;

        for i in 0..count {
            if let Ok(tab_el) = tab_list.GetElement(i) {
                if let Ok(name_bstr) = tab_el.CurrentName() {
                    let tab_name = name_bstr.to_string().to_lowercase();
                    let mut tab_score = 0;

                    if !hint_lower.is_empty() && hint_lower.len() >= 3 && tab_name.contains(&hint_lower) {
                        tab_score += 200;
                    } else if !hint_tokens.is_empty() {
                        let matched_words = hint_tokens.iter().filter(|w| tab_name.contains(**w)).count();
                        if matched_words > 0 {
                            tab_score += (matched_words as i32) * 55;
                        }
                    }

                    // Match the specific web brand / app name if not a generic browser name
                    if !core_name_token.is_empty()
                        && !is_browser_token(core_name_token)
                        && tab_name.contains(core_name_token)
                    {
                        tab_score += 95;
                    }
                    if !core_id_token.is_empty()
                        && !is_browser_token(core_id_token)
                        && tab_name.contains(core_id_token)
                    {
                        tab_score += 75;
                    }

                    // Award "audio playing" bonus if the tab matched hint/brand OR if no title hint was given
                    if (tab_score > 0 || hint_lower.is_empty())
                        && (tab_name.contains("audio playing") || tab_name.contains("playing audio"))
                    {
                        tab_score += 55;
                    }

                    if tab_score > best_tab_score {
                        best_tab_score = tab_score;
                        best_idx = i;
                    }
                }
            }
        }

        if best_idx >= 0 && best_tab_score >= 50 {
            if let Ok(target_tab) = tab_list.GetElement(best_idx) {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                    std::thread::sleep(std::time::Duration::from_millis(45));
                }
                // Bring this browser window to the foreground only AFTER confirming it contains the target tab
                force_foreground_window(hwnd);

                let mut switched = false;

                // Pattern 1: UIA SelectionItemPattern::Select()
                if let Ok(sel_pat) = target_tab
                    .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                        UIA_SelectionItemPatternId,
                    )
                {
                    if sel_pat.Select().is_ok() {
                        if let Ok(is_sel) = sel_pat.CurrentIsSelected() {
                            if is_sel.as_bool() {
                                switched = true;
                            }
                        }
                    }
                }

                // Pattern 2: UIA LegacyIAccessiblePattern::DoDefaultAction()
                if !switched {
                    if let Ok(leg_pat) = target_tab
                        .GetCurrentPatternAs::<IUIAutomationLegacyIAccessiblePattern>(
                            UIA_LegacyIAccessiblePatternId,
                        )
                    {
                        let _ = leg_pat.DoDefaultAction();
                    }
                }

                // Pattern 3: UIA InvokePattern::Invoke()
                if !switched {
                    if let Ok(inv_pat) = target_tab
                        .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                    {
                        let _ = inv_pat.Invoke();
                    }
                }

                // Re-verify selection state via SelectionItemPattern; if still not selected, click center of tab rect
                if let Ok(sel_pat) = target_tab
                    .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                        UIA_SelectionItemPatternId,
                    )
                {
                    if let Ok(is_sel) = sel_pat.CurrentIsSelected() {
                        switched = is_sel.as_bool();
                    }
                }

                if !switched {
                    if let Ok(rect) = target_tab.CurrentBoundingRectangle() {
                        let w = rect.right - rect.left;
                        let h = rect.bottom - rect.top;
                        if w > 8 && h > 8 {
                            let click_x = rect.left + (w * 2) / 5;
                            let click_y = rect.top + h / 2;
                            let mut prev_pt = POINT { x: 0, y: 0 };
                            let had_pos = GetCursorPos(&mut prev_pt) != 0;
                            SetCursorPos(click_x, click_y);
                            mouse_event(MOUSEEVENTF_LEFTDOWN | MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
                            if had_pos {
                                SetCursorPos(prev_pt.x, prev_pt.y);
                            }
                            switched = true;
                        }
                    }
                }

                return switched || true;
            }
        }
    }

    false
}

/// Activates or restores an application or browser window associated with a notification or media source.
/// Matches top-level windows via title hint (e.g. YouTube/Spotify Web track name) and process/app names,
/// and switches to the exact browser tab playing the media via Windows UI Automation.
#[cfg(target_os = "windows")]
pub fn activate_app_window(
    app_id: &str,
    app_name: &str,
    title_hint: Option<&str>,
) -> Result<bool, String> {
    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
    };

    struct CandidateWindow {
        hwnd: HWND,
        score: i32,
        is_browser: bool,
        is_exact_window: bool,
    }

    struct SearchContext {
        core_id_token: String,
        core_name_token: String,
        is_packaged_native: bool,
        title_hint: Option<String>,
        current_pid: u32,
        candidates: Vec<CandidateWindow>,
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let ctx = &mut *(lparam as *mut SearchContext);

            if IsWindowVisible(hwnd) == 0 {
                return 1;
            }

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == 0 || pid == ctx.current_pid {
                return 1;
            }

            let mut title_buf = [0u16; 512];
            let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
            if title_len <= 0 {
                return 1;
            }
            let title = String::from_utf16_lossy(&title_buf[..title_len as usize]).to_lowercase();

            let mut process_name = String::new();
            let h_proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if !h_proc.is_null() {
                let mut img_buf = [0u16; 1024];
                let mut img_len = img_buf.len() as u32;
                if QueryFullProcessImageNameW(h_proc, 0, img_buf.as_mut_ptr(), &mut img_len) != 0 {
                    let full_path =
                        String::from_utf16_lossy(&img_buf[..img_len as usize]).to_lowercase();
                    if let Some(filename) = full_path.split('\\').last() {
                        process_name = filename.to_string();
                    } else {
                        process_name = full_path;
                    }
                }
                CloseHandle(h_proc);
            }

            let is_generic_browser_proc = ["brave", "chrome", "msedge", "firefox", "opera", "vivaldi", "arc"]
                .iter()
                .any(|b| process_name.contains(b));

            // If this is a packaged native app (WhatsApp Desktop, Camera, Snipping Tool, Teams, etc.),
            // NEVER allow generic browser windows to be enumerated or scored as candidates!
            if ctx.is_packaged_native && is_generic_browser_proc {
                return 1;
            }

            let has_browser_suffix = title.ends_with(" - google chrome")
                || title.ends_with(" - chrome")
                || title.ends_with(" - brave")
                || title.ends_with(" - mozilla firefox")
                || title.ends_with(" - opera")
                || title.ends_with(" - vivaldi")
                || (title.ends_with("edge") && title.contains("microsoft"));

            // A standalone PWA window is run by a browser process but has no browser suffix in its title
            let is_standalone_pwa = is_generic_browser_proc && !has_browser_suffix;
            let is_browser = is_generic_browser_proc && has_browser_suffix;

            let is_browser_tok = |tok: &str| {
                matches!(
                    tok,
                    "chrome" | "msedge" | "edge" | "brave" | "firefox" | "opera" | "vivaldi" | "arc" | "browser"
                )
            };

            // If the notification explicitly came from a specific browser (e.g. Brave),
            // strictly skip other browser processes (Chrome, Edge) so they don't hijack!
            if is_browser
                && !ctx.core_id_token.is_empty()
                && is_browser_tok(&ctx.core_id_token)
                && !process_name.contains(&ctx.core_id_token)
            {
                return 1;
            }

            let target_is_generic_browser = is_browser_tok(&ctx.core_name_token)
                && (ctx.core_id_token.is_empty() || is_browser_tok(&ctx.core_id_token));

            // Check if this standalone PWA window matches the requested app/site
            let is_pwa_match = is_standalone_pwa
                && ((!ctx.core_name_token.is_empty()
                    && !is_browser_tok(&ctx.core_name_token)
                    && title.contains(&ctx.core_name_token))
                    || (!ctx.core_id_token.is_empty()
                        && !is_browser_tok(&ctx.core_id_token)
                        && title.contains(&ctx.core_id_token))
                    || (target_is_generic_browser
                        && ctx.title_hint.as_ref().map(|h| h.len() >= 3 && title.contains(h)).unwrap_or(false)));

            // If we are activating media from a generic browser (e.g. SMTC reports chrome.exe/msedge.exe)
            // and there is an open Media PWA window (YouTube Music, Spotify, SoundCloud, Apple Music),
            // give it a strong candidate score (135) so it wins if no tabbed browser tab matches via UIA!
            let is_media_pwa_candidate = is_standalone_pwa
                && target_is_generic_browser
                && (title.contains("youtube music")
                    || title.contains("spotify")
                    || title.contains("soundcloud")
                    || title.contains("apple music"));

            let mut is_exact_window = false;
            let mut app_match_score = 0;

            // 1. Process image name match (including Windows 11 New Outlook, Teams, Snipping Tool, WhatsApp)
            let is_outlook_match = (ctx.core_id_token == "outlook" || ctx.core_name_token == "outlook")
                && (process_name == "olk.exe"
                    || process_name == "hxoutlook.exe"
                    || process_name.contains("outlook"));
            let is_teams_match = (ctx.core_id_token == "teams" || ctx.core_name_token == "teams")
                && (process_name.contains("teams") || process_name == "ms-teams.exe");
            let is_snip_match = (ctx.core_id_token.contains("screensketch")
                || ctx.core_name_token.contains("snipping")
                || ctx.core_name_token.contains("screensketch"))
                && (process_name.contains("snippingtool") || process_name.contains("screensketch"));
            let is_whatsapp_proc = (ctx.core_id_token == "whatsapp" || ctx.core_name_token == "whatsapp")
                && (!is_generic_browser_proc
                    && (process_name.contains("whatsapp")
                        || (process_name == "applicationframehost.exe" && title.contains("whatsapp"))
                        || title.contains("whatsapp")));

            if is_pwa_match {
                app_match_score += 220;
                is_exact_window = true;
            } else if is_whatsapp_proc {
                app_match_score += 210;
                is_exact_window = true;
            } else if is_media_pwa_candidate {
                app_match_score += 135;
            } else if is_outlook_match || is_teams_match || is_snip_match {
                app_match_score += 160;
                is_exact_window = true;
            } else if !is_standalone_pwa && !ctx.core_id_token.is_empty() && process_name.contains(&ctx.core_id_token) {
                app_match_score += 85;
            } else if !is_standalone_pwa && !ctx.core_name_token.is_empty() && process_name.contains(&ctx.core_name_token) {
                app_match_score += 85;
            }

            // 2. Window title contains specific app/site name (NEVER award for generic browser suffix like "chrome"/"edge"/"brave")
            if !ctx.core_name_token.is_empty()
                && ctx.core_name_token.len() >= 3
                && !is_browser_tok(&ctx.core_name_token)
                && title.contains(&ctx.core_name_token)
            {
                app_match_score += 80;
                if !is_browser {
                    is_exact_window = true;
                }
            }

            // 3. If notification/media came from a browser or web brand, allow tabbed browser windows as candidates for UIA tab search
            let is_web_brand = matches!(
                ctx.core_name_token.as_str(),
                "youtube"
                    | "whatsapp"
                    | "gmail"
                    | "telegram"
                    | "discord"
                    | "slack"
                    | "teams"
                    | "outlook"
                    | "instagram"
                    | "linkedin"
                    | "spotify"
                    | "netflix"
                    | "github"
                    | "chatgpt"
                    | "reddit"
            );
            if is_browser && app_match_score == 0 && (is_web_brand || !is_browser_tok(&ctx.core_name_token)) {
                app_match_score += 40;
            }

            // CRITICAL: Never allow `title_hint` words to score an unrelated app or unrelated PWA window!
            if app_match_score == 0 {
                return 1;
            }

            let mut score = app_match_score;

            // 4. Exact or partial title hint match (ONLY for windows that matched the target app/browser)
            if let Some(ref hint) = ctx.title_hint {
                if hint.len() >= 3 && title.contains(hint) {
                    score += 150;
                    if !is_browser {
                        is_exact_window = true;
                    }
                } else {
                    let words: Vec<&str> = hint
                        .split(|c: char| !c.is_alphanumeric())
                        .filter(|w| w.len() >= 3 && !is_common_stop_word(w))
                        .collect();
                    let matched = words.iter().filter(|w| title.contains(**w)).count();
                    if matched > 0 {
                        score += (matched as i32) * 35;
                        if is_standalone_pwa && matched >= 2 {
                            is_exact_window = true;
                        }
                    }
                }
            }

            ctx.candidates.push(CandidateWindow {
                hwnd,
                score,
                is_browser,
                is_exact_window,
            });

            1 // continue enumeration
        }
    }

    let current_pid = unsafe { GetCurrentProcessId() };
    let core_id_token = extract_core_app_token(app_id);
    let core_name_token = extract_core_app_token(app_name);

    let id_lower = app_id.to_lowercase();
    let name_lower = app_name.to_lowercase();
    let is_packaged_native = id_lower.contains("5319275a.whatsappdesktop")
        || id_lower.contains("microsoft.windowscamera")
        || id_lower.contains("microsoft.screensketch")
        || id_lower.contains("msteams")
        || id_lower.contains("spotifyab")
        || (id_lower.contains('!')
            && !["chrome", "brave", "msedge", "edge", "firefox", "opera", "vivaldi", "arc"]
                .iter()
                .any(|b| id_lower.contains(b)));

    let mut context = SearchContext {
        core_id_token,
        core_name_token,
        is_packaged_native,
        title_hint: title_hint.map(|s| s.to_lowercase()),
        current_pid,
        candidates: Vec::new(),
    };

    unsafe {
        EnumWindows(Some(enum_windows_callback), &mut context as *mut _ as LPARAM);
    }

    context.candidates.sort_by(|a, b| b.score.cmp(&a.score));

    // Step 1: If there is an exact non-tabbed top-level window match (e.g. YouTube Music PWA, WhatsApp desktop window,
    // or a native app window), activate it DIRECTLY and never let browser tab search steal focus!
    if let Some(exact) = context.candidates.iter().find(|c| c.is_exact_window) {
        unsafe {
            force_foreground_window(exact.hwnd);
        }
        return Ok(true);
    }

    // Step 2: If no exact top-level window exists, but browser candidates exist,
    // search for a hidden background tab playing the media via Windows UI Automation
    for cand in &context.candidates {
        if cand.is_browser {
            if try_activate_browser_tab_via_uia(
                cand.hwnd,
                title_hint,
                &context.core_id_token,
                &context.core_name_token,
            ) {
                return Ok(true);
            }
        }
    }

    // Step 3: Activate the highest-scoring candidate window (score >= 85 for browsers, or non-browser window)
    if let Some(best) = context.candidates.iter().find(|c| !c.is_browser || c.score >= 85) {
        unsafe {
            force_foreground_window(best.hwnd);
        }
        return Ok(true);
    }

    // Step 4: If it is a real packaged Windows App AUMID (contains '!'), or known Windows UWP app (WhatsApp Desktop),
    // launch via explorer shell:AppsFolder to bring the native app window to the front
    let trimmed_id = app_id.trim();
    let target_aumid = if trimmed_id.contains('!') {
        Some(trimmed_id.to_string())
    } else if id_lower.contains("whatsapp") || name_lower.contains("whatsapp") {
        Some("5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App".to_string())
    } else {
        None
    };

    if let Some(aumid) = target_aumid {
        let folder_arg = format!("shell:AppsFolder\\{}", aumid);
        let wide_explorer: Vec<u16> = "explorer.exe".encode_utf16().chain(std::iter::once(0)).collect();
        let wide_arg: Vec<u16> = folder_arg.encode_utf16().chain(std::iter::once(0)).collect();
        let wide_open: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                wide_open.as_ptr(),
                wide_explorer.as_ptr(),
                wide_arg.as_ptr(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            );
            if (res as isize) > 32 {
                return Ok(true);
            }
        }
    }

    // Step 5: Check official URI protocol handlers (e.g. spotify:, tg:, discord:, whatsapp:)
    let protocol = if id_lower.contains("spotify") || name_lower.contains("spotify") {
        Some("spotify:".to_string())
    } else if id_lower.contains("whatsapp") || name_lower.contains("whatsapp") {
        Some("whatsapp:".to_string())
    } else if id_lower.contains("telegram") || name_lower.contains("telegram") {
        Some("tg:".to_string())
    } else if id_lower.contains("discord") || name_lower.contains("discord") {
        Some("discord:".to_string())
    } else if id_lower.contains("slack") || name_lower.contains("slack") {
        Some("slack:".to_string())
    } else if id_lower.contains("outlook") || name_lower.contains("outlook") {
        Some("ms-outlook:".to_string())
    } else if id_lower.contains("teams") || name_lower.contains("teams") {
        Some("msteams:".to_string())
    } else if id_lower.contains("screensketch") || name_lower.contains("snipping") {
        Some("ms-screenclip:".to_string())
    } else {
        None
    };

    if let Some(proto) = protocol {
        let wide_proto: Vec<u16> = proto.encode_utf16().chain(std::iter::once(0)).collect();
        let wide_open: Vec<u16> = "open".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                wide_open.as_ptr(),
                wide_proto.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            );
            if (res as isize) > 32 {
                return Ok(true);
            }
        }
    }

    // Step 6: Web URL fallback for web-based services (YouTube Music)
    if id_lower.contains("youtube") || name_lower.contains("youtube") {
        let wide_url: Vec<u16> = "https://music.youtube.com\0".encode_utf16().collect();
        let wide_open: Vec<u16> = "open\0".encode_utf16().collect();
        unsafe {
            let res = ShellExecuteW(
                std::ptr::null_mut(),
                wide_open.as_ptr(),
                wide_url.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            );
            if (res as isize) > 32 {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(not(target_os = "windows"))]
pub fn activate_app_window(
    _app_id: &str,
    _app_name: &str,
    _title_hint: Option<&str>,
) -> Result<bool, String> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_check_handles_zero_hwnd_safely() {
        assert!(!is_foreground_fullscreen(0));
    }

    #[test]
    fn test_activate_whatsapp_and_youtube() {
        let res_wa = activate_app_window("whatsapp", "WhatsApp", None);
        eprintln!("ACTIVATE_WHATSAPP_RESULT: {:?}", res_wa);
        assert!(res_wa.is_ok());

        let res_yt = activate_app_window("youtube", "YouTube Music", Some("Guru Randhawa"));
        eprintln!("ACTIVATE_YTM_RESULT: {:?}", res_yt);
        assert!(res_yt.is_ok());
    }
}
