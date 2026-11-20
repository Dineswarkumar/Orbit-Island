// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied and adapted from nook/crates/platform/src/window.rs.

//! Window styling, placement, click-through, and z-order.
//!
//! No window region (`SetWindowRgn`) is used. The window is layered
//! (`WS_EX_LAYERED`) and `WS_EX_TRANSPARENT` is toggled by the UI thread
//! from [`crate::hittest::HitTester`].

use crate::{Result, Win32Context};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumThreadWindows, GW_OWNER, GWL_EXSTYLE, GWL_STYLE, GWLP_HWNDPARENT, GetClientRect,
    GetWindow, GetWindowLongPtrW, GetWindowRect, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST,
    IsWindowVisible, LWA_ALPHA, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, WM_NCCALCSIZE, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP,
    WS_SYSMENU, WS_THICKFRAME,
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
