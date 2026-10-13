# ORBIT Project State

- **Current Milestone:** Milestone 2 - Core Shell (IMPLEMENTED & READY FOR USER TESTING)
- **Active Branch:** `feat/core-shell`
- **Environment:** Windows 11 (26300), Node 24.21.0, npm 11.19.0, Rust 1.98.1 (MSVC), Tauri CLI 2.12.2, WebView2 154.0.4258.62
- **Core Shell Deliverables:**
  - **Refined Compact Pill:** Compact footprint halved from 400x56 px to **200x56 px**. Removed "ORBIT" wordmark and settings button from compact mode. Increased clock text size for clean visibility.
  - **Hover-Driven Expansion & Auto-Collapse:** Hovering the mouse over the compact pill triggers expansion to 480x220 px. Moving the mouse away auto-collapses back to the 200px pill with a debounce timeout.
  - **Right-Click Context Menu:** Right-clicking anywhere on the expanded island opens an in-island context menu offering theme selection (Obsidian, Espresso, Evergreen, Nord, Porcelain Light) and direct access to ORBIT Settings.
  - **Home Island Focus:** Removed countdown timer options from the home panel per feedback, giving clean prominence to the large live clock/date display.
  - **Settings Window Close/Reopen Fix:** Fixed defect where closing the Settings window destroyed the webview and prevented reopening. Native `CloseRequested` window event is now intercepted with `api.prevent_close()` to hide the window instead, allowing Settings to be repeatedly opened from the island and tray menu without app restarts.
- **Next Milestone:** Milestone 3 - Feature Slices (Media GSMTC, Calendar, Productivity Hub, Hardware Monitor, Notifications).
