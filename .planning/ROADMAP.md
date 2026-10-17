# ORBIT Roadmap

## Milestone 1: Foundation
- **Status:** Completed
- **Delivered:**
  - Scaffolded Tauri 2 + Vite + React + TypeScript + Tailwind CSS using npm.
  - Multi-window configuration (transparent island + settings window).
  - Minimal Rust backend: IPC commands for JSON config persistence in `%APPDATA%\com.orbit.island\config.json`.
  - System tray icon bootstrap (`TrayIconBuilder::with_id("orbit-tray")` retained in app state).
  - Click-through outside bounds and corners via Win32 `SetWindowRgn`.
  - Baseline memory measurement across verified process tree.

## Milestone 2: Core Shell
- **Status:** Implemented and user-accepted (2026-10-14)
- **Delivered:**
  - Compact (200x56) & expanded (400x184) states with synchronized Win32 rounded region clipping.
  - 4-section scrollable navigation (Home, Calendar, Productivity Hub, Hardware Monitor) with clamped mouse wheel/touchpad scrolling and vertical section indicator dots.
  - Minimal compact pill displaying live clock/AM-PM or active media capsule.
  - Settings window with constrained scroll body, pinned header, and save action.
  - Live configuration and theme broadcast between Settings and Island webviews via Tauri events.
  - Native right-click menu with themes and Settings access.
  - Config validation and atomic JSON replacement.
  - Telemetry distinguishing Private Working Set, Private Bytes, and Working Set.
- **Deferred / Pending:** Compacting animation reverse transition glitch is intentionally deferred per user instruction.

## Milestone 3: Feature Slices
- **Status:** Completed
- **Delivered Slices:**
  1. Universal Media Session Controls (GSMTC).
  2. Compact Notifications & Privacy Indicators.
  3. Calendar & Productivity Hub.
  4. Hardware and System Monitor.
  5. File Shelf (SQLite persistence & native picker).
  6. Island UI Redesign (Clamped 3-section navigation).

## Milestone 4: Reliability & Production Hardening
- **Status:** Completed
- **Delivered:**
  - Fullscreen application auto-suppression via native Win32 `is_foreground_fullscreen`.
  - Multi-monitor / DPI scale change handling and off-screen window recovery (`ensure_window_on_screen`).
  - Rotating diagnostic logger (`orbit.log` -> `orbit.log.1` at 2MB limit in `%APPDATA%\com.orbit.island\logs\`).
  - Windows installer NSIS per-user configuration in `tauri.conf.json`.
  - One-click MSI-Center-style RAM / Working Set purging (`free_up_memory`).
  - Standalone embedded custom-protocol packaging.

## ORBIT Version 1.0.0 (Master Execution Blueprint)
- **Status:** Completed & Production Released
- **Delivered:**
  - **Phase 1 — Built-In Countdown / Focus Timer:**
    - Right-click context menu quick triggers (`5m`, `15m`, `25m`, `Cancel`).
    - Timestamp deadline persistence in `localStorage` across restarts and sleep.
    - Compact capsule circular SVG progress ring (`#f59e0b` amber) + `MM:SS` countdown (matching `reference-timer-and-media.png`).
    - Expanded Home interactive quick-timer bar (`5m`, `15m`, `25m`, `45m`, `Stop`).
    - Deadline expiry transient critical notification.
  - **Phase 2 — File Shelf Drawer & Drag-and-Drop:**
    - `FileShelfDrawer.tsx` component inside the expanded canvas.
    - Top-left `Shelf (N)` toggle pill on idle Home view.
    - Drag-and-drop auto-ingestion (`tauri://drag-drop`) expanding island to Section 0 Home and opening shelf drawer.
    - Actions: Open (`ShellExecuteW`), Reveal in Explorer, Remove reference, Clear shelf, and Add file via native picker dialog.
  - **Phase 3 — Windows Startup Registry & Reset Controls:**
    - Native `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` synchronization (`sync_startup_registry`).
    - Settings "Startup & Reset" card with Launch at Windows Sign-In toggle.
    - "Reset Settings to Defaults" and "Clear Local Reminders & Shelf References" (`clear_local_data`) commands.
  - **Phase 4 — Production Release & Verification:**
    - Version bumped to `1.0.0` in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
    - Verification: `npm run build` (passed 0 errors), `cargo test` (all 25/25 unit tests passed).
    - Standalone release binary `orbit.exe` compiled and launched.
