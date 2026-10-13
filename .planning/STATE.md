# ORBIT Project State

- **Current Milestone:** Milestone 2 - Core Shell (Implemented; Pending Physical Review)
- **Active Branch:** `feat/core-shell`
- **Environment:** Windows 11 (26300), Node 24.21.0, npm 11.19.0, Rust 1.98.1 (MSVC), Tauri CLI 2.12.2, WebView2 154.0.4258.62
- **Core Shell Deliverables:**
  - **Minimal Compact Pill (200x56 px):** Centered live time only (hours and minutes) with readable secondary AM/PM in prominent 20px typography. Completely free of dates, dots, wordmarks, settings icons, and placeholder/mock weather. Native footprint kept at 200x56 px with Win32 capsule region clipping.
  - **Expanded Idle Clock Island (400x184 px):** Reduced outer dimensions by 20% from 500x230 to 400x184 px. Directly grounded in `reference-idle-clock.png`: large bold time (34px), secondary AM/PM, formatted date underneath, and subtle 4-dot section navigation indicators on the right edge. Header displays canonical ORBIT logo asset with preserved aspect ratio and discreet collapse button.
  - **Race-Free Hover & Collapse State Machine:** Completely eliminates the delayed-collapse race condition. Generation-tracked transition IDs (`transitionIdRef`) and explicitly tracked timer references cancel pending collapse callbacks on pointer re-entry, preventing stale timers from resizing the window to compact while expanded.
  - **Clean Compacting Transition:** Expanded content pins to fixed 400x184 dimensions and fades out immediately (100ms) with zero vertical squishing or reflowing. Compact content delays entrance (150ms) until the container reaches capsule dimensions, fully eliminating text sliding or flying underneath the pill.
  - **Settings Scrollable Layout:** Constrained scrollable body with pinned header and footer. Content scrolls cleanly via mouse wheel, touchpad, and keyboard across 640x520 and smaller resized dimensions, while the Save action remains permanently visible. Body `overflow: hidden` is preserved for the transparent Island.
  - **Live Configuration Sync & Themes:** Saving preferences in Settings emits a `config-changed` Tauri event that updates the Island in real time without app restarts. Right-clicking the expanded island allows instant theme selection with visible border/accent glow feedback.
  - **Accurate Telemetry Reporting:** Corrected telemetry metric descriptions: explicitly labels `PrivateUsage` as Win32 Private Bytes (commit charge) and `WorkingSetSize` as Total Working Set. Does not falsely claim PRD private working set compliance based on commit charge.
- **Verification Status:** Frontend build, Rust check, and release build completed. Pending user physical review.
- **Next Milestone:** Milestone 3 - Feature Slices (First slice: Windows Media Session GSMTC integration, with active media replacing the Home clock).
