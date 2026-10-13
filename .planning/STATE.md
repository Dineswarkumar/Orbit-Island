# ORBIT Project State

- **Current Milestone:** Milestone 2 - Core Shell (IMPLEMENTED & READY FOR USER TESTING)
- **Active Branch:** `feat/core-shell`
- **Environment:** Windows 11 (26300), Node 24.21.0, npm 11.19.0, Rust 1.98.1 (MSVC), Tauri CLI 2.12.2, WebView2 154.0.4258.62
- **Core Shell Deliverables:**
  - **Minimal Compact Pill (200x56 px):** Centered live time only (hours and minutes) with readable secondary AM/PM in prominent 20px typography. Completely free of dates, dots, wordmarks, settings icons, and placeholder/mock weather. Footprint kept at 200x56 px.
  - **Expanded Panel Breathing Room & Content Scale (500x230 px):** Native window and webview expanded to 500x230 px providing relaxed margins. Interior elements scaled down by ~20% (clock font at 30px, no seconds). Header displays the canonical ORBIT logo asset preserving its natural aspect ratio with a discreet collapse button. Removed "ORBIT HOME" text, theme badges, settings icon, and instructional footer text.
  - **Compacting Glitch Resolution:** Content exit opacity/position coordinated with container shrink. The 500x230 expanded view drops opacity cleanly without vertical squishing/reflowing, and the compact view delays entry until the container reaches compact capsule dimensions. Fully eliminates text flying or sliding underneath the pill.
  - **Hover-Driven Expansion & Auto-Collapse:** Hovering the mouse expands the island; moving away collapses it with a 350ms debounce.
  - **Right-Click Context Menu:** Theme selection and ORBIT Settings available exclusively via right-click context menu.
  - **Settings Window Close/Reopen Fix:** Native `CloseRequested` intercepted to hide rather than destroy the webview.
- **Verification Status:** Code implementation, TypeScript build, and Tauri release build completed. Pending user physical review and manual checklist verification.
- **Next Milestone:** Milestone 3 - Feature Slices. First planned slice: Windows Media Session Integration (GSMTC), where active media playback replaces the large Home clock per PRD priority. Implementation will begin only after physical review.
