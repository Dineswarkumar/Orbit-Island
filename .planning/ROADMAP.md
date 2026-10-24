# ORBIT — Project Roadmap

## Overview
- **Product**: ORBIT (Windows 11 Dynamic Island Desktop Overlay)
- **Architecture**: Hybrid Native Stack — Tauri 2 (Rust Backend) + React 19 + TypeScript + Tailwind CSS (WebView2 UI)
- **Primary Goal**: Ultra-low RAM footprint (< 50 MB target, < 100 MB maximum) with native 60fps spring animations, strict module isolation, and zero background CPU waste.

---

## Completed Milestones & Phases (Phases 1–4 — Core v1.0.0)

### Milestone 1: Foundation & Memory Proof [COMPLETED]
- [x] **M1.1**: Initialize Tauri 2 + Vite + React + TypeScript + Tailwind CSS project skeleton.
- [x] **M1.2**: Implement strict JSON configuration store (`%APPDATA%\com.orbit.island\config.json`) with schema validation and safe fallback defaults.
- [x] **M1.3**: Configure dual-window setup (`main` frameless transparent overlay + `settings` standard window) and System Tray integration.
- [x] **M1.4**: Implement real-time Windows process memory telemetry (`GetProcessMemoryInfo` / Working Set & Private Bytes) to prove the RAM budget early.

### Milestone 2: Core Shell & State Machine [COMPLETED]
- [x] **M2.1**: Build the 3-State Island State Machine (`Compact` -> `Transient Notification` -> `Expanded`).
- [x] **M2.2**: Implement dynamic bounding-box click-through and window region resizing so transparent areas never block the user's desktop cursor.
- [x] **M2.3**: Build the Settings Window UI with live reactive updates to the Island overlay.
- [x] **M2.4**: Add multi-section horizontal/vertical carousel navigation, indicator dots, and reduced-motion accessibility support.

### Milestone 3: Feature Slices (Phases 1–4 Executed) [COMPLETED]
- [x] **Slice 1 (Media Session Integration — Phase 1)**: Windows `GlobalSystemMediaTransportControlsSessionManager` (SMTC) integration, album art base64 pipeline, play/pause/skip/seek controls, source pinning, and 26px control buttons.
- [x] **Slice 2 (Notification & Privacy Indicators — Phase 1 & 2)**: Windows `UserNotificationListener` integration, drop-expand-rewind entrance animation, brand logos (WhatsApp, Telegram, Discord, Gmail, Slack, Teams, Spotify, GitHub, YouTube, X), privacy mode masking, and Camera/Microphone active registry monitoring.
- [x] **Slice 3 (Calendar, Reminders & Timer — Phase 3)**: SQLite persistence (`rusqlite`), local reminder scheduling, interactive calendar, and persistent countdown timer in the compact pill.
- [x] **Slice 4 (Hardware Monitor — Phase 2)**: Event-driven system metrics (CPU, GPU via PDH, RAM with 1-click `EmptyWorkingSet` Boost cleaner, Disk C, Network Up/Down, Battery dynamic charging pill, and 2-second compact utility alerts for Charger/Wi-Fi/Bluetooth/Hotspot).
- [x] **Slice 5 (File Shelf — Phase 3)**: Drag-and-drop temporary file reference shelf (max 10 items) with native file picker, Explorer reveal, and SQLite persistence.
- [x] **Slice 6 (Themes & Appearance — Phase 1)**: 8 curated 3D themes (`Obsidian Studio`, `Linear Indigo`, `Titanium Slate`, `OLED Black`, `Warm Bronze`, `Sage Emerald`, `Arctic Frost`, `Crystal Frost`) with opaque high-contrast surfaces and 4 transparency material modes.

### Milestone 4: Reliability, Hardening & Packaging (Phase 4) [COMPLETED]
- [x] **M4.1**: Single-instance mutex (`CreateMutexW` + focus wakeup), Launch-at-Startup (`HKCU\...\Run`), and multi-monitor DPI clamping (`ensure_window_on_screen`).
- [x] **M4.2**: Smart `Always on Top` (`set_window_topmost`) + fullscreen media/game detection (`is_foreground_fullscreen` ignoring normal maximized browsers/apps while auto-hiding on true fullscreen video like YouTube/VLC).
- [x] **M4.3**: Rotating local diagnostic logger (`%APPDATA%\com.orbit.island\logs\orbit.log`, 2 MB cap).
- [x] **M4.4**: Production Windows NSIS Installer (`ORBIT_1.0.0_x64-setup.exe`), `README.md`, `LICENSE`, and `.gitignore`.

---

## Remaining PRD Completion Phases (Phases 5–8 — 8 Sub-Phases)

### Phase 5: Concurrent Activity Layout, Enhanced Timer, & Optional Compact Weather [COMPLETED]
- [x] **Sub-Phase 5.1**: Optional Compact Weather & Temperature Glance (`showWeather`, `°C`/`°F`, cached non-blocking fetcher with offline fallback).
- [x] **Sub-Phase 5.2**: Simultaneous Timer + Media Dual Capsule (`compact_dual` 256px region) & Timer Pause/Resume/Reset/Custom Label.

### Phase 6: Adaptive Album Art Accents, Follow-Windows Theme, & Context Menu Pin/Expand Controls [COMPLETED]
- [x] **Sub-Phase 6.1**: Adaptive Accent Color Extraction from Album Artwork (`adaptiveAlbumAccent` canvas dominant-color sampler).
- [x] **Sub-Phase 6.2**: Follow Windows System Theme (`theme: "system"`) & Context Menu Pin/Expand/Fullscreen Controls (`Pin Expanded Island`, `Auto-Hide in Fullscreen Video`, `Expand on Click Only`).

### Phase 7: File Shelf Copy Path, Managed Copies, & Missing-File Recovery [COMPLETED]
- [x] **Sub-Phase 7.1**: Copy Path action & Missing/Moved-File status badge in File Shelf.
- [x] **Sub-Phase 7.2**: Optional Managed Shelf Storage Copy Mode (`%APPDATA%\com.orbit.island\shelf_storage\` with safe temp file copy for files ≤ 50 MB).

### Phase 8: First-Launch Guided Onboarding Tour, 3-Tier Reset, & Diagnostic Log Export [COMPLETED]
- [x] **Sub-Phase 8.1**: First-Launch Skippable 3-Step Guided Tour (`OnboardingOverlay.tsx`) + Replay Tour button in Settings.
- [x] **Sub-Phase 8.2**: Three-Tier Reset (`Reset Settings`, `Clear Local Data`, `Full Reset to Onboarding`) & Reviewed Diagnostic Log Export (`orbit.log` reveal/export).
- [x] **Sub-Phase 8.3**: Bump version to v2.0.0 across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`. Ready for GitHub release!

### Phase 9: Compact Right Slot Customizer, Live Network Transfers & Rec Indicator, Meeting & Stealth Suppressions [COMPLETED]
- [x] **Sub-Phase 9.1**: Compact Right Slot Customizer (8 modes: `weather`, `orbit_logo`, `windows_logo`, `apple_logo`, `linux_logo`, `custom_text`, `custom_image`, `smart`).
- [x] **Sub-Phase 9.2**: Live Upload / Download Speed activity flip & Screen Recording `🔴 REC` indicator.
- [x] **Sub-Phase 9.3**: Incoming Call action, live call counter `📞 MM:SS`, Meeting Auto-Suppression, Screen Share Stealth (`WDA_EXCLUDEFROMCAPTURE`), & Global Hotkeys (`Alt+O`, `Alt+H`, `Alt+S`, `Alt+M`).
- [x] **Sub-Phase 9.4**: Comprehensive responsive settings UI with full light/dark support.

### Phase 9.5: Bug Fixes, Worst-Case Multi-Activity UX & Theme Engine Overhaul [COMPLETED]
- [x] **Task 1**: Fixed notification redirection in multi-browser (`Brave`, `Chrome`, `Edge`) and native packaged app (`WhatsApp.Root`, UWP `ApplicationFrameHost`) setups via `is_packaged_native` and strict emitting browser matching in `src-tauri/src/services/window.rs`.
- [x] **Task 2**: Fixed compact notification `Open`, `View`, and `Dismiss` buttons (`onPointerDown={(e) => e.stopPropagation()}`), optimistic synchronous dismissal, and eliminated notification re-popping upon return/collapse via `clearTransientNotification()` in `IslandApp.tsx` and `CompactCapsule.tsx`.
- [x] **Task 3**: Ensured Camera and Microphone indicators remain visible across all compact and expanded states (Media player, Notification Inspector, and non-home section carousel). Added worst-case multi-activity handling (Simultaneous Media + Call + Timer) with a dynamic Activity Mode Switcher and temporary Live Call & Privacy Deck in `HomeSection.tsx`.
- [x] **Task 4**: Overhauled all 8 themes (`Midnight Sapphire`, `Nebula Violet`, `VisionOS Spatial Titanium`, `OLED Cyber Pitch`, `Sunset Rose & Amber`, `Aurora Botanical Emerald`, `Arc Ocean & Solar Gold`, `Apple Ceramic Pearl`) with dedicated surface tints, theme-colored 3D borders (`var(--island-border)`), card fills (`var(--island-card-bg)`), and scoped `.theme-light` high-contrast typography rules in `theme.ts` and `globals.css`.
- [x] **Task 5**: Verified via `npm run build` (0 errors), `cargo test` (32/32 unit tests passed), `cargo build --release` (clean build in 3m 03s), and confirmed live release binary `orbit.exe` running with PID 8648.

### Phase 10: Smart Calendar Hub — Academic & Office Schedule Importer, Live Event Alerts & Upgraded 3D Calendar UI [COMPLETED]
- [x] **Task 1**: Implemented native academic and office schedule engine in Rust (`src-tauri/src/services/db.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`) supporting `.ics`, `.csv`, and `.json` files, civil-date epoch math with local UTC bias adjustment (`GetTimeZoneInformation`), deduplication within 60s window, category auto-classification (`exam`, `deadline`, `meeting`, `class`, `holiday`, `event`), and `pick_and_import_calendar` / `clear_imported_calendar` commands.
- [x] **Task 2**: Added frontend types (`ReminderItem`), IPC client methods (`pickAndImportCalendar`, `clearImportedCalendar`), live synchronization via `"reminders-updated"`, 15-minute countdown alert detection with category colors, and `todayNextEvent` computation in `IslandApp.tsx`.
- [x] **Task 3**: Upgraded 3D Calendar Section UI (`CalendarSection.tsx`) with colored category badges (`EXAM`, `DUE`, `MEETING`, `CLASS`, `HOLIDAY`), multi-color event dots in the 7-column calendar matrix, drawer `📥 Import` button, and in-form category cycling. Added `todayNextEvent` 3D pill badge in `HomeSection.tsx` for 1-click navigation to Calendar.
- [x] **Task 4**: Added Bento Card in `SettingsApp.tsx`: "Academic & Office Schedule Sync (.ICS / .CSV / .JSON)" with 3 quick import buttons (`🎓 Academic`, `💼 Office`, `📅 iCal`), sample CSV copy helper, and `Clear Imported Schedules` action preserving local tasks.
- [x] **Task 5**: Verified via `npm run build` (0 errors), `cargo test` (34/34 unit tests passed), `cargo build --release` (clean compilation in 1m 48s), and confirmed live release binary `orbit.exe` running with PID 21768.

### Phase 11: Dynamic Section 2 Multi-Task Hub, Full-Cell Colored Calendar Grid & Eye-Friendly Typography Upgrade [COMPLETED]
- [x] **Task 1**: Created `src/components/island/MultiTaskSection.tsx` with a 2x2 Bento Grid for Media, Live Call/Privacy, Focus Timer & Shelf, and Today's Schedule. Integrated dynamic section spawning at Dot 2 (`dynamicSections`) in `src/components/IslandApp.tsx` when 2+ concurrent activities are active, with automatic dot clamping and navigation.
- [x] **Task 2**: Unclipped the Home top bar in `src/components/island/HomeSection.tsx` by locking right-hand `[🔔 N]` and `[⚙️]` buttons with `shrink-0`, repositioning `todayNextEvent` (`📅 Tantrafiesta Technical Fest`) underneath the date as a centered `11.5px` pill badge, and boosting font sizes across the clock screen and focus timers (`11px` bold).
- [x] **Task 3**: Replaced micro-dots on the 7-column calendar matrix in `src/components/island/CalendarSection.tsx` with full-cell category color highlights (`#fb7185` Rose-Red for Exams, `#34d399` Emerald-Green for Holidays, `#fbbf24` Amber-Gold for Fests/Events, `#f97316` Coral-Orange for Deadlines, `#38bdf8` Sky-Blue for Classes). Synchronized the Left 3D Date Hero Card's month title and bottom action badge with the selected date's primary event category.
- [x] **Task 4**: Verified via `npm run build` (0 errors across 1955 modules), `cargo test` (34/34 unit tests passed in 0.48s), `cargo build --release` (clean compilation in 2m 06s), and confirmed live release binary `orbit.exe` running with PID 24568.

### Phase 12: The Elite Theme Engine & Visual Showcase Overhaul [COMPLETED]
- [x] **Task 1**: Complete theme token system & 11-theme curated lineup in `src/core/theme.ts` and `src/styles/globals.css` (Apple Dynamic Island Pure OLED, Linear/Raycast Studio, VisionOS Spatial Glass, Catppuccin Mocha, Cyberpunk Neo-Tokyo, Sunset Velvet & Amber, Nordic Botanical Emerald, Arc Ocean & Solar Gold, and Apple Ceramic Alabaster).
- [x] **Task 2**: Deep theme token binding in Island components (`HomeSection.tsx`, `MultiTaskSection.tsx`, `CalendarSection.tsx`) replacing hardcoded gray/white classes with dynamic tokens (`--island-card-bg`, `--island-card-border`, `--island-text`, `--island-muted`, `--theme-accent`).
- [x] **Task 3**: Interactive visual Bento Theme Showcase Gallery in `src/components/SettingsApp.tsx` with live mini Dynamic Island previews, 3-color swatches, and 1-click instant live preview selection.
- [x] **Task 4**: Configuration validation and backward compatibility in `src-tauri/src/services/config.rs` and `src/core/types/config.ts`, verified via `npm run build` (0 errors), `cargo test` (34/34 passing), `cargo build --release` (clean in 1m 52s), and live launch of `orbit.exe` under PID 8860.
