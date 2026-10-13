# ORBIT Project State

- **Current Milestone:** Milestone 2 - Core Shell (IMPLEMENTED & READY FOR USER TESTING)
- **Active Branch:** `feat/core-shell`
- **Environment:** Windows 11 (26300), Node 24.21.0, npm 11.19.0, Rust 1.98.1 (MSVC), Tauri CLI 2.12.2, WebView2 154.0.4258.62
- **Core Shell Deliverables:**
  - **Refined Balanced Compact Pill (200x56 px):** Sized at 200x56 px divided into two clear, balanced halves with a subtle hairline divider. Left half displays the live clock (+2–3px larger font at 18px with secondary AM/PM). Right half displays a concise date/day indicator (`formatCompactDate`) without mock/fake weather data. Pulsating decorative dot and compact Settings icon completely removed.
  - **Synchronized Fluid Expand & Collapse Motion:** Eliminated rectangular client area flashes and snapped corners. Native window sizing and Win32 `SetWindowRgn` rounded clipping (consistent 56px corner diameter matching CSS `28px` corner radius) are synchronized with a 250ms cubic-bezier transition, delivering fluid expansion on hover and smooth auto-collapse on mouse leave while maintaining click-through outside the pill in both states.
  - **Hover-Driven Expansion & Auto-Collapse:** Hovering the mouse over the compact pill triggers expansion to 480x220 px. Moving the mouse away auto-collapses back to the 200px pill with a debounce timeout.
  - **Right-Click Context Menu:** Right-clicking anywhere on the expanded island opens an in-island context menu offering theme selection (Obsidian, Espresso, Evergreen, Nord, Porcelain Light) and direct access to ORBIT Settings.
  - **Home Island Focus:** Retains dedicated live clock/date prominence on the Home panel with activity slot priority, without timer controls on Home.
  - **Settings Window Close/Reopen Fix:** Fixed defect where closing the Settings window destroyed the webview and prevented reopening. Native `CloseRequested` window event is intercepted with `api.prevent_close()` to hide the window instead, allowing Settings to be repeatedly opened from the island and tray menu without app restarts.
- **Next Milestone:** Milestone 3 - Feature Slices (Media GSMTC, Calendar, Productivity Hub, Hardware Monitor, Notifications).
