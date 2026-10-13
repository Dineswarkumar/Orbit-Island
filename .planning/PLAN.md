# Foundation Milestone Plan

## Goal
Establish an empty, production-grade Tauri 2 + React + TypeScript foundation with dual windows, JSON configuration persistence, system tray, and baseline memory/startup measurements.

## Scope
1. Scaffold package structure using npm with React 19/18, TypeScript, Tailwind CSS, Vite, Zustand, Zod.
2. Initialize `src-tauri` with Tauri 2 and Windows-ready dependencies (`serde`, `serde_json`, `tokio`, `sysinfo` or Win32 memory check).
3. Window Configuration in `tauri.conf.json`:
   - `main`: Title "ORBIT Island", size 800x400 (or initial bounds), transparent: true, decorations: false, alwaysOnTop: true, skipTaskbar: true, shadow: false.
   - `settings`: Title "ORBIT Settings", size 680x560, transparent: false, decorations: true, visible: false.
4. Rust backend services & commands:
   - Config service: Load and save `config.json` in `%APPDATA%\ORBIT\config.json`.
   - Window commands: `show_settings_window`, `hide_settings_window`.
   - Memory command: `get_process_memory` (enumerates private working set of ORBIT process and child WebView2 processes for baseline verification).
   - System tray: Left click / right click tray menu with "Settings" and "Quit ORBIT".
5. Frontend Foundation:
   - Window router: Dispatches to `IslandShell` or `SettingsShell` depending on window label (`main` vs `settings`).
   - `IslandShell`: Minimal transparent pill with ORBIT branding, testing transparency and click bounds.
   - `SettingsShell`: Independent settings container loading saved JSON config.
   - Zustand `useConfigStore` with Zod validation schemas.
6. Telemetry & Acceptance:
   - Build verification (`npm run build`, `cargo check`).
   - Launch verification via `tauri dev` or release build.
   - Measure startup latency and process memory (Private Working Set across main process and all WebView2 child processes).
