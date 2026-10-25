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
    <div className="w-full h-full flex flex-col select-none space-y-1 pb-0.5 pr-0.5">
      {/* Header Row */}
      <div className="flex items-center justify-between pb-1 border-b border-white/10 shrink-0">
        <div className="flex items-center space-x-1.5 min-w-0">
          <span
            className="w-2 h-2 rounded-full animate-pulse shrink-0"
            style={{ backgroundColor: activeThemeMeta.accent }}
          />
          <span
            className="text-[11px] font-bold tracking-tight truncate"
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
          className="px-2 py-0.5 rounded-full orbit-pill-badge-3d text-[10px] font-semibold text-blue-300 hover:text-white transition-colors cursor-pointer flex items-center space-x-1 shrink-0"
          title="Open Held Files in Shelf"
        >
          <span>📁 Shelf ({shelfItems.length})</span>
        </button>
      </div>

      {/* 2x2 Bento Grid for Concurrent Tasks (Equal 1fr rows prevent bottom overflow) */}
      <div className="grid grid-cols-2 grid-rows-2 gap-1.5 flex-1 min-h-0">
        {/* Slot 1: Media Controller Card */}
        <div
          onClick={(e) => {
            e.stopPropagation();
            if (isMediaActive) onOpenMediaHome();
          }}
          className="rounded-xl orbit-card-3d px-2.5 py-1.5 flex flex-col justify-between min-w-0 min-h-0 cursor-pointer hover:border-white/25 transition-colors overflow-hidden"
        >
          <div className="flex items-center justify-between shrink-0">
            <span className="text-[9.5px] font-bold uppercase tracking-wider text-amber-300 leading-none truncate pr-1">
              {mediaSession?.playback_status === "playing" ? "♫ Playing" : "♫ Media"}
            </span>
            {isMediaActive && (
              <div
                className="flex items-center space-x-1 shrink-0"
                onClick={(e) => e.stopPropagation()}
              >
                <button
                  type="button"
                  onClick={() => onMediaCommand("PlayPause")}
                  className="w-4.5 h-4.5 rounded-full bg-white/15 hover:bg-white/25 flex items-center justify-center text-[9px] text-white cursor-pointer transition-colors"
                  title="Play / Pause"
                >
                  {mediaSession?.playback_status === "playing" ? "⏸" : "▶"}
                </button>
                <button
                  type="button"
                  onClick={() => onMediaCommand("Next")}
                  className="w-4.5 h-4.5 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-[9px] text-white/85 cursor-pointer transition-colors"
                  title="Next Track"
                >
                  ⏭
                </button>
              </div>
            )}
          </div>
          <div className="min-w-0">
            <div
              className="text-[11px] font-bold truncate leading-tight"
              style={{ color: "var(--island-text)" }}
            >
              {isMediaActive && mediaSession
                ? mediaSession.track.title || "Unknown Track"
                : "No Active Media"}
            </div>
            <div
              className="text-[9.5px] font-medium truncate leading-tight"
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
          className="rounded-xl orbit-card-3d px-2.5 py-1.5 flex flex-col justify-between min-w-0 min-h-0 cursor-pointer hover:border-white/25 transition-colors overflow-hidden"
        >
          <div className="flex items-center justify-between shrink-0">
            <span
              className="text-[9.5px] font-bold uppercase tracking-wider leading-none truncate pr-1"
              style={{ color: isCallOrSensorActive ? "#34d399" : "var(--island-muted)" }}
            >
              {isCallOrSensorActive ? "● Live Call / Mic" : "🛡️ Privacy Guard"}
            </span>
            {callDurationSecs !== null && callDurationSecs > 0 && (
              <span className="text-[10px] font-mono font-bold text-emerald-300 tabular-nums leading-none shrink-0">
                {formatSeconds(callDurationSecs)}
              </span>
            )}
          </div>
          <div className="min-w-0">
            <div
              className="text-[11px] font-bold truncate leading-tight"
              style={{ color: "var(--island-text)" }}
            >
              {isCallOrSensorActive ? activeSensorApp : "Cam & Mic Idle"}
            </div>
            <div
              className="text-[9.5px] font-medium truncate leading-tight"
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
        <div className="rounded-xl orbit-card-3d px-2.5 py-1.5 flex flex-col justify-between min-w-0 min-h-0 overflow-hidden">
          <div className="flex items-center justify-between shrink-0">
            <span className="text-[9.5px] font-bold uppercase tracking-wider text-sky-300 leading-none truncate pr-1">
              ⏱ {timerLabel} Timer
            </span>
            {timerRemainingSecs > 0 && (
              <div className="flex items-center space-x-1 shrink-0">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    if (isTimerPaused) onResumeTimer();
                    else onPauseTimer();
                  }}
                  className="w-4.5 h-4.5 rounded bg-white/15 hover:bg-white/25 flex items-center justify-center text-[9px] font-bold text-white cursor-pointer transition-colors"
                  title={isTimerPaused ? "Resume Timer" : "Pause Timer"}
                >
                  {isTimerPaused ? "▶" : "⏸"}
                </button>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onStopTimer();
                  }}
                  className="w-4.5 h-4.5 rounded bg-rose-500/20 hover:bg-rose-500/35 flex items-center justify-center text-[9px] font-bold text-rose-300 cursor-pointer transition-colors"
                  title="Stop Timer"
                >
                  ✕
                </button>
              </div>
            )}
          </div>
          {timerRemainingSecs > 0 ? (
            <div className="flex items-baseline space-x-1.5 min-w-0">
              <span className="text-[15px] font-extrabold text-amber-300 tabular-nums leading-none">
                {formatSeconds(timerRemainingSecs)}
              </span>
              <span className="text-[9px] font-medium text-white/45 leading-none">
                {isTimerPaused ? "paused" : "running"}
              </span>
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
                  className="px-2 py-0.5 rounded orbit-pill-badge-3d hover:bg-white/20 text-[9.5px] font-bold cursor-pointer transition-colors leading-none"
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
          className="rounded-xl orbit-card-3d px-2.5 py-1.5 flex flex-col justify-between min-w-0 min-h-0 cursor-pointer hover:border-white/25 transition-colors overflow-hidden"
        >
          <div className="flex items-center justify-between shrink-0">
            <span
              className="text-[9.5px] font-bold uppercase tracking-wider truncate leading-none pr-1"
              style={{ color: eventCategoryColor }}
            >
              📅 {todayNextEvent ? (todayNextEvent.category || "Event").toUpperCase() : "Schedule"}
            </span>
            <span className="text-[9px] font-semibold text-white/50 leading-none shrink-0">Calendar →</span>
          </div>
          <div className="min-w-0">
            <div
              className="text-[11px] font-bold truncate leading-tight"
              style={{ color: "var(--island-text)" }}
            >
              {todayNextEvent ? todayNextEvent.title : "No Events Today"}
            </div>
            <div
              className="text-[9.5px] font-medium truncate leading-tight"
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
