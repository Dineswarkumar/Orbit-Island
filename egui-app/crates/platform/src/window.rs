// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied and adapted from nook/crates/platform/src/window.rs.

//! Window styling, placement, click-through, and z-order.
//!
//! No window region (`SetWindowRgn`) is used. The window is layered
//! (`WS_EX_LAYERED`) and `WS_EX_TRANSPARENT` is toggled by the UI thread
//! from [`crate::hittest::HitTester`].

use crate::{Result, Win32Context};
use orbit_core::config::Placement;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumThreadWindows, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GWLP_HWNDPARENT, GetClientRect,
    GetWindow, GetWindowLongPtrW, GetWindowRect, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST,
    IsWindowVisible, LWA_ALPHA, STYLESTRUCT, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetLayeredWindowAttributes, SetWindowDisplayAffinity,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WM_NCACTIVATE,
    WM_NCCALCSIZE, WM_NCPAINT, WM_STYLECHANGING, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX,
    WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME,
};
use windows::core::BOOL;

/// A thread-safe Win32 window handle wrapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hwnd(isize);

impl Hwnd {
    pub fn from_raw(raw: isize) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> isize {
        self.0
    }

    pub(crate) fn get(self) -> HWND {
        HWND(self.0 as *mut core::ffi::c_void)
    }
}

/// Frame bits winit keeps on an undecorated window. Stripped so DWM does not
/// leave a non-client frame or top strip.
const FRAME_STYLE: u32 =
    WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0 | WS_THICKFRAME.0;

/// Ensures the client area covers the whole window (`WM_NCCALCSIZE` returns 0).
/// Call once on the window's thread.
pub fn fill_client(hwnd: Hwnd) -> Result<()> {
    unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if msg == WM_NCCALCSIZE && wparam.0 != 0 {
            return LRESULT(0);
        }
        if msg == WM_NCPAINT {
            return LRESULT(0);
        }
        if msg == WM_NCACTIVATE {
            return LRESULT(1);
        }
        if msg == WM_STYLECHANGING && wparam.0 == GWL_STYLE.0 as usize && lparam.0 != 0 {
            // Strip frame/caption bits before Windows or winit applies them (prevents white titlebar flash on tray popup).
            // SAFETY: Windows passes a valid STYLESTRUCT pointer in lparam for WM_STYLECHANGING.
            let ss = unsafe { &mut *(lparam.0 as *mut STYLESTRUCT) };
            ss.styleNew = (ss.styleNew | WS_POPUP.0) & !FRAME_STYLE;
            return LRESULT(0);
        }
        // SAFETY: forwarding to the next window procedure in the chain.
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }
    // SAFETY: subclassing our own window from its thread; `proc` is 'static.
    if !unsafe { SetWindowSubclass(hwnd.get(), Some(proc), 1, 0) }.as_bool() {
        return Err(crate::PlatformError::Other(
            "SetWindowSubclass failed".into(),
        ));
    }
    refresh_frame(hwnd)
}

/// Recomputes the frame only if the client area no longer matches the window rect.
pub fn ensure_client_fills(hwnd: Hwnd) -> Option<((i32, i32), (i32, i32))> {
    let (mut c, mut w) = (RECT::default(), RECT::default());
    // SAFETY: valid out pointers on our own window.
    unsafe {
        if GetClientRect(hwnd.get(), &mut c).is_err() || GetWindowRect(hwnd.get(), &mut w).is_err()
        {
            return None;
        }
    }
    let client = (c.right - c.left, c.bottom - c.top);
    let window = (w.right - w.left, w.bottom - w.top);
    if client == window {
        return None;
    }
    let _ = refresh_frame(hwnd);
    Some((client, window))
}

fn update_style(hwnd: Hwnd) -> bool {
    // SAFETY: plain style query/update on a window we own.
    unsafe {
        let style = GetWindowLongPtrW(hwnd.get(), GWL_STYLE);
        let want = (style | WS_POPUP.0 as isize) & !(FRAME_STYLE as isize);
        if want == style {
            return false;
        }
        SetWindowLongPtrW(hwnd.get(), GWL_STYLE, want);
        true
    }
}

fn update_ex_style(hwnd: Hwnd, f: impl FnOnce(isize) -> isize) -> Result<Option<isize>> {
    // SAFETY: plain style query/update on a window we own.
    unsafe {
        let ex = GetWindowLongPtrW(hwnd.get(), GWL_EXSTYLE);
        let want = f(ex);
        if want == ex {
            return Ok(None);
        }
        SetWindowLongPtrW(hwnd.get(), GWL_EXSTYLE, want);
        Ok(Some(ex))
    }
}

/// Makes the window a frameless, layered, no-taskbar tool window that does not
/// take focus unless `activatable` is true. Only writes when bits drifted.
pub fn ensure_overlay(hwnd: Hwnd, activatable: bool) -> Result<bool> {
    let changed = update_ex_style(hwnd, |ex| {
        let mut want =
            (ex | (WS_EX_TOOLWINDOW.0 | WS_EX_LAYERED.0) as isize) & !(WS_EX_APPWINDOW.0 as isize);
        if activatable {
            want &= !(WS_EX_NOACTIVATE.0 as isize);
        } else {
            want |= WS_EX_NOACTIVATE.0 as isize;
        }
        want
    })?;
    let restyled = update_style(hwnd);
    if changed.is_none() && !restyled {
        return Ok(false);
    }
    if let Some(before) = changed
        && before & WS_EX_LAYERED.0 as isize == 0
    {
        set_opacity(hwnd, 255)?;
    }
    refresh_frame(hwnd)?;
    Ok(true)
}

/// Whole-window opacity of a layered window (0 hides it, 255 shows per-pixel alpha).
pub fn set_opacity(hwnd: Hwnd, alpha: u8) -> Result<()> {
    // SAFETY: attribute update on a layered window we own.
    unsafe { SetLayeredWindowAttributes(hwnd.get(), Default::default(), alpha, LWA_ALPHA) }
        .api("SetLayeredWindowAttributes")
}

/// Shows the window without activating it, if it is hidden.
pub fn ensure_shown(hwnd: Hwnd) {
    // SAFETY: visibility query/show on a window we own.
    unsafe {
        if !IsWindowVisible(hwnd.get()).as_bool() {
            let _ = ShowWindow(hwnd.get(), SW_SHOWNOACTIVATE);
        }
    }
}

/// Lets mouse input fall through the (layered) window when `through` is true.
/// Only refreshes the frame when the bit actually changes.
pub fn set_click_through(hwnd: Hwnd, through: bool) -> Result<()> {
    let changed = update_ex_style(hwnd, |ex| {
        if through {
            ex | WS_EX_TRANSPARENT.0 as isize
        } else {
            ex & !(WS_EX_TRANSPARENT.0 as isize)
        }
    })?;
    if changed.is_some() {
        refresh_frame(hwnd)?;
    }
    Ok(())
}

fn refresh_frame(hwnd: Hwnd) -> Result<()> {
    // SAFETY: frame refresh on our own window.
    unsafe {
        SetWindowPos(
            hwnd.get(),
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    }
    .api("SetWindowPos(FRAMECHANGED)")
}

pub fn is_topmost(hwnd: Hwnd) -> bool {
    // SAFETY: style query on a window we own.
    let ex = unsafe { GetWindowLongPtrW(hwnd.get(), GWL_EXSTYLE) };
    ex & WS_EX_TOPMOST.0 as isize != 0
}

/// Puts the overlay above all windows (`on_top`) or behind normal windows.
/// Detaches owned windows (e.g. hidden IME window) while promoting to topmost
/// so Windows does not silently ignore `HWND_TOPMOST`.
pub fn set_layer(main: Hwnd, owner: Option<Hwnd>, on_top: bool) -> Result<()> {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    let place = |h: Hwnd, after: HWND| {
        // SAFETY: z-order change on windows we own.
        unsafe { SetWindowPos(h.get(), Some(after), 0, 0, 0, 0, flags) }.api("SetWindowPos(layer)")
    };
    if on_top {
        let root = owner.unwrap_or(main);
        let tree = owned_by(root.get());
        // SAFETY: owner (GWLP_HWNDPARENT) changes on windows of this thread,
        // restored immediately below.
        let owners: Vec<(HWND, isize)> = tree
            .iter()
            .map(|&h| unsafe { (h, SetWindowLongPtrW(h, GWLP_HWNDPARENT, 0)) })
            .collect();
        let promoted = place(root, HWND_TOPMOST).and_then(|()| {
            if owner.is_some() {
                place(main, HWND_TOPMOST)
            } else {
                Ok(())
            }
        });
        for (h, o) in owners.into_iter().rev() {
            // SAFETY: see above.
            unsafe { SetWindowLongPtrW(h, GWLP_HWNDPARENT, o) };
        }
        promoted?;
        if !is_topmost(main) {
            return Err(crate::PlatformError::Other(
                "window refused to become topmost".into(),
            ));
        }
    } else {
        if let Some(o) = owner {
            place(o, HWND_NOTOPMOST)?;
        }
        place(main, HWND_NOTOPMOST)?;
        place(main, HWND_BOTTOM)?;
        if let Some(o) = owner {
            place(o, HWND_BOTTOM)?;
        }
    }
    Ok(())
}

fn owned_by(root: HWND) -> Vec<HWND> {
    extern "system" fn collect(h: HWND, l: LPARAM) -> BOOL {
        // SAFETY: `l` is the `Vec` passed below, alive for the whole call.
        unsafe { (*(l.0 as *mut Vec<HWND>)).push(h) };
        true.into()
    }
    let mut all: Vec<HWND> = Vec::new();
    // SAFETY: the callback only pushes into `all`.
    unsafe {
        let _ = EnumThreadWindows(
            GetCurrentThreadId(),
            Some(collect),
            LPARAM(&mut all as *mut Vec<HWND> as isize),
        );
    }
    let depth = |h: HWND| -> Option<usize> {
        let mut cur = h;
        for d in 1..8 {
            // SAFETY: owner query; fails at the top of the chain.
            cur = unsafe { GetWindow(cur, GW_OWNER) }.ok()?;
            if cur == root {
                return Some(d);
            }
        }
        None
    };
    let mut owned: Vec<(usize, HWND)> = all
        .into_iter()
        .filter_map(|h| depth(h).map(|d| (d, h)))
        .collect();
    owned.sort_by_key(|&(d, _)| std::cmp::Reverse(d));
    owned.into_iter().map(|(_, h)| h).collect()
}

/// Moves and resizes the window (physical pixels) without touching z-order.
pub fn set_rect(hwnd: Hwnd, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
    // SAFETY: SetWindowPos on our own window.
    unsafe { SetWindowPos(hwnd.get(), None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE) }
        .api("SetWindowPos(rect)")
}

/// Fills the window's GDI surface with transparent black.
pub fn clear_gdi_surface(hwnd: Hwnd) {
    use windows::Win32::Graphics::Gdi::{
        BLACK_BRUSH, FillRect, GetDC, GetStockObject, HBRUSH, ReleaseDC,
    };
    // SAFETY: a DC of our own window, released right after.
    unsafe {
        let mut rc = RECT::default();
        if GetClientRect(hwnd.get(), &mut rc).is_err() {
            return;
        }
        let dc = GetDC(Some(hwnd.get()));
        if dc.is_invalid() {
            return;
        }
        FillRect(dc, &rc, HBRUSH(GetStockObject(BLACK_BRUSH).0));
        ReleaseDC(Some(hwnd.get()), dc);
    }
}

pub fn dpi(hwnd: Hwnd) -> u32 {
    // SAFETY: returns 0 for an invalid window, which we map to 96.
    match unsafe { GetDpiForWindow(hwnd.get()) } {
        0 => 96,
        d => d,
    }
}

/// Computes physical `(x, y, w, h)` for an island rect inside the monitor's usable work area.
pub fn compute_island_rect(
    work: crate::monitor::Rect,
    dpi: u32,
    placement: Placement,
    width_dp: f32,
    height_dp: f32,
) -> (i32, i32, i32, i32) {
    let eff_dpi = if dpi == 0 { 96 } else { dpi };
    let scale = eff_dpi as f32 / 96.0;
    let w = ((width_dp * scale).round() as i32).clamp(1, work.width().max(1));
    let h = ((height_dp * scale).round() as i32).max(1);
    let margin = (24.0 * scale).round() as i32;
    let x = match placement {
        Placement::Left => work.left + margin,
        Placement::Right => (work.right - w - margin).max(work.left),
        Placement::Center => work.left + (work.width() - w) / 2,
    };
    (x, work.top, w, h)
}

/// Excludes or includes the window from screen capture / screen sharing.
pub fn set_capture_stealth(hwnd: Hwnd, hide: bool) -> Result<()> {
    let affinity = if hide {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    // SAFETY: display affinity call on a window we own.
    unsafe { SetWindowDisplayAffinity(hwnd.get(), affinity) }.api("SetWindowDisplayAffinity")
}

/// Activates or restores an application or browser window associated with a notification or media source.
pub fn activate_app_window(
    app_id: &str,
    app_name: &str,
    title_hint: Option<&str>,
) -> std::result::Result<bool, String> {
    win_activate::activate_app_window(app_id, app_name, title_hint)
}

mod win_activate {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::HWND as WinHwnd;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationInvokePattern,
        IUIAutomationLegacyIAccessiblePattern, IUIAutomationSelectionItemPattern,
        TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_InvokePatternId,
        UIA_LegacyIAccessiblePatternId, UIA_SelectionItemPatternId, UIA_TabItemControlTypeId,
    };
    use windows_sys::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM, POINT};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, GetCurrentThreadId, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowTextW,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_RESTORE, SW_SHOW, SetCursorPos,
        SetForegroundWindow, ShowWindow,
    };

    static LAST_ACTIVATE: Mutex<Option<(String, Instant)>> = Mutex::new(None);

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

    fn extract_core_app_token(raw: &str) -> String {
        let lower = raw.trim().to_lowercase();
        if lower.is_empty() {
            return String::new();
        }
        for known in [
            "brave",
            "chrome",
            "msedge",
            "edge",
            "firefox",
            "opera",
            "vivaldi",
            "arc",
            "spotify",
            "vlc",
            "whatsapp",
            "telegram",
            "discord",
            "slack",
            "teams",
            "zoom",
            "webex",
            "skype",
            "outlook",
            "code",
            "snippingtool",
            "screensketch",
            "camera",
            "soundrecorder",
            "calculator",
            "notepad",
            "terminal",
            "powershell",
            "youtube",
            "youtubemusic",
        ] {
            if lower.contains(known) {
                return match known {
                    "edge" => "msedge".to_string(),
                    "screensketch" => "snippingtool".to_string(),
                    "youtubemusic" => "youtube".to_string(),
                    _ => known.to_string(),
                };
            }
        }
        if lower.contains('\\') || lower.contains('/') {
            let clean = lower.replace('/', "\\");
            if let Some(filename) = clean.split('\\').last() {
                let stem = filename.trim_end_matches(".exe");
                if !stem.is_empty() && stem.len() >= 3 && !is_generic_token(stem) {
                    return stem.to_string();
                }
            }
        }
        let without_app = lower.split('!').next().unwrap_or(&lower);
        let package_prefix = without_app.split('_').next().unwrap_or(without_app);
        if let Some(last_dot) = package_prefix.split('.').last() {
            let candidate = last_dot.trim();
            if candidate.len() >= 3 && !is_generic_token(candidate) {
                return candidate.to_string();
            }
        }
        for token in lower.split(['.', '\\', '/', '_', ' ', '-']) {
            let t = token.trim().trim_end_matches(".exe");
            if t.len() >= 3 && !is_generic_token(t) {
                return t.to_string();
            }
        }
        String::new()
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

    unsafe fn force_foreground_window(hwnd: HWND) {
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

    fn try_activate_browser_tab_via_uia(
        hwnd: HWND,
        title_hint: Option<&str>,
        core_id_token: &str,
        core_name_token: &str,
    ) -> bool {
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
                "chrome"
                    | "msedge"
                    | "edge"
                    | "brave"
                    | "firefox"
                    | "opera"
                    | "vivaldi"
                    | "arc"
                    | "browser"
            )
        };

        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let Ok(automation) =
                CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            else {
                return false;
            };
            let Ok(root_el) = automation.ElementFromHandle(WinHwnd(hwnd)) else {
                return false;
            };
            let prop_var = VARIANT::from(UIA_TabItemControlTypeId.0);
            let Ok(condition) =
                automation.CreatePropertyCondition(UIA_ControlTypePropertyId, &prop_var)
            else {
                return false;
            };
            let Ok(tab_list) = root_el.FindAll(TreeScope_Descendants, &condition) else {
                return false;
            };
            let count = tab_list.Length().unwrap_or(0);
            if count <= 0 {
                return false;
            }
            let mut best_idx: i32 = -1;
            let mut best_tab_score: i32 = 0;
            for i in 0..count {
                if let Ok(tab_el) = tab_list.GetElement(i)
                    && let Ok(name_bstr) = tab_el.CurrentName()
                {
                    let tab_name = name_bstr.to_string().to_lowercase();
                    let mut tab_score = 0;
                    if !hint_lower.is_empty()
                        && hint_lower.len() >= 3
                        && tab_name.contains(&hint_lower)
                    {
                        tab_score += 200;
                    } else if !hint_tokens.is_empty() {
                        let matched_words =
                            hint_tokens.iter().filter(|w| tab_name.contains(**w)).count();
                        if matched_words > 0 {
                            tab_score += (matched_words as i32) * 55;
                        }
                    }
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
                    if (tab_score > 0 || hint_lower.is_empty())
                        && (tab_name.contains("audio playing")
                            || tab_name.contains("playing audio"))
                    {
                        tab_score += 55;
                    }
                    if tab_score > best_tab_score {
                        best_tab_score = tab_score;
                        best_idx = i;
                    }
                }
            }
            if best_idx >= 0
                && best_tab_score >= 50
                && let Ok(target_tab) = tab_list.GetElement(best_idx)
            {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                    std::thread::sleep(Duration::from_millis(45));
                }
                force_foreground_window(hwnd);
                let mut switched = false;
                if let Ok(sel_pat) = target_tab
                    .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                        UIA_SelectionItemPatternId,
                    )
                    && sel_pat.Select().is_ok()
                {
                    let is_sel = sel_pat
                        .CurrentIsSelected()
                        .map(|v| v.as_bool())
                        .unwrap_or(true);
                    if is_sel {
                        switched = true;
                    } else {
                        std::thread::sleep(Duration::from_millis(15));
                        if sel_pat
                            .CurrentIsSelected()
                            .map(|v| v.as_bool())
                            .unwrap_or(true)
                        {
                            switched = true;
                        }
                    }
                }
                if !switched
                    && let Ok(leg_pat) = target_tab
                        .GetCurrentPatternAs::<IUIAutomationLegacyIAccessiblePattern>(
                            UIA_LegacyIAccessiblePatternId,
                        )
                    && leg_pat.DoDefaultAction().is_ok()
                {
                    switched = true;
                }
                if !switched
                    && let Ok(inv_pat) = target_tab
                        .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                    && inv_pat.Invoke().is_ok()
                {
                    switched = true;
                }
                if !switched
                    && let Ok(rect) = target_tab.CurrentBoundingRectangle()
                {
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
                    }
                }
                return true;
            }
            false
        }
    }

    pub fn activate_app_window(
        app_id: &str,
        app_name: &str,
        title_hint: Option<&str>,
    ) -> std::result::Result<bool, String> {
        let key = format!(
            "{}|{}|{}",
            app_id.trim().to_lowercase(),
            app_name.trim().to_lowercase(),
            title_hint.unwrap_or("").trim().to_lowercase()
        );
        {
            let mut guard = LAST_ACTIVATE
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = Instant::now();
            if let Some((ref last_key, ref last_time)) = *guard
                && last_key == &key
                && now.duration_since(*last_time) < Duration::from_millis(600)
            {
                tracing::debug!("Debouncing duplicate activate_app_window for key: {key}");
                return Ok(true);
            }
            *guard = Some((key, now));
        }

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
                    if QueryFullProcessImageNameW(h_proc, 0, img_buf.as_mut_ptr(), &mut img_len) != 0
                    {
                        let full_path =
                            String::from_utf16_lossy(&img_buf[..img_len as usize]).to_lowercase();
                        process_name = full_path
                            .split('\\')
                            .last()
                            .unwrap_or(&full_path)
                            .to_string();
                    }
                    CloseHandle(h_proc);
                }

                let is_generic_browser_proc =
                    ["brave", "chrome", "msedge", "firefox", "opera", "vivaldi", "arc"]
                        .iter()
                        .any(|b| process_name.contains(b));
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
                let is_standalone_pwa = is_generic_browser_proc && !has_browser_suffix;
                let is_browser = is_generic_browser_proc && has_browser_suffix;
                let is_browser_tok = |tok: &str| {
                    matches!(
                        tok,
                        "chrome"
                            | "msedge"
                            | "edge"
                            | "brave"
                            | "firefox"
                            | "opera"
                            | "vivaldi"
                            | "arc"
                            | "browser"
                    )
                };
                if is_browser
                    && !ctx.core_id_token.is_empty()
                    && is_browser_tok(&ctx.core_id_token)
                    && !process_name.contains(&ctx.core_id_token)
                {
                    return 1;
                }
                let target_is_generic_browser = is_browser_tok(&ctx.core_name_token)
                    && (ctx.core_id_token.is_empty() || is_browser_tok(&ctx.core_id_token));
                let is_pwa_match = is_standalone_pwa
                    && ((!ctx.core_name_token.is_empty()
                        && !is_browser_tok(&ctx.core_name_token)
                        && title.contains(&ctx.core_name_token))
                        || (!ctx.core_id_token.is_empty()
                            && !is_browser_tok(&ctx.core_id_token)
                            && title.contains(&ctx.core_id_token))
                        || (target_is_generic_browser
                            && ctx
                                .title_hint
                                .as_ref()
                                .map(|h| h.len() >= 3 && title.contains(h))
                                .unwrap_or(false)));
                let is_media_pwa_candidate = is_standalone_pwa
                    && target_is_generic_browser
                    && (title.contains("youtube music")
                        || title.contains("spotify")
                        || title.contains("soundcloud")
                        || title.contains("apple music"));

                let mut is_exact_window = false;
                let mut app_match_score = 0;

                let is_outlook_match =
                    (ctx.core_id_token == "outlook" || ctx.core_name_token == "outlook")
                        && (process_name == "olk.exe"
                            || process_name == "hxoutlook.exe"
                            || process_name.contains("outlook"));
                let is_teams_match =
                    (ctx.core_id_token == "teams" || ctx.core_name_token == "teams")
                        && (process_name.contains("teams") || process_name == "ms-teams.exe");
                let is_snip_match = (ctx.core_id_token.contains("screensketch")
                    || ctx.core_name_token.contains("snipping")
                    || ctx.core_name_token.contains("screensketch"))
                    && (process_name.contains("snippingtool")
                        || process_name.contains("screensketch"));
                let is_whatsapp_proc =
                    (ctx.core_id_token == "whatsapp" || ctx.core_name_token == "whatsapp")
                        && !is_generic_browser_proc
                        && (process_name.contains("whatsapp")
                            || (process_name == "applicationframehost.exe"
                                && title.contains("whatsapp"))
                            || title.contains("whatsapp"));

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
                } else if !is_standalone_pwa
                    && !ctx.core_id_token.is_empty()
                    && process_name.contains(&ctx.core_id_token)
                {
                    app_match_score += 85;
                } else if !is_standalone_pwa
                    && !ctx.core_name_token.is_empty()
                    && process_name.contains(&ctx.core_name_token)
                {
                    app_match_score += 85;
                }

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
                if is_browser
                    && app_match_score == 0
                    && (is_web_brand || !is_browser_tok(&ctx.core_name_token))
                {
                    app_match_score += 40;
                }
                if app_match_score == 0 {
                    return 1;
                }

                let mut score = app_match_score;
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
                1
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
                && ![
                    "chrome", "brave", "msedge", "edge", "firefox", "opera", "vivaldi", "arc",
                ]
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
            EnumWindows(
                Some(enum_windows_callback),
                &mut context as *mut _ as LPARAM,
            );
        }
        context.candidates.sort_by(|a, b| b.score.cmp(&a.score));

        if let Some(exact) = context.candidates.iter().find(|c| c.is_exact_window) {
            unsafe {
                force_foreground_window(exact.hwnd);
            }
            return Ok(true);
        }
        for cand in &context.candidates {
            if cand.is_browser
                && try_activate_browser_tab_via_uia(
                    cand.hwnd,
                    title_hint,
                    &context.core_id_token,
                    &context.core_name_token,
                )
            {
                return Ok(true);
            }
        }
        if let Some(best) = context
            .candidates
            .iter()
            .find(|c| !c.is_browser || c.score >= 85)
        {
            unsafe {
                force_foreground_window(best.hwnd);
            }
            return Ok(true);
        }

        let trimmed_id = app_id.trim();
        let target_aumid = if trimmed_id.contains('!') {
            Some(trimmed_id.to_string())
        } else if id_lower.contains("whatsapp") || name_lower.contains("whatsapp") {
            Some("5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App".to_string())
        } else {
            None
        };
        if let Some(aumid) = target_aumid
            && let Ok(explorer_path) = crate::system::system_root_binary("explorer.exe")
        {
            let folder_arg = format!("shell:AppsFolder\\{}", aumid);
            let wide_explorer: Vec<u16> = explorer_path
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let wide_arg: Vec<u16> = folder_arg.encode_utf16().chain(std::iter::once(0)).collect();
            let wide_open: Vec<u16> = "open\0".encode_utf16().collect();
            unsafe {
                let res = ShellExecuteW(
                    std::ptr::null_mut(),
                    wide_open.as_ptr(),
                    wide_explorer.as_ptr(),
                    wide_arg.as_ptr(),
                    std::ptr::null(),
                    1,
                );
                if (res as isize) > 32 {
                    return Ok(true);
                }
            }
        }

        let protocol = if id_lower.contains("spotify") || name_lower.contains("spotify") {
            Some("spotify:")
        } else if id_lower.contains("whatsapp") || name_lower.contains("whatsapp") {
            Some("whatsapp:")
        } else if id_lower.contains("telegram") || name_lower.contains("telegram") {
            Some("tg:")
        } else if id_lower.contains("discord") || name_lower.contains("discord") {
            Some("discord:")
        } else if id_lower.contains("slack") || name_lower.contains("slack") {
            Some("slack:")
        } else if id_lower.contains("outlook") || name_lower.contains("outlook") {
            Some("ms-outlook:")
        } else if id_lower.contains("teams") || name_lower.contains("teams") {
            Some("msteams:")
        } else if id_lower.contains("screensketch") || name_lower.contains("snipping") {
            Some("ms-screenclip:")
        } else {
            None
        };
        if let Some(proto) = protocol {
            let wide_proto: Vec<u16> = proto.encode_utf16().chain(std::iter::once(0)).collect();
            let wide_open: Vec<u16> = "open\0".encode_utf16().collect();
            unsafe {
                let res = ShellExecuteW(
                    std::ptr::null_mut(),
                    wide_open.as_ptr(),
                    wide_proto.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    1,
                );
                if (res as isize) > 32 {
                    return Ok(true);
                }
            }
        }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::Rect;

    #[test]
    fn placement_top_taskbar_bottom_taskbar_and_autohide() {
        let top_taskbar = Rect {
            left: 0,
            top: 48,
            right: 1920,
            bottom: 1080,
        };
        let bottom_taskbar = Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1032,
        };
        let autohide = Rect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };

        // Top taskbar: y starts at work.top = 48
        assert_eq!(
            compute_island_rect(top_taskbar, 96, Placement::Left, 200.0, 40.0),
            (24, 48, 200, 40)
        );
        assert_eq!(
            compute_island_rect(top_taskbar, 96, Placement::Center, 200.0, 40.0),
            (860, 48, 200, 40)
        );
        assert_eq!(
            compute_island_rect(top_taskbar, 96, Placement::Right, 200.0, 40.0),
            (1696, 48, 200, 40)
        );

        // Bottom taskbar & auto-hide at 125% DPI (120)
        assert_eq!(
            compute_island_rect(bottom_taskbar, 120, Placement::Left, 200.0, 40.0),
            (30, 0, 250, 50)
        );
        assert_eq!(
            compute_island_rect(autohide, 120, Placement::Right, 200.0, 40.0),
            (1640, 0, 250, 50)
        );
        assert_eq!(
            compute_island_rect(autohide, 120, Placement::Center, 200.0, 40.0),
            (835, 0, 250, 50)
        );
    }
}
