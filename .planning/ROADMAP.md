# ORBIT Roadmap

## Milestone 1: Foundation (Current)
- **Status:** In Progress
- **Scope:**
  - Scaffold Tauri 2 + Vite + React + TypeScript + Tailwind CSS using npm.
  - Multi-window configuration (transparent island + settings window).
  - Minimal Rust backend: IPC commands for JSON config persistence in `%APPDATA%\ORBIT\config.json`.
  - System tray icon bootstrap (Settings, Quit).
  - Baseline memory & startup measurement of the empty Tauri/React shell.
- **Exit Criteria:**
  - Clean builds (`npm run build`, `cargo check`).
  - Transparent main window renders properly with pass-through outside bounds.
  - Independent Settings window functions cleanly.
  - Configuration persists across app restarts.
  - Memory footprint measured and evaluated against <50 MB goal / 100 MB max.

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
