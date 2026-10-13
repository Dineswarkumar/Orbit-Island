# Foundation Milestone Plan

## Goal
Establish an empty, production-grade Tauri 2 + React + TypeScript foundation with dual windows, JSON configuration persistence, system tray, and baseline memory/startup measurements.

## Scope
1. Scaffold package structure using npm with React 19/18, TypeScript, Tailwind CSS, Vite, Zustand, Zod.
2. Initialize `src-tauri` with Tauri 2 and Windows-ready dependencies (`serde`, `serde_json`, `tokio`, `sysinfo` or Win32 memory check).
3. Window Configuration in `tauri.conf.json`:
   - `main`: Title "ORBIT Island", size 400x56 (compact pill bounds with native Win32 `SetWindowRgn` capsule clipping), transparent: true, decorations: false, alwaysOnTop: true, skipTaskbar: true, shadow: false.
   - `settings`: Title "ORBIT Settings", size 640x520, transparent: false, decorations: true, visible: false.
4. Rust backend services & commands:
   - Config service: Load and save `config.json` in `%APPDATA%\com.orbit.island\config.json`.
   - Window commands: `open_settings_window`, `close_settings_window`, `center_island_window`.
   - Memory command: `get_memory_metrics` (enumerates Private Working Set and Private Bytes across `orbit.exe` and complete WebView2 tree).
   - System tray: `TrayIconBuilder::with_id("orbit-tray")` retained in app state with "Settings...", "Toggle Island", and "Quit ORBIT".
5. Frontend Foundation:
   - Window router: Dispatches to `IslandApp` or `SettingsApp` based on URL parameter (`?window=settings`).
   - `IslandApp`: Compact pill (400x56) with ORBIT branding, settings trigger, and click bounds.
   - `SettingsApp`: Independent settings container with real-time memory telemetry and configuration persistence feedback.
   - Zustand `useConfigStore` with Zod validation schemas.
6. Telemetry & Acceptance:
   - Build verification (`npm run build`, `cargo check`).
   - Launch verification via release build.
   - Measure startup latency (143-356 ms across cold/warm runs).
   - Measure settled idle memory across 7-process tree (124.19 MB settled average PWS; aspiration <30 MB, target <50 MB; overage tracked).
   - Verify native click-through including transparent rounded corners via Win32 `WindowFromPoint`.
