# ORBIT Roadmap

## Milestone 1: Foundation (Current)
- **Status:** In Progress
- **Scope:**
  - Scaffold Tauri 2 + Vite + React + TypeScript + Tailwind CSS using npm.
  - Multi-window configuration (transparent island + settings window).
  - Minimal Rust backend: IPC commands for JSON config persistence in `%APPDATA%\com.orbit.island\config.json`.
  - System tray icon bootstrap (`TrayIconBuilder::with_id("orbit-tray")` retained in app state).
  - Baseline memory & startup measurement of the empty Tauri/React shell.
- **Exit Criteria:**
  - Clean builds (`npm run build`, `cargo check`).
  - Transparent main window (400x56) renders properly with native Win32 `SetWindowRgn` capsule pass-through outside bounds and corners.
  - Independent Settings window functions cleanly.
  - Configuration persists across app restarts with live feedback.
  - Memory footprint measured honestly across full process tree against aspiration (<30 MB) and target (<50 MB); overage recorded and tracked.

## Milestone 2: Core Shell (Next)
- **Status:** Queued
- **Scope:**
  - Compact & expanded pill states with Motion spring physics.
  - Live clock/date with 12/24-hour modes.
  - Persistent countdown timer with deadline recovery.
  - 4-section scroll navigation (Home -> Calendar -> Productivity Hub -> Hardware Monitor).
  - Context menu and tray integration.
  - Settings UI sections and density controls.

## Milestone 3: Feature Slices
- **Status:** Queued
- **Features:**
  - Universal media session controls (GSMTC).
  - Compact notifications and privacy indicators.
  - Calendar & Productivity Hub (local SQLite + OAuth providers).
  - Hardware monitor (CPU, RAM, GPU, network, disk, power).
  - File Shelf (up to 10 held references, drag/drop).

## Milestone 4: Reliability & Packaging
- **Status:** Queued
- **Scope:**
  - Fullscreen detection & suppression.
  - Multi-monitor / DPI change handling & off-screen recovery.
  - Local diagnostic logging.
  - Windows installer creation (NSIS or WiX).
