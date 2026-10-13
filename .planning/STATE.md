# ORBIT Project State

- **Current Milestone:** Milestone 1 - Foundation (AUDITED & DEFECTS FIXED)
- **Active Branch:** `feat/foundation-milestone`
- **Environment:** Windows 11 (26300), Node 24.21.0, npm 11.19.0, Rust 1.98.1 (MSVC), Tauri CLI 2.12.2, WebView2 154.0.4258.62
- **Audit Adjustments & Defect Fixes:**
  - **Startup Latency:** Re-measured from process launch to visible rendered window using Win32 window enumeration. Cold run: 198 ms; warm runs: 143-356 ms. Meets < 2s target.
  - **Click-Through Fix:** Identified defect where transparent canvas (700x450) captured clicks over underlying windows despite CSS `pointer-events: none`. Fixed for Foundation by sizing the window to the exact compact pill footprint (400x56), eliminating transparent click-stealing.
  - **Memory Accounting Audit:** Mapped exact memory metrics across full 7-process tree (`orbit.exe` + 6 `msedgewebview2.exe` descendants). Main process Private Working Set is 7.39 MB. Total tree Private Working Set is 130.39 MB; total tree Private Bytes is 198.61 MB. Fails the 50 MB goal and 100 MB ceiling if counting the full Chromium runtime tree (largely driven by Chromium GPU process at ~60 MB).
  - **UI Scope Clean-Up:** Removed temporary developer memory badge from main island pill; memory diagnostics preserved strictly in Settings window.
  - **Manual Checks Audit:** Explicitly distinguished automated verification from physical human UI/tray testing; marked physical manual actions as unverified.
  - **Logo & Brand Assets:** Canonical brand asset preserved at `assets/orbit-logo.png` with original wordmark intact. Cropped squircle icon card preserved separately at `assets/orbit-icon.png` and generated into `src-tauri/icons/`.
  - **Memory Accounting Status:** Aspiration is < 30 MB, target is < 50 MB. 7-process attributable tree currently sits at ~124 MB Private Working Set (~90 MB with experimental `--in-process-gpu`). Current Foundation overage is accepted temporarily as development proceeds; `--in-process-gpu` is kept strictly experimental and not enabled in production. Foundation memory targets are explicitly tracked as not met.
  - **Human Verification:** TC-07 (Settings window interaction) and TC-08 (Tray menu interaction) remain explicitly marked UNVERIFIED pending user manual execution.
- **Next Milestone:** Milestone 2 - Core Shell (Approved to proceed under Tauri 2, React, TypeScript, and Rust).
