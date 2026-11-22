//! Event-driven fullscreen foreground detection via WinEvent hooks.
//! Zero polling: sleeps inside `GetMessageW` until Windows raises
//! `EVENT_SYSTEM_FOREGROUND` or `EVENT_OBJECT_LOCATIONCHANGE`.

use crate::window::Hwnd;
use crate::{PlatformError, Result};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows_sys::Win32::UI::Shell::SHQueryUserNotificationState;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND, GWL_STYLE,
    GetClassNameW, GetForegroundWindow, GetMessageW, GetWindowLongW, GetWindowRect, MSG,
    OBJID_WINDOW, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, TranslateMessage,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_QUIT, WS_CAPTION, WS_THICKFRAME,
};

struct HookState {
    island_hwnd: isize,
    last_state: bool,
    on_change: Box<dyn Fn(bool) + Send>,
}

static HOOK_STATE: Mutex<Option<HookState>> = Mutex::new(None);

fn shell_reports_fullscreen_app() -> bool {
    // QUNS_BUSY = 2, QUNS_RUNNING_D3D_FULL_SCREEN = 3, QUNS_PRESENTATION_MODE = 4
    let mut state: i32 = 0;
    // SAFETY: valid out pointer for SHQueryUserNotificationState.
    let hr = unsafe { SHQueryUserNotificationState(&mut state) };
    hr >= 0 && matches!(state, 2 | 3 | 4)
}

/// Checks whether a third-party foreground application is currently fullscreen
/// (D3D game, borderless fullscreen, or fullscreen browser video).
pub fn is_foreground_fullscreen(island_hwnd: isize) -> bool {
    if island_hwnd == 0 {
        return false;
    }

    // SAFETY: Win32 window state queries on foreground & own handles.
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        if fg_hwnd.is_null() || fg_hwnd == (island_hwnd as HWND) {
            return false;
        }

        let mut class_name = [0u16; 256];
        let len = GetClassNameW(fg_hwnd, class_name.as_mut_ptr(), class_name.len() as i32);
        if len > 0 {
            let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
            if class_str == "Progman" || class_str == "WorkerW" || class_str == "Shell_TrayWnd" {
                return false;
            }
        }

        if shell_reports_fullscreen_app() {
            return true;
        }

        let style = GetWindowLongW(fg_hwnd, GWL_STYLE) as u32;
        let has_caption = (style & WS_CAPTION) == WS_CAPTION;
        let has_frame = (style & WS_THICKFRAME) != 0;
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

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if event == EVENT_OBJECT_LOCATIONCHANGE {
        if id_object != OBJID_WINDOW {
            return;
        }
        // SAFETY: pure handle comparison.
        let fg = unsafe { GetForegroundWindow() };
        if hwnd.is_null() || hwnd != fg {
            return;
        }
    }

    let mut guard = HOOK_STATE.lock().unwrap_or_else(|p| p.into_inner());
    let Some(state) = guard.as_mut() else {
        return;
    };
    let is_fs = is_foreground_fullscreen(state.island_hwnd);
    if is_fs != state.last_state {
        state.last_state = is_fs;
        (state.on_change)(is_fs);
    }
}

pub struct FullscreenWatcher {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl FullscreenWatcher {
    pub fn spawn(
        island_hwnd: Hwnd,
        on_change: impl Fn(bool) + Send + 'static,
    ) -> Result<Self> {
        let initial = is_foreground_fullscreen(island_hwnd.raw());
        if initial {
            on_change(true);
        }
        {
            let mut guard = HOOK_STATE.lock().unwrap_or_else(|p| p.into_inner());
            *guard = Some(HookState {
                island_hwnd: island_hwnd.raw(),
                last_state: initial,
                on_change: Box::new(on_change),
            });
        }

        let ready = Arc::new((Mutex::new(None::<u32>), Condvar::new()));
        let ready_bg = ready.clone();

        let thread = std::thread::Builder::new()
            .name("orbit-fullscreen".into())
            .spawn(move || {
                // SAFETY: thread-local message queue creation and WinEvent hooks.
                unsafe {
                    let mut init_msg: MSG = std::mem::zeroed();
                    PeekMessageW(&mut init_msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                    let tid = GetCurrentThreadId();
                    {
                        let (lock, cv) = &*ready_bg;
                        *lock.lock().unwrap_or_else(|p| p.into_inner()) = Some(tid);
                        cv.notify_all();
                    }

                    let hook_fg = SetWinEventHook(
                        EVENT_SYSTEM_FOREGROUND,
                        EVENT_SYSTEM_FOREGROUND,
                        std::ptr::null_mut(),
                        Some(win_event_proc),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    );
                    let hook_loc = SetWinEventHook(
                        EVENT_OBJECT_LOCATIONCHANGE,
                        EVENT_OBJECT_LOCATIONCHANGE,
                        std::ptr::null_mut(),
                        Some(win_event_proc),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                    );
                    if hook_fg.is_null() || hook_loc.is_null() {
                        tracing::warn!("SetWinEventHook returned null for fullscreen watcher");
                    }

                    let mut msg: MSG = std::mem::zeroed();
                    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                        TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    if !hook_fg.is_null() {
                        UnhookWinEvent(hook_fg);
                    }
                    if !hook_loc.is_null() {
                        UnhookWinEvent(hook_loc);
                    }
                }
            })
            .map_err(|e| PlatformError::Other(format!("spawning fullscreen watcher: {e}")))?;

        let (lock, cv) = &*ready;
        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        while guard.is_none() {
            guard = cv.wait(guard).unwrap_or_else(|p| p.into_inner());
        }
        let thread_id = guard.unwrap_or(0);

        Ok(Self {
            thread_id,
            thread: Some(thread),
        })
    }
}

impl Drop for FullscreenWatcher {
    fn drop(&mut self) {
        if self.thread_id != 0 {
            // SAFETY: posting WM_QUIT to our own watcher thread so GetMessageW exits cleanly.
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM::default(), LPARAM::default());
            }
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        let mut guard = HOOK_STATE.lock().unwrap_or_else(|p| p.into_inner());
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_hwnd_is_never_fullscreen() {
        assert!(!is_foreground_fullscreen(0));
    }
}
