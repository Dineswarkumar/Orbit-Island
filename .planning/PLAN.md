# ORBIT V1.0 Master Execution Blueprint for 3.8 Flash (`PLAN.md`)

> **HOW TO USE THIS FILE WITH 3.8 FLASH:**
> 1. Switch your Antigravity model selector dropdown to **3.8 Flash**.
> 2. Paste this prompt to 3.8 Flash:
>    `Read .planning/PLAN.md and execute Phase 1, Phase 2, Phase 3, and Phase 4 in order. Obey all Section 1 Guardrails, run verification after each phase, and rebuild/relaunch target\release\orbit.exe at the end.`

---

## 1. Non-Negotiable Guardrails for 3.8 Flash (DO NOT VIOLATE)

1. **Preserve Win32 GDI Breathing Margins (`src-tauri/src/services/window.rs`)**:
   - `compact`: `(126.0, 0.0, 274.0, 48.0, 46.0)`
   - `notification`: `(36.0, 0.0, 364.0, 74.0, 58.0)`
   - `expanded`: `(16.0, 0.0, 384.0, 180.0, 56.0)`
   - Never shrink these regions or call `enforce_frameless` during runtime state transitions.
2. **Preserve Modular Component Structure (`src/components/island/`)**:
   - Keep `CompactCapsule.tsx`, `HomeSection.tsx`, `CalendarSection.tsx`, `HardwareSection.tsx`, and `NotificationBrandIcon.tsx` modular. Do NOT dump large JSX blocks back into `IslandApp.tsx`.
3. **Preserve Studio Colors, Per-Metric Hardware Colors, & Zero Mouse Glow**:
   - Never re-add cursor spotlight radial gradients (`--spot-x`, `--spot-y`, `--glow-x`, `--glow-y`).
   - Keep the 6 distinct semantic colors in `HardwareSection.tsx` (CPU `#3b82f6`, NET UP `#10b981`, RAM `#8b5cf6`, NET DOWN `#06b6d4`, Battery `#22c55e`/`#f59e0b`/`#ef4444` with NO "Power" title, Disk `#f59e0b`).
   - Keep 1-decimal precision (`.toFixed(1)`) across all hardware percentages.
4. **Preserve Notification Drop-Expand-Rewind & Brand Icons**:
   - Keep the 3-stage notification choreography (`316×56`, `translate3d(0, 7px, 6px)`, `280ms` delayed compact region shrink) and `NotificationBrandIcon` + sender (`title`) & message (`body`) separation.
5. **Standalone Release Binary Rule (`src-tauri/Cargo.toml`)**:
   - Keep `default = ["custom-protocol"]` in `src-tauri/Cargo.toml` so `cargo build --release` always embeds `../dist` and never shows the WebView2 `localhost:1420` error.

---

## 2. Step-by-Step Phases to Complete ORBIT Version 1.0

### Phase 1 — Built-In Countdown / Focus Timer (PRD §5.3 & `reference-timer-and-media.png`)
- **Objective**: Allow the user to start/pause/cancel a quick countdown timer (e.g., 5m, 15m, 25m, 45m) from the right-click context menu or Expanded Home, persist the target deadline timestamp in `localStorage` so it survives restarts, and show a sleek circular progress ring + `MM:SS` countdown.
- **Files to Create / Modify**:
  - `src-tauri/src/lib.rs` (Context Menu Timer actions)
  - `src/components/IslandApp.tsx`
  - `src/components/island/CompactCapsule.tsx`
  - `src/components/island/HomeSection.tsx`
- **Exact Implementation Spec**:
  1. **Context Menu Timer Triggers (`src-tauri/src/lib.rs`)**:
     - In `show_context_menu` (or the context menu builder in `src-tauri/src/commands/mod.rs` / `lib.rs`), add a `"Timer"` submenu with options:
       - `ctx_timer_5`: `"Start 5m Timer"`
       - `ctx_timer_15`: `"Start 15m Timer"`
       - `ctx_timer_25`: `"Start 25m Focus"`
       - `ctx_timer_stop`: `"Cancel Timer"`
     - When clicked, emit `"timer-command"` event to the frontend with payload `{ action: "start", durationSecs: 300 | 900 | 1500 }` or `{ action: "stop" }`.
  2. **Deadline-Based Timer State (`src/components/IslandApp.tsx`)**:
     - Store `timerDeadlineMs: number | null` and `timerTotalSecs: number` (persisted in `localStorage` under `"orbit_timer_state"` so it survives app restarts per PRD §5.3).
     - Compute `timerRemainingSecs = timerDeadlineMs ? Math.max(0, Math.ceil((timerDeadlineMs - Date.now()) / 1000)) : 0`.
     - When `timerRemainingSecs` reaches `0` from an active countdown, clear `timerDeadlineMs` and trigger a high-priority transient notification (`title: "Timer Complete"`, `body: "Your countdown timer has finished."`, `isCritical: true`).
  3. **Compact & Expanded Display (`CompactCapsule.tsx` & `HomeSection.tsx`)**:
     - In `CompactCapsule.tsx`: When a timer is active (and no transient notification is overriding), show a sleek circular SVG progress ring (`#f59e0b` amber) + formatted `MM:SS` countdown next to the clock (or media).
     - In `HomeSection.tsx`: Add a subtle quick-timer pill bar (`5m`, `15m`, `25m`, or `Stop MM:SS` when active) at the bottom of the idle Home view (when privacy pill is not occupying the slot) so users can also start/stop a timer with one click inside the expanded island.

---

### Phase 2 — File Shelf Drawer & Drag-and-Drop Access in Expanded Home (PRD §5.8)
- **Objective**: Connect the existing SQLite `file_shelf` backend (`getShelfItems`, `addShelfItem`, `removeShelfItem`, `openShelfItem`, `revealShelfItem`, `pickShelfFile` in `src/core/ipc/client.ts`) to a clean, toggleable File Shelf view inside Expanded Home (`HomeSection.tsx`) and auto-open it on file drag-over.
- **Files to Create / Modify**:
  - Create `src/components/island/FileShelfDrawer.tsx`
  - Modify `src/components/island/HomeSection.tsx` and `src/components/IslandApp.tsx`
- **Exact Implementation Spec**:
  1. Create `src/components/island/FileShelfDrawer.tsx`:
     - Displays held files (`ShelfItem[]` from `src/core/types/shelf.ts`) in a compact scrollable list inside the `356×168` expanded canvas.
     - Header bar: `"File Shelf ({items.length}/10)"`, `"+ Add File"` button (calls `ipc.pickShelfFile()`), `"Clear"` button (`ipc.clearShelf()`), and `"← Back"` button.
     - Each file row (`orbit-well-3d rounded-lg px-2 py-1 flex items-center justify-between`):
       - Extension badge (`PDF`, `PNG`, `ZIP`, etc. in `#60a5fa`), truncated file name, and quick icon buttons for **Open** (`ipc.openShelfItem(id)`), **Reveal in Explorer** (`ipc.revealShelfItem(id)`), and **Remove** (`ipc.removeShelfItem(id)` — never deletes original file).
  2. In `src/components/island/HomeSection.tsx`:
     - Add a subtle top-left pill button (`Shelf ({shelfItems.length})`) on the Idle Home view to toggle `showFileShelf`.
  3. In `src/components/IslandApp.tsx`:
     - Load initial shelf items via `ipc.getShelfItems()`.
     - Listen to `"tauri://drag-drop"` (or Tauri v2 `tauri://drag-drop` payload `{ paths: string[] }`): when files are dropped onto the island, call `ipc.addShelfItem(path, "reference")`, refresh `shelfItems`, expand the island to Section 0 (`Home`), and set `showFileShelf(true)`.

---

### Phase 3 — Windows Sign-In Auto-Launch (`HKCU\...\Run`) & Data Reset Controls in Settings (PRD §6)
- **Objective**: Make `launchAtStartup` in `AppConfig` genuinely register/unregister `orbit.exe` in Windows CurrentUser Startup registry, and add a "Reset & Diagnostics" card in `SettingsApp.tsx`.
- **Files to Modify**:
  - `src-tauri/src/services/config.rs`
  - `src-tauri/src/commands/mod.rs`
  - `src-tauri/src/lib.rs`
  - `src/core/ipc/client.ts`
  - `src/components/SettingsApp.tsx`
- **Exact Implementation Spec**:
  1. **Windows Startup Registry Sync (`src-tauri/src/services/config.rs`)**:
     - Add `pub fn sync_startup_registry(enable: bool)`:
       - Under `#[cfg(target_os = "windows")]`, open `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` using `windows_sys::Win32::System::Registry::{RegOpenKeyExW, RegSetValueExW, RegDeleteValueW, RegCloseKey, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ}`.
       - If `enable == true`, set value `"ORBIT"` to the current executable path (`std::env::current_exe()`).
       - If `enable == false`, delete value `"ORBIT"` (ignoring not-found errors).
     - Call `sync_startup_registry(config.launch_at_startup)` inside `save_config` (`commands/mod.rs`).
  2. **Clear Local Data Command (`src-tauri/src/commands/mod.rs`)**:
     - Add `#[tauri::command] pub async fn clear_local_data(db_state: State<'_, Arc<DbManager>>) -> Result<(), String>` that clears `reminders` and `file_shelf` tables in `orbit.db` without touching original user files on disk.
  3. **Settings UI (`src/components/SettingsApp.tsx`)**:
     - Add a **"Startup & Reset"** section at the bottom of `SettingsApp.tsx` with:
       - Toggle for **Launch ORBIT at Windows Sign-In** (`config.launchAtStartup`).
       - Button for **Reset Settings to Defaults**.
       - Button for **Clear Local Reminders & Shelf References** (calls `ipc.clearLocalData()`).

---

### Phase 4 — Version 1.0.0 Release Bump, Final Test Suite & Production Binary Launch
- **Objective**: Bump version to `1.0.0`, run all unit tests and production builds, launch the final `1.0.0` `orbit.exe`, and update `.planning/STATE.md`, `.planning/ROADMAP.md`, and `.planning/UAT.md`.
- **Files to Modify**:
  - `package.json` (`"version": "1.0.0"`)
  - `src-tauri/Cargo.toml` (`version = "1.0.0"`)
  - `src-tauri/tauri.conf.json` (`"version": "1.0.0"`)
  - `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/UAT.md`
- **Verification & Launch Sequence (Mandatory)**:
  1. Run `npm run build` (must exit 0 with zero TypeScript or Vite errors).
  2. Run `cargo test --manifest-path src-tauri/Cargo.toml` (all unit tests must pass).
  3. Run `Stop-Process -Name orbit -Force -ErrorAction SilentlyContinue`.
  4. Run `cargo build --release --manifest-path src-tauri/Cargo.toml`.
  5. Run `Start-Process "T:\B-Tech\Projects\Orbit-island\src-tauri\target\release\orbit.exe"` and verify via `Get-Process orbit`.
  6. Update `.planning/STATE.md` and `.planning/ROADMAP.md` to record **ORBIT v1.0.0 Complete**.
