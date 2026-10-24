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
          <span
            className="text-[11.5px] font-bold tracking-tight"
            style={{ color: "var(--island-text)" }}
          >
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
            <div
              className="text-[11.5px] font-bold truncate"
              style={{ color: "var(--island-text)" }}
            >
              {isMediaActive && mediaSession
                ? mediaSession.track.title || "Unknown Track"
                : "No Active Media"}
            </div>
            <div
              className="text-[10px] font-medium truncate"
              style={{ color: "var(--island-muted)" }}
            >
              {isMediaActive && mediaSession
                ? mediaSession.track.artist || mediaSession.source_title || mediaSession.source_id
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
              style={{ color: isCallOrSensorActive ? "#34d399" : "var(--island-muted)" }}
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
            <div
              className="text-[11.5px] font-bold truncate"
              style={{ color: "var(--island-text)" }}
            >
              {isCallOrSensorActive ? activeSensorApp : "Cam & Mic Idle"}
            </div>
            <div
              className="text-[10px] font-medium truncate"
              style={{ color: "var(--island-muted)" }}
            >
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
                  className="px-2 py-0.5 rounded orbit-pill-badge-3d hover:bg-white/20 text-[10.5px] font-bold cursor-pointer transition-colors"
                  style={{ color: "var(--island-text)" }}
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
            <div
              className="text-[11.5px] font-bold truncate"
              style={{ color: "var(--island-text)" }}
            >
              {todayNextEvent ? todayNextEvent.title : "No Events Today"}
            </div>
            <div
              className="text-[10px] font-medium truncate"
              style={{ color: "var(--island-muted)" }}
            >
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
