# ORBIT — Execution Plan for 3.8 Flash (Phase 11: Dynamic Section 2 Multi-Task Hub, Full-Cell Colored Calendar Grid & Eye-Friendly Typography Upgrade)

> **Instructions for 3.8 Flash**: Execute Tasks 1 through 4 in exact order. Do NOT skip any file or edge case. After completing all code edits, run `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo build --release --manifest-path src-tauri/Cargo.toml`, then relaunch `src-tauri\target\release\orbit.exe`.

---

## Task 1: Dynamic Section 2 — Multi-Task Bento Hub (`src/components/island/MultiTaskSection.tsx` & `src/components/IslandApp.tsx`)

### Goal
When the user has multiple activities/contexts running simultaneously (Music + Call/Mic/Camera + Focus Timer + Held Shelf Files + Today's Schedule Event), automatically spawn a temporary **Section 2 (Dot 2, right below Home)** on the vertical section rail so the Home top bar never gets crowded or clipped, and give the user a clean 2×2 Bento dashboard with readable `11.5px–12px` text.

### 1.1 Create `src/components/island/MultiTaskSection.tsx`
Create a new component file `src/components/island/MultiTaskSection.tsx` with the following exact implementation:

```tsx
import React from "react";
import { ThemeMeta } from "../../core/theme";
import { MediaSessionSnapshot } from "../../core/types/media";
import { PrivacyState } from "../../core/types/notifications";
import { ReminderItem } from "../../core/types/reminders";
import { ShelfItem } from "../../core/types/shelf";
import { ipc } from "../../core/ipc/client";

interface MultiTaskSectionProps {
  activeThemeMeta: ThemeMeta;
  isMediaActive: boolean;
  mediaSession: MediaSessionSnapshot | null;
  onMediaCommand: (cmd: string) => void;
  onOpenMediaHome: () => void;
  privacyState: PrivacyState | null;
  callDurationSecs: number | null;
  timerRemainingSecs: number;
  isTimerPaused: boolean;
  timerLabel: string;
  onStartTimer: (secs: number, label?: string) => void;
  onPauseTimer: () => void;
  onResumeTimer: () => void;
  onStopTimer: () => void;
  shelfItems: ShelfItem[];
  onOpenShelf: () => void;
  todayNextEvent: ReminderItem | null;
  onOpenCalendarSection: () => void;
  formatSeconds: (secs: number) => string;
}

export const MultiTaskSection: React.FC<MultiTaskSectionProps> = ({
  activeThemeMeta,
  isMediaActive,
  mediaSession,
  onMediaCommand,
  onOpenMediaHome,
  privacyState,
  callDurationSecs,
  timerRemainingSecs,
  isTimerPaused,
  timerLabel,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
  shelfItems,
  onOpenShelf,
  todayNextEvent,
  onOpenCalendarSection,
  formatSeconds,
}) => {
  const isCallOrSensorActive = Boolean(
    privacyState?.cameraActive || privacyState?.microphoneActive
  );
  const activeSensorApp =
    privacyState?.cameraApp || privacyState?.microphoneApp || "System Audio/Video";

  const eventCategoryColor =
    todayNextEvent?.category === "exam"
      ? "#fb7185"
      : todayNextEvent?.category === "deadline"
      ? "#f97316"
      : todayNextEvent?.category === "holiday"
      ? "#34d399"
      : todayNextEvent?.category === "event" || todayNextEvent?.category === "meeting"
      ? "#fbbf24"
      : todayNextEvent?.category === "class"
      ? "#38bdf8"
      : activeThemeMeta.color;

  return (
    <div className="w-full h-full flex flex-col justify-between select-none pr-4 py-0.5">
      {/* Header Row */}
      <div className="flex items-center justify-between pb-1 border-b border-white/10 shrink-0">
        <div className="flex items-center space-x-1.5">
          <span
            className="w-2 h-2 rounded-full animate-pulse"
            style={{ backgroundColor: activeThemeMeta.accent }}
          />
          <span className="text-[11.5px] font-bold tracking-tight text-white/95">
            Live Multi-Task Hub
          </span>
        </div>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onOpenShelf();
          }}
          className="px-2 py-0.5 rounded-full orbit-pill-badge-3d text-[10.5px] font-semibold text-blue-300 hover:text-white transition-colors cursor-pointer flex items-center space-x-1"
          title="Open Held Files in Shelf"
        >
          <span>📁 Shelf ({shelfItems.length})</span>
        </button>
      </div>

      {/* 2x2 Bento Grid for Concurrent Tasks */}
      <div className="grid grid-cols-2 gap-2 flex-1 my-1 min-h-0">
        {/* Slot 1: Media Controller Card */}
        <div
          onClick={(e) => {
            e.stopPropagation();
            if (isMediaActive) onOpenMediaHome();
          }}
          className="rounded-xl orbit-card-3d p-2 flex flex-col justify-between min-w-0 cursor-pointer hover:border-white/25 transition-colors"
        >
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-bold uppercase tracking-wider text-amber-300">
              {mediaSession?.playback_status === "playing" ? "♫ Playing" : "♫ Media"}
            </span>
            {isMediaActive && (
              <div
                className="flex items-center space-x-1"
                onClick={(e) => e.stopPropagation()}
              >
                <button
                  type="button"
                  onClick={() => onMediaCommand("PlayPause")}
                  className="w-5 h-5 rounded-full bg-white/15 hover:bg-white/25 flex items-center justify-center text-[10px] text-white cursor-pointer"
                  title="Play / Pause"
                >
                  {mediaSession?.playback_status === "playing" ? "⏸" : "▶"}
                </button>
                <button
                  type="button"
                  onClick={() => onMediaCommand("Next")}
                  className="w-5 h-5 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-[10px] text-white/85 cursor-pointer"
                  title="Next Track"
                >
                  ⏭
                </button>
              </div>
            )}
          </div>
          <div className="min-w-0">
            <div className="text-[11.5px] font-bold text-white truncate">
              {isMediaActive && mediaSession
                ? mediaSession.track.title || "Unknown Track"
                : "No Active Media"}
            </div>
            <div className="text-[10px] font-medium text-white/65 truncate">
              {isMediaActive && mediaSession
                ? mediaSession.track.artist || mediaSession.source_app_id
                : "Play music to control here"}
            </div>
          </div>
        </div>

        {/* Slot 2: Live Call & Privacy Sensor Card */}
        <div
          onClick={(e) => {
            e.stopPropagation();
            if (isCallOrSensorActive) {
              const app =
                privacyState?.microphoneApp || privacyState?.cameraApp || "";
              if (app) ipc.activateAppSource("", app, null).catch(() => {});
            }
          }}
          className="rounded-xl orbit-card-3d p-2 flex flex-col justify-between min-w-0 cursor-pointer hover:border-white/25 transition-colors"
        >
          <div className="flex items-center justify-between">
            <span
              className="text-[10px] font-bold uppercase tracking-wider"
              style={{ color: isCallOrSensorActive ? "#34d399" : "rgba(255,255,255,0.5)" }}
            >
              {isCallOrSensorActive ? "● Live Call / Mic" : "🛡️ Privacy Guard"}
            </span>
            {callDurationSecs !== null && callDurationSecs > 0 && (
              <span className="text-[10.5px] font-mono font-bold text-emerald-300 tabular-nums">
                {formatSeconds(callDurationSecs)}
              </span>
            )}
          </div>
          <div className="min-w-0">
            <div className="text-[11.5px] font-bold text-white truncate">
              {isCallOrSensorActive ? activeSensorApp : "Cam & Mic Idle"}
            </div>
            <div className="text-[10px] font-medium text-white/65 truncate">
              {isCallOrSensorActive
                ? `${privacyState?.cameraActive ? "Camera " : ""}${
                    privacyState?.microphoneActive ? "Mic " : ""
                  }Active · Click to Focus`
                : "Hardware sensors protected"}
            </div>
          </div>
        </div>

        {/* Slot 3: Focus Timer Controller Card */}
        <div className="rounded-xl orbit-card-3d p-2 flex flex-col justify-between min-w-0">
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-bold uppercase tracking-wider text-sky-300">
              ⏱ {timerLabel} Timer
            </span>
            {timerRemainingSecs > 0 ? (
              <div className="flex items-center space-x-1">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    if (isTimerPaused) onResumeTimer();
                    else onPauseTimer();
                  }}
                  className="px-1.5 py-0.2 rounded bg-white/15 hover:bg-white/25 text-[9.5px] font-bold text-white cursor-pointer"
                >
                  {isTimerPaused ? "▶" : "⏸"}
                </button>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onStopTimer();
                  }}
                  className="px-1.5 py-0.2 rounded bg-rose-500/20 hover:bg-rose-500/35 text-[9.5px] font-bold text-rose-300 cursor-pointer"
                >
                  ✕
                </button>
              </div>
            ) : null}
          </div>
          {timerRemainingSecs > 0 ? (
            <div className="text-[15px] font-extrabold text-amber-300 tabular-nums leading-tight">
              {formatSeconds(timerRemainingSecs)}
            </div>
          ) : (
            <div className="flex items-center space-x-1 pt-0.5">
              {[
                { l: "15m", s: 900 },
                { l: "25m", s: 1500 },
                { l: "45m", s: 2700 },
              ].map((preset) => (
                <button
                  key={preset.l}
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onStartTimer(preset.s, timerLabel);
                  }}
                  className="px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-[10.5px] font-bold text-white/90 cursor-pointer transition-colors"
                >
                  {preset.l}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Slot 4: Today's Academic / Office Schedule Card */}
        <div
          onClick={(e) => {
            e.stopPropagation();
            onOpenCalendarSection();
          }}
          className="rounded-xl orbit-card-3d p-2 flex flex-col justify-between min-w-0 cursor-pointer hover:border-white/25 transition-colors"
        >
          <div className="flex items-center justify-between">
            <span
              className="text-[10px] font-bold uppercase tracking-wider truncate"
              style={{ color: eventCategoryColor }}
            >
              📅 {todayNextEvent ? (todayNextEvent.category || "Event").toUpperCase() : "Schedule"}
            </span>
            <span className="text-[9.5px] font-semibold text-white/50">Calendar →</span>
          </div>
          <div className="min-w-0">
            <div className="text-[11.5px] font-bold text-white truncate">
              {todayNextEvent ? todayNextEvent.title : "No Events Today"}
            </div>
            <div className="text-[10px] font-medium text-white/65 truncate">
              {todayNextEvent
                ? new Date(todayNextEvent.dueTimestampMs).toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                  })
                : "Click to view full month grid"}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
```

### 1.2 Wire Dynamic Section 2 (Option A) in `src/components/IslandApp.tsx`
1. Import `MultiTaskSection` at the top of `src/components/IslandApp.tsx`:
   ```ts
   import { MultiTaskSection } from "./island/MultiTaskSection";
   ```
2. Compute `hasMultiTaskSection` and `dynamicSections` inside `IslandApp` (right after `todayNextEvent` around line 235):
   ```ts
   const hasMultiTaskSection = useMemo(() => {
     const activeCount = [
       Boolean(
         mediaSession &&
           (mediaSession.playback_status === "playing" ||
             (mediaSession.playback_status === "paused" &&
               mediaSession.track.title.trim().length > 0))
       ),
       Boolean(privacyState?.cameraActive || privacyState?.microphoneActive),
       Boolean(timerDeadlineMs !== null || isTimerPaused),
       shelfItems.length > 0,
       Boolean(todayNextEvent),
     ].filter(Boolean).length;
     return activeCount >= 2;
   }, [mediaSession, privacyState, timerDeadlineMs, isTimerPaused, shelfItems.length, todayNextEvent]);

   const dynamicSections = useMemo(() => {
     const list: { id: "home" | "multitask" | "calendar" | "hardware"; label: string }[] = [
       { id: "home", label: "Home" },
     ];
     if (hasMultiTaskSection) {
       list.push({ id: "multitask", label: "Live Multi-Task Hub" });
     }
     list.push({ id: "calendar", label: "Calendar" });
     list.push({ id: "hardware", label: "Hardware Monitor" });
     return list;
   }, [hasMultiTaskSection]);

   useEffect(() => {
     if (activeSection >= dynamicSections.length) {
       setActiveSection(Math.max(0, dynamicSections.length - 1));
     }
   }, [activeSection, dynamicSections.length]);
   ```
3. In `handleWheel` (around line 1760 of `src/components/IslandApp.tsx`), replace `SECTIONS.length - 1` with `dynamicSections.length - 1`.
4. In the 3D Section Indicator Rail (around line 2417 of `src/components/IslandApp.tsx`), replace `SECTIONS.map` with `dynamicSections.map`.
5. In the 3D Vertical Drum Stage (around line 2453 of `src/components/IslandApp.tsx`), replace `{[0, 1, 2].map((sectionIdx) => {` with `{dynamicSections.map((section, sectionIdx) => {` and switch on `section.id`:
   - `section.id === "home"` -> `<HomeSection ... onOpenCalendarSection={() => { const calIdx = dynamicSections.findIndex((s) => s.id === "calendar"); setActiveSection(calIdx >= 0 ? calIdx : 1); }} />`
   - `section.id === "multitask"` ->
     ```tsx
     <MultiTaskSection
       activeThemeMeta={activeThemeMeta}
       isMediaActive={isMediaActive}
       mediaSession={mediaSession}
       onMediaCommand={handleMediaCommand}
       onOpenMediaHome={() => {
         setActiveSection(0);
       }}
       privacyState={privacyState}
       callDurationSecs={callDurationSecs}
       timerRemainingSecs={timerRemainingSecs}
       isTimerPaused={isTimerPaused}
       timerLabel={timerLabel}
       onStartTimer={startTimer}
       onPauseTimer={pauseTimer}
       onResumeTimer={resumeTimer}
       onStopTimer={stopTimer}
       shelfItems={shelfItems}
       onOpenShelf={() => {
         setActiveSection(0);
         setShowFileShelf(true);
       }}
       todayNextEvent={todayNextEvent}
       onOpenCalendarSection={() => {
         const calIdx = dynamicSections.findIndex((s) => s.id === "calendar");
         setActiveSection(calIdx >= 0 ? calIdx : 1);
       }}
       formatSeconds={formatSeconds}
     />
     ```
   - `section.id === "calendar"` -> `<CalendarSection ... />`
   - `section.id === "hardware"` -> `<HardwareSection ... />`

---

## Task 2: Unclip Home Top Bar, Reposition Today's Event & Enlarge Typography (`src/components/island/HomeSection.tsx`)

### Goal
Fix the clipped Settings gear `[⚙️]` on the Clock screen (`media_1792848776945_c205eb92.png`) by removing `todayNextEvent` from the narrow top bar and placing it cleanly right underneath `Saturday, 24 October` in the center stage, while enlarging small text across the Clock and Focus Timer bar so the user's eyes never strain.

### Exact Changes in `src/components/island/HomeSection.tsx`
1. **Top Bar Cleanup (Lines 958–1109)**:
   - Remove `{todayNextEvent && (...)}` from the top-left `div` (lines 1016–1041).
   - Add `min-w-0` to the top-left `div` and `shrink-0` to the top-right `div` so the right-side `[🔔 N]` and `[⚙️]` buttons can **never** be pushed or clipped off-screen.
   - Increase top-bar pill text from `text-[10px]` to `text-[11px] font-bold` (`Music`, `Call`, `Shelf`, and notification count).
2. **Center Stage: Clock, Larger Date & Centered Event Pill (Lines 1112–1142)**:
   - Increase the Date Subtitle (`{formattedDate}`) from `text-[12.5px]` muted grey to `text-[13.5px] font-semibold text-white/90`.
   - Right underneath `{formattedDate}` inside the center stage, render `todayNextEvent` (if present) as a sleek, centered, comfortable `11.5px` pill badge with plenty of horizontal room (`max-w-[280px]`):
     ```tsx
     {todayNextEvent && (
       <button
         type="button"
         onClick={(e) => {
           e.stopPropagation();
           onOpenCalendarSection?.();
         }}
         className="relative z-10 mt-1 px-3 py-0.5 rounded-full orbit-pill-badge-3d flex items-center space-x-1.5 hover:bg-white/15 transition-colors cursor-pointer max-w-[280px]"
         style={{
           color:
             todayNextEvent.category === "exam"
               ? "#fb7185"
               : todayNextEvent.category === "deadline"
               ? "#f97316"
               : todayNextEvent.category === "holiday"
               ? "#34d399"
               : todayNextEvent.category === "meeting" || todayNextEvent.category === "event"
               ? "#fbbf24"
               : todayNextEvent.category === "class"
               ? "#38bdf8"
               : activeThemeMeta.color,
         }}
         title={`Today's Schedule: ${todayNextEvent.title} — Click to open Calendar`}
       >
         <span className="text-[11px] shrink-0">📅</span>
         <span className="text-[11.5px] font-bold truncate">
           {todayNextEvent.title}
         </span>
       </button>
     )}
     ```
3. **Focus Timer Bar Typography Boost (Lines 1190–1303)**:
   - Increase `FOCUS:` cycle button from `text-[9.5px]` to `text-[11px] font-extrabold px-2 py-0.5 text-white/90`.
   - Increase `5m`, `15m`, `25m`, `45m` preset buttons from `text-[9.5px] text-white/70` to `text-[11px] font-bold px-2.5 py-0.5 text-white/90` so they are crisp and effortless to read.

---

## Task 3: Full-Cell Colored Calendar Grid & Left Hero Card Sync (`src/components/island/CalendarSection.tsx`)

### Goal
Replace the bottom micro-dots on the 7-column calendar grid with **Full-Cell Category Color Highlighting** (matching the user's IIITN Academic Calendar PDF and `media_1792849119128_1ed79c05.png` feedback), synchronize the Left 3D Date Hero Card with the selected date's event color, and boost font sizes across the Calendar grid and drawer.

### Exact Changes in `src/components/island/CalendarSection.tsx`
1. **Enhanced Category Palette Helper (`getCategoryStyle`)**:
   Give every academic/office category a distinct, high-contrast color and badge:
   - `exam`: `{ hex: "#fb7185", bg: "rgba(251, 113, 133, 0.28)", border: "#fb7185", text: "#ffe4e6", badge: "EXAM" }` (Rose-Red)
   - `deadline` / `due`: `{ hex: "#f97316", bg: "rgba(249, 115, 22, 0.28)", border: "#f97316", text: "#ffedd5", badge: "DUE" }` (Coral-Orange)
   - `holiday`: `{ hex: "#34d399", bg: "rgba(52, 211, 153, 0.28)", border: "#34d399", text: "#d1fae5", badge: "HOLIDAY" }` (Emerald-Green)
   - `event`: `{ hex: "#fbbf24", bg: "rgba(251, 191, 36, 0.28)", border: "#fbbf24", text: "#fef3c7", badge: "EVENT" }` (Amber-Gold — for Fests like Tantrafiesta, Abhivyakti, E-Summit)
   - `meeting`: `{ hex: "#a855f7", bg: "rgba(168, 85, 247, 0.28)", border: "#a855f7", text: "#f3e8ff", badge: "MEETING" }` (Violet)
   - `class`: `{ hex: "#38bdf8", bg: "rgba(56, 189, 248, 0.28)", border: "#38bdf8", text: "#e0f2fe", badge: "CLASS" }` (Sky-Blue)
   - default (`task`): `{ hex: themeColor, bg: "rgba(255, 255, 255, 0.14)", border: themeColor, text: "#ffffff", badge: "TASK" }`

2. **Priority Category Resolver for Any Date (`getDayPrimaryCategory`)**:
   Write a helper that takes a list of `ReminderItem[]` for a date and returns the highest-priority category style (`exam` > `deadline` > `holiday` > `event` > `meeting` > `class` > `task`), or `null` if that date has no events.

3. **Full-Cell Highlighting on the 7-Column Calendar Matrix**:
   - Remove the `<span className="w-1.5 h-1.5 rounded-full absolute -bottom-0.5" ... />` bottom dot completely so rows never collide!
   - Make each date cell `h-[20px] w-[24px] mx-auto rounded-full flex items-center justify-center relative text-[11px] font-bold tabular-nums transition-all cursor-pointer`.
   - Style each date cell as follows:
     - **If `isSelected`**:
       - `backgroundColor`: `dayStyle ? dayStyle.hex : activeThemeMeta.accent`
       - `color`: `"#090a0f"` (if bright fill like `#34d399`, `#fbbf24`, `#38bdf8`) or `"#ffffff"`
       - `boxShadow`: `0 0 0 1.5px #ffffff, 0 2px 10px ${dayStyle ? dayStyle.hex : activeThemeMeta.accent}90`
     - **Else if `dayStyle` (Unselected Date with Event)**:
       - `backgroundColor`: `dayStyle.bg`
       - `border`: `1px solid ${dayStyle.border}`
       - `color`: `dayStyle.text`
       - `boxShadow`: `0 1px 6px ${dayStyle.hex}35`
     - **Else if `isToday` (Unselected Today with No Event)**:
       - `border`: `1.5px solid ${activeThemeMeta.accent}`
       - `color`: `activeThemeMeta.accent`

4. **Left 3D Date Hero Card Sync & Larger Text**:
   - Compute `selectedDayStyle = getDayPrimaryCategory(selectedDateReminders, activeThemeMeta.color)`.
   - Color the Month header (`October`) with `selectedDayStyle ? selectedDayStyle.hex : activeThemeMeta.color` and increase its font size to `text-[12px] font-extrabold`.
   - Increase the sculpted 3D Day Number (`24`) to `text-[40px]` and tint its subtle text-shadow when `selectedDayStyle` is present.
   - Increase Weekday (`Saturday`) to `text-[11px] font-semibold text-white/75`.
   - Style the bottom toggle button (`1 Event` / `Holiday` / `1 Exam` / `+ Add Task`) with `selectedDayStyle`'s background, border, and color (`text-[9.5px] font-bold px-2.5 py-0.5`), and if `selectedDateReminders.length === 1`, display the specific badge label (e.g. `1 Exam`, `Holiday`, `1 Event`, `1 Due`, `1 Class`) so the user immediately sees the category on the Left Card!

5. **Schedule Drawer Readability Boost**:
   - Increase drawer item title from `text-[10px]` to `text-[11.5px] font-semibold`.
   - Increase category badge from `text-[7.5px]` to `text-[8.5px]` and time label from `text-[8.5px]` to `text-[9.5px]`.
   - Include `"event"` and `"holiday"` in the new-task category cycle button (`["task", "exam", "event", "holiday", "meeting", "class", "deadline"]`).

---

## Task 4: Build, Test & Live Release Verification
1. Run `npm run build` — verify 0 TypeScript and Vite build errors.
2. Stop running `orbit.exe` (`Stop-Process -Name orbit -Force -ErrorAction SilentlyContinue`).
3. Run `cargo test --manifest-path src-tauri/Cargo.toml` — verify all 34 unit tests pass.
4. Run `cargo build --release --manifest-path src-tauri/Cargo.toml`.
5. Launch `src-tauri\target\release\orbit.exe` and verify the process is running cleanly.
