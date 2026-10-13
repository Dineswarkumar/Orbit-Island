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

## Milestone 2: Core Shell (Current)
- **Status:** Implemented & Awaiting Physical Review
- **Delivered:**
  - Compact (200x56) & expanded (400x184) states with synchronized Win32 rounded region clipping and fluid CSS transitions.
  - Minimal compact pill displaying prominent live time and secondary AM/PM with zero placeholder clutter.
  - Expanded idle clock panel grounded in `reference-idle-clock.png`: large clock, date underneath, section navigation dots, and canonical ORBIT logo.
  - Race-free hover-to-expand and auto-collapse state machine with cancellation of stale resize timers.
  - Clean compacting exit with zero text flying or sliding underneath the capsule.
  - Scrollable Settings window layout with pinned header and save actions, usable across default and resized dimensions.
  - Live configuration and theme broadcast between Settings and Island webviews via Tauri events.
  - Accurate telemetry labeling distinguishing Win32 Private Bytes (commit charge) from Total Working Set.

## Milestone 3: Feature Slices
- **Status:** Queued (Next)
- **Planned Slices:**
  1. Universal media session controls (GSMTC) with active playback priority over Home clock.
  2. Compact notifications and privacy indicators.
  3. Calendar & Productivity Hub.
  4. Hardware monitor (CPU, RAM, GPU, network, disk, power).
  5. File Shelf (up to 10 held references, drag/drop).

## Milestone 4: Reliability & Packaging
- **Status:** Queued
- **Scope:**
  - Fullscreen detection & suppression.
  - Multi-monitor / DPI change handling & off-screen recovery.
  - Local diagnostic logging.
  - Windows installer creation (NSIS or WiX).
