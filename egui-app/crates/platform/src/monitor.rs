// Copyright (c) 2026 The Nook contributors
// Licensed under the MIT License (see egui-app/THIRD_PARTY_NOTICES.md).
// Copied and adapted from nook/crates/platform/src/monitor.rs.

//! Monitor lookup and work-area queries in physical pixels.

use crate::window::Hwnd;
use crate::{PlatformError, Result, Win32Context};
use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITOR_DEFAULTTONEAREST,
    MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint, MonitorFromWindow,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};
use windows::core::BOOL;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

impl From<RECT> for Rect {
    fn from(r: RECT) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Monitor {
    pub id: isize,
    pub bounds: Rect,
    pub work: Rect,
    /// Effective DPI (96 = 100%).
    pub dpi: u32,
}

fn info(h: HMONITOR) -> Result<Monitor> {
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: cbSize set, valid out pointer.
    if !unsafe { GetMonitorInfoW(h, &mut mi) }.as_bool() {
        return Err(PlatformError::Other("GetMonitorInfoW failed".into()));
    }
    let (mut dpi_x, mut dpi_y) = (0, 0);
    // SAFETY: valid out pointers; on failure the DPI falls back to 96.
    let dpi = match unsafe { GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) } {
        Ok(()) if dpi_x > 0 => dpi_x,
        _ => 96,
    };
    Ok(Monitor {
        id: h.0 as isize,
        bounds: mi.rcMonitor.into(),
        work: mi.rcWork.into(),
        dpi,
    })
}

/// Every connected monitor, primary first.
pub fn all() -> Vec<Monitor> {
    unsafe extern "system" fn collect(h: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the `Vec` passed to `EnumDisplayMonitors` below.
        let list = unsafe { &mut *(lparam.0 as *mut Vec<HMONITOR>) };
        list.push(h);
        BOOL(1)
    }
    let mut handles: Vec<HMONITOR> = Vec::new();
    // SAFETY: the callback only touches `handles` through `lparam`.
    let _ = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut handles as *mut _ as isize),
        )
    };
    let mut out: Vec<Monitor> = handles.into_iter().filter_map(|h| info(h).ok()).collect();
    out.sort_by_key(|m| {
        (
            m.bounds.left != 0 || m.bounds.top != 0,
            m.bounds.left,
            m.bounds.top,
        )
    });
    out
}

pub fn cursor_pos() -> Result<(i32, i32)> {
    let mut p = POINT::default();
    // SAFETY: valid out pointer.
    unsafe { GetCursorPos(&mut p) }.api("GetCursorPos")?;
    Ok((p.x, p.y))
}

pub fn primary() -> Result<Monitor> {
    // SAFETY: (0,0) always lies on the primary monitor.
    info(unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) })
}

pub fn under_cursor() -> Result<Monitor> {
    let (x, y) = cursor_pos()?;
    // SAFETY: plain lookup.
    info(unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) })
}

pub fn of_window(hwnd: Hwnd) -> Result<Monitor> {
    // SAFETY: plain lookup.
    info(unsafe { MonitorFromWindow(hwnd.get(), MONITOR_DEFAULTTONEAREST) })
}

/// Returns the primary monitor usable work area via `SPI_GETWORKAREA`,
/// automatically excluding docked taskbars on any edge while keeping
/// full bounds when taskbar auto-hide is enabled.
pub fn primary_work_area() -> Rect {
    let mut rc = RECT::default();
    // SAFETY: valid out pointer for SPI_GETWORKAREA.
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rc as *mut RECT as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .is_ok();
    if ok && rc.right > rc.left && rc.bottom > rc.top {
        return rc.into();
    }
    primary().map(|m| m.work).unwrap_or(Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    })
}
