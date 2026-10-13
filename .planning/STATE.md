# ORBIT Project State

- **Current Milestone:** Milestone 1 - Foundation (AUDITED & DEFECTS FIXED)
- **Active Branch:** `feat/foundation-milestone`
- **Environment:** Windows 11 (26300), Node 24.21.0, npm 11.19.0, Rust 1.98.1 (MSVC), Tauri CLI 2.12.2, WebView2 154.0.4258.62
- **Audit Adjustments & Defect Fixes:**
  - **Startup Latency:** Re-measured from process launch to visible rendered window using Win32 window enumeration. Cold run: 198 ms; warm runs: 143-356 ms. Meets < 2s target.
  - **Tray Icon Handle Retention:** Retained tray handle in Tauri app-managed state (`app.manage(TrayState(Arc::new(tray)))`) with explicit ID `"orbit-tray"` to guarantee lifetime across the complete app lifecycle.
  - **Native Click-Through & Capsule Clipping:** Bounded main window to 400x56 px and applied native Win32 `SetWindowRgn` with `CreateRoundRectRgn`. Verified via Win32 `WindowFromPoint` hit-tests that points outside the window rectangle AND transparent corner pixels inside the 400x56 bounds physically pass through to the underlying desktop/windows, while the center hits the ORBIT island.
  - **Memory Accounting Audit & Reconciliation:** Single initial cold launch yielded 130.39 MB Private Working Set (PWS). Settled 3-launch idle audit established the average at 124.19 MB PWS (190.96 MB Private Bytes). Both figures reconciled and documented. Status: NOT MET (Overage accepted temporarily while development continues; target remains <50 MB and aspiration <30 MB).
  - **Settings Save Feedback & Runtime Application:** `updateConfig` now returns real persistence status with explicit error/success states in the UI. Foundation runtime settings (`alwaysOnTop`) dynamically update the native window on save. Planned controls (Theme, Density, Time Format, Launch at Startup) are explicitly labeled with their roadmap milestones (Core Shell / Installer) to prevent misleading implication of active behavior.
  - **Pending Human Verification:** TC-07 (physical Settings window interaction) and TC-08 (physical Tray icon mouse clicking) remain explicitly UNVERIFIED pending user manual testing.
- **Next Milestone:** Milestone 2 - Core Shell (Awaiting user review and approval).
