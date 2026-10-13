# ORBIT Project State

- **Current Milestone:** Milestone 2 - Core Shell (IMPLEMENTED & READY FOR USER TESTING)
- **Active Branch:** `feat/core-shell`
- **Environment:** Windows 11 (26300), Node 24.21.0, npm 11.19.0, Rust 1.98.1 (MSVC), Tauri CLI 2.12.2, WebView2 154.0.4258.62
- **Core Shell Deliverables:**
  - **Compact & Expanded Interaction:** Clicking the pill triggers smooth, instantaneous expansion to 480x220 px. Collapsing returns to 400x56 px.
  - **Zero-Flicker Native Window Synchronization:** IPC command `set_island_state` dynamically resizes the native window, re-anchors to top-center, and recalculates the Win32 `SetWindowRgn` capsule/rounded region (`CreateRoundRectRgn`) for zero click-stealing.
  - **Live System Clock:** Accurate 1-second interval clock supporting 12h (AM/PM) and 24h formats based on persisted config.
  - **Persistent Countdown Timer:** Dedicated timer store supporting 5m, 15m, 25m presets and custom countdowns. Tracks `targetEndTime` in local storage for deadline recovery across restarts. Active timer replaces the compact clock slot according to product rules.
  - **Preserved Wiring:** Tray menu, Settings window trigger, and configuration persistence remain completely wired and functional.
- **Next Milestone:** Milestone 3 - Feature Slices (Media GSMTC, Calendar, Productivity Hub, Hardware Monitor, Notifications).
