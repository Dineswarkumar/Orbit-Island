import React from "react";
import { AppConfig } from "../../core/types/config";
import { ThemeMeta } from "../../core/theme";
import { MediaSessionSnapshot, AudioSpectrumState } from "../../core/types/media";
import { NotificationItem, NotificationSnapshot, PrivacyState } from "../../core/types/notifications";
import { NotificationBrandIcon } from "./NotificationBrandIcon";
import { FileShelfDrawer } from "./FileShelfDrawer";
import { ShelfItem } from "../../core/types/shelf";
import { ReminderItem } from "../../core/types/reminders";
import { ipc } from "../../core/ipc/client";

interface HomeSectionProps {
  config: AppConfig;
  activeThemeMeta: ThemeMeta;
  showHomeNotifications: boolean;
  setShowHomeNotifications: (show: boolean) => void;
  setSelectedNotifId: (id: number | null) => void;
  displayNotif: NotificationItem | null;
  notificationSnapshot: NotificationSnapshot | null;
  onDismissNotification: (id: number) => void;
  onClearAllNotifications: () => void;
  onViewNotification: (notif?: NotificationItem | null) => void;
  formatRelativeTime: (timestampMs: number) => string;
  isMediaActive: boolean;
  mediaSession: MediaSessionSnapshot | null;
  isMediaCommandPending: boolean;
  onTogglePin: () => void;
  onMediaCommand: (cmd: string) => void;
  isDragging: boolean;
  dragPos: number;
  pendingSeekPos: number | null;
  lastServerPos: number;
  lastServerTs: number;
  onSeekPointerDown: (e: React.PointerEvent<HTMLInputElement>) => void;
  onSeekChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onSeekPointerUp: (e: React.PointerEvent<HTMLInputElement>) => void;
  onSeekPointerCancel: (e: React.PointerEvent<HTMLInputElement>) => void;
  onSeekKeyUp: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  formatSeconds: (secs: number) => string;
  timeDisplay: { timeStr: string; ampm: string };
  formattedDate: string;
  expandedPrivacySentence: string | null;
  privacyState: PrivacyState | null;
  timerRemainingSecs?: number;
  timerTotalSecs?: number;
  onStartTimer?: (durationSecs: number, label?: string) => void;
  onStopTimer?: () => void;
  isTimerPaused?: boolean;
  timerLabel?: string;
  onPauseTimer?: () => void;
  onResumeTimer?: () => void;
  onResetTimer?: () => void;
  onChangeTimerLabel?: (label: string) => void;
  shelfItems?: ShelfItem[];
  showFileShelf?: boolean;
  setShowFileShelf?: (show: boolean) => void;
  onRefreshShelf?: () => void;
  callDurationSecs?: number | null;
  audioSpectrum?: AudioSpectrumState | null;
  todayNextEvent?: ReminderItem | null;
  onOpenCalendarSection?: () => void;
  showTimerOverlay?: boolean;
  setShowTimerOverlay?: (show: boolean) => void;
  homeSubView?: "auto" | "media" | "call" | "clock";
  setHomeSubView?: (view: "auto" | "media" | "call" | "clock") => void;
}

export const HomeSection: React.FC<HomeSectionProps> = ({
  config,
  activeThemeMeta,
  showHomeNotifications,
  setShowHomeNotifications,
  setSelectedNotifId,
  displayNotif,
  notificationSnapshot,
  onDismissNotification,
  onClearAllNotifications,
  onViewNotification,
  formatRelativeTime,
  isMediaActive,
  mediaSession,
  isMediaCommandPending,
  onTogglePin,
  onMediaCommand,
  isDragging,
  dragPos,
  pendingSeekPos,
  lastServerPos,
  lastServerTs,
  onSeekPointerDown,
  onSeekChange,
  onSeekPointerUp,
  onSeekPointerCancel,
  onSeekKeyUp,
  formatSeconds,
  timeDisplay,
  formattedDate,
  expandedPrivacySentence,
  privacyState,
  timerRemainingSecs = 0,
  timerTotalSecs = 0,
  onStartTimer,
  onStopTimer,
  isTimerPaused = false,
  timerLabel = "Focus",
  onPauseTimer,
  onResumeTimer,
  onResetTimer,
  onChangeTimerLabel,
  shelfItems = [],
  showFileShelf = false,
  setShowFileShelf,
  onRefreshShelf,
  callDurationSecs = null,
  audioSpectrum = null,
  todayNextEvent = null,
  onOpenCalendarSection,
  showTimerOverlay: showTimerOverlayProp,
  setShowTimerOverlay: setShowTimerOverlayProp,
  homeSubView: homeSubViewProp,
  setHomeSubView: setHomeSubViewProp,
}) => {
  const [localShowTimerOverlay, setLocalShowTimerOverlay] = React.useState(false);
  const [localHomeSubView, setLocalHomeSubView] = React.useState<"auto" | "media" | "call" | "clock">("auto");
  const showTimerOverlay = showTimerOverlayProp !== undefined ? showTimerOverlayProp : localShowTimerOverlay;
  const setShowTimerOverlay = setShowTimerOverlayProp ?? setLocalShowTimerOverlay;
  const homeSubView = homeSubViewProp !== undefined ? homeSubViewProp : localHomeSubView;
  const setHomeSubView = setHomeSubViewProp ?? setLocalHomeSubView;
  const isCallOrPrivacyActive = Boolean(privacyState?.cameraActive || privacyState?.microphoneActive);

  // Auto-reset "call" view when call/privacy turns off
  React.useEffect(() => {
    if (!isCallOrPrivacyActive && homeSubView === "call") {
      setHomeSubView("auto");
    }
  }, [isCallOrPrivacyActive, homeSubView]);

  if (showFileShelf) {
    return (
      <FileShelfDrawer
        items={shelfItems}
        onBack={() => setShowFileShelf?.(false)}
        onRefresh={() => onRefreshShelf?.()}
      />
    );
  }

  if (homeSubView === "call" && isCallOrPrivacyActive) {
    const activeApp = privacyState?.microphoneApp || privacyState?.cameraApp || "Active Call";
    return (
      <div className="flex flex-col justify-between h-full select-none px-1 py-0.5">
        {/* Call Deck Header */}
        <div className="flex items-center justify-between w-full">
          <div className="flex items-center space-x-2">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setHomeSubView("auto");
              }}
              className="px-2 py-0.5 rounded-md text-[10px] font-semibold orbit-pill-badge-3d text-white/85 hover:text-white transition-colors cursor-pointer flex items-center space-x-1"
            >
              <span>←</span>
              <span>Back</span>
            </button>
            <span className="text-[11px] font-bold tracking-tight text-white/90">
              Live Call & Privacy
            </span>
          </div>
          {callDurationSecs !== null && (
            <div className="px-2.5 py-0.5 rounded-full orbit-pill-badge-3d flex items-center space-x-1.5 text-emerald-300 border border-emerald-500/30 bg-emerald-500/10">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
              <span className="text-[10px] font-mono font-bold">
                {Math.floor(callDurationSecs / 60)}:{String(callDurationSecs % 60).padStart(2, "0")}
              </span>
            </div>
          )}
        </div>

        {/* Live Call Center Card */}
        <div
          onClick={() => {
            const app = privacyState?.microphoneApp || privacyState?.cameraApp || "";
            if (app) {
              ipc.activateAppSource("", app, null).catch(() => {});
            }
          }}
          className="my-auto py-2.5 px-3.5 rounded-xl orbit-card-3d flex items-center space-x-3 cursor-pointer hover:border-emerald-500/40 transition-colors"
          style={{ borderLeft: "3px solid #22c55e" }}
          title={`Click to open ${activeApp}`}
        >
          <div className="w-9 h-9 rounded-full bg-emerald-500/20 border border-emerald-400/40 flex items-center justify-center shrink-0">
            {privacyState?.cameraActive ? (
              <svg className="w-4 h-4 text-emerald-400" viewBox="0 0 24 24" fill="currentColor">
                <path d="M17 10.5V7c0-.55-.45-1-1-1H4c-.55 0-1 .45-1 1v10c0 .55.45 1 1 1h12c.55 0 1-.45 1-1v-3.5l4 4v-11l-4 4z" />
              </svg>
            ) : (
              <svg className="w-4 h-4 text-amber-400" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z" />
                <path d="M17 11c0 2.76-2.24 5-5 5s-5-2.24-5-5H5c0 3.53 2.61 6.43 6 6.92V21h2v-3.08c3.39-.49 6-3.39 6-6.92h-2z" />
              </svg>
            )}
          </div>
          <div className="flex flex-col min-w-0 flex-1">
            <div className="flex items-center space-x-2">
              <span className="text-[12px] font-bold text-white truncate">
                {activeApp}
              </span>
              <div className="flex items-center space-x-1 shrink-0">
                {privacyState?.cameraActive && (
                  <span className="px-1.5 py-0.2 rounded-full text-[8.5px] font-bold bg-emerald-500/25 text-emerald-300 border border-emerald-500/35">
                    CAM
                  </span>
                )}
                {privacyState?.microphoneActive && (
                  <span className="px-1.5 py-0.2 rounded-full text-[8.5px] font-bold bg-amber-500/25 text-amber-300 border border-amber-500/35">
                    MIC
                  </span>
                )}
              </div>
            </div>
            <p className="text-[10px] text-white/60 truncate mt-0.5">
              {expandedPrivacySentence || `${activeApp} is actively accessing your devices`}
            </p>
          </div>
        </div>

        {/* Call Deck Actions */}
        <div className="flex items-center justify-between pt-1">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              const app = privacyState?.microphoneApp || privacyState?.cameraApp || "";
              if (app) {
                ipc.activateAppSource("", app, null).catch(() => {});
              }
            }}
            className="px-3 py-1 rounded-lg text-[10.5px] font-bold bg-emerald-500/20 text-emerald-200 border border-emerald-400/35 hover:bg-emerald-500/35 transition-colors cursor-pointer"
          >
            Open Active Call / App
          </button>
          <div className="flex items-center space-x-1.5">
            {isMediaActive && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setHomeSubView("media");
                }}
                className="px-2.5 py-1 rounded-lg text-[10px] font-semibold orbit-pill-badge-3d text-white/80 hover:text-white transition-colors cursor-pointer"
              >
                🎵 Switch to Media
              </button>
            )}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setHomeSubView("auto");
              }}
              className="px-2.5 py-1 rounded-lg text-[10px] font-medium orbit-pill-badge-3d text-white/60 hover:text-white transition-colors cursor-pointer"
            >
              Back to Home
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (showHomeNotifications && displayNotif && config.notificationsEnabled) {
    const isIncomingCall = /incoming call|voice call|video call|is calling|meeting started/i.test(
      `${displayNotif.title} ${displayNotif.body}`
    );
    return (
      <div className="flex flex-col justify-between h-full select-none px-1 py-0.5">
        {/* 3D Notification Inspector Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2 min-w-0">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setSelectedNotifId(null);
                setShowHomeNotifications(false);
              }}
              className="px-2 py-0.5 rounded-md text-[10px] font-semibold orbit-pill-badge-3d text-white/85 hover:text-white transition-colors cursor-pointer flex items-center space-x-1 mr-1 shrink-0"
              title="Back to Home"
            >
              <span>←</span>
              <span>Back</span>
            </button>
            <NotificationBrandIcon
              notification={displayNotif}
              size="inspector"
              fallbackColor={activeThemeMeta.accent}
            />
            <span
              className="text-[11px] font-bold truncate tracking-tight"
              style={{ color: "var(--island-text)" }}
            >
              {displayNotif.appName}
            </span>
            {displayNotif.count > 1 && (
              <span className="text-[8.5px] px-1.5 py-0.2 rounded-full bg-white/10 text-white/80 font-bold tabular-nums">
                +{displayNotif.count}
              </span>
            )}
            {displayNotif.isCritical && (
              <span className="text-[8px] px-1 py-0.2 rounded bg-rose-500/30 text-rose-300 font-bold uppercase tracking-wider">
                Alert
              </span>
            )}
          </div>
          <div className="flex items-center space-x-1.5">
            {isCallOrPrivacyActive && (
              <div className="flex items-center space-x-1 px-1.5 py-0.5 rounded-full bg-black/40 border border-white/10 mr-1 shrink-0">
                {privacyState?.cameraActive && (
                  <span className="w-2 h-2 rounded-full bg-emerald-400 shadow-[0_0_5px_#22c55e] animate-pulse" title="Camera in use" />
                )}
                {privacyState?.microphoneActive && (
                  <span className="w-2 h-2 rounded-full bg-amber-400 shadow-[0_0_5px_#facc15] animate-pulse" title="Microphone in use" />
                )}
              </div>
            )}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onDismissNotification(displayNotif.id);
              }}
              className="p-1 rounded-md orbit-pill-badge-3d hover:brightness-110 transition-colors cursor-pointer"
              style={{ color: "var(--island-muted)" }}
              title="Dismiss this notification from ORBIT"
            >
              <svg
                className="w-3.5 h-3.5"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
              >
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        </div>

        {/* 3D Notification Body Card */}
        <div
          onClick={() => {
            ipc.activateAppSource(displayNotif.appId, displayNotif.appName, displayNotif.title);
            setShowHomeNotifications(false);
          }}
          className="my-auto py-1.5 px-2.5 rounded-xl orbit-card-3d space-y-0.5 overflow-hidden select-text cursor-pointer hover:border-white/30 transition-colors"
          style={{ borderLeft: `2px solid ${activeThemeMeta.accent}` }}
          title={`Click to open ${displayNotif.appName}`}
        >
          <h4
            className="font-bold text-[12px] leading-snug truncate"
            style={{ color: "var(--island-text)" }}
          >
            {config.privacyMode ? displayNotif.appName : displayNotif.title}
          </h4>
          <p
            className="text-[10.5px] leading-relaxed line-clamp-2 select-text"
            style={{ color: "var(--island-muted)" }}
          >
            {config.privacyMode
              ? "New notification (Content hidden by Privacy Mode)"
              : displayNotif.body || displayNotif.title}
          </p>
        </div>

        {/* Actions and Queue Footer */}
        <div className="flex items-center justify-between pt-1 text-[9.5px]">
          <div
            className="flex items-center space-x-1.5 font-semibold truncate"
            style={{ color: "var(--island-muted)" }}
          >
            <span>{formatRelativeTime(displayNotif.timestampMs)}</span>
            {notificationSnapshot && notificationSnapshot.totalCount > 1 && (
              <span>· {notificationSnapshot.totalCount} in queue</span>
            )}
            {notificationSnapshot && notificationSnapshot.overflowCount > 0 && (
              <span>(+{notificationSnapshot.overflowCount} more)</span>
            )}
          </div>
          <div className="flex items-center space-x-1.5 shrink-0">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                ipc.activateAppSource(displayNotif.appId, displayNotif.appName, displayNotif.title);
                setShowHomeNotifications(false);
              }}
              className={`px-2 py-0.5 rounded-md orbit-pill-badge-3d font-semibold hover:brightness-110 transition-colors cursor-pointer text-[9.5px] flex items-center space-x-1 ${
                isIncomingCall
                  ? "bg-emerald-500/30 text-emerald-200 border border-emerald-400/40"
                  : ""
              }`}
              style={{ color: isIncomingCall ? undefined : "var(--island-text)" }}
              title={isIncomingCall ? "Answer / Open Call window" : `Open ${displayNotif.appName}`}
            >
              <span>{isIncomingCall ? "Answer / Open Call" : "Open"}</span>
              <svg className="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                <polyline points="15 3 21 3 21 9" />
                <line x1="10" y1="14" x2="21" y2="3" />
              </svg>
            </button>
            {notificationSnapshot && notificationSnapshot.totalCount > 1 && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onClearAllNotifications();
                }}
                className="px-2 py-0.5 rounded-md orbit-pill-badge-3d font-semibold hover:brightness-110 transition-colors cursor-pointer text-[9.5px]"
                style={{ color: "var(--island-muted)" }}
              >
                Clear all
              </button>
            )}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onDismissNotification(displayNotif.id);
              }}
              className="px-2 py-0.5 rounded-md font-semibold orbit-pill-badge-3d hover:brightness-110 transition-colors cursor-pointer text-[9.5px]"
              style={{ color: "var(--island-text)" }}
            >
              Dismiss
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (isMediaActive && mediaSession && !showTimerOverlay && homeSubView !== "clock") {
    const duration = mediaSession.timeline?.duration_secs ?? 0;
    const isTimelineReliable = Boolean(
      mediaSession.timeline && duration > 0 && duration <= 86400 * 2
    );

    let currentPos = mediaSession.timeline?.position_secs ?? 0;
    if (mediaSession.playback_status === "playing" && lastServerTs > 0) {
      const elapsed = (Date.now() - lastServerTs) / 1000;
      currentPos = Math.min(duration, Math.max(0, lastServerPos + elapsed));
    }

    const displayPos = isDragging
      ? dragPos
      : pendingSeekPos !== null
      ? pendingSeekPos
      : currentPos;
    const progressPercent =
      duration > 0 ? Math.min(100, Math.max(0, (displayPos / duration) * 100)) : 0;
    const canSeek = Boolean(mediaSession.controls.can_seek);

    const hasAudioSignal =
      mediaSession.playback_status === "playing" &&
      (audioSpectrum?.peak ?? 0) >= 0.005;

    return (
      <div className="orbit-media-surface flex flex-col h-full min-h-0 select-none px-1.5 py-0.5 relative overflow-hidden rounded-2xl gap-1">

        {/* Source & Pin Header */}
        <div className="flex items-center justify-between w-full min-h-[20px] shrink-0 overflow-hidden relative z-10">
          <div className="flex items-center space-x-1.5 min-w-0 flex-1 mr-2 overflow-hidden">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                ipc.activateAppSource(
                  mediaSession.source_id,
                  mediaSession.source_title,
                  mediaSession.track.title || mediaSession.source_title
                );
              }}
              className="px-3 py-0.5 rounded-full bg-indigo-950/55 border border-indigo-400/45 text-[10px] font-extrabold uppercase tracking-wider text-[#93c5fd] shadow-[inset_0_1px_1px_rgba(255,255,255,0.25)] truncate max-w-[110px] inline-block shrink-0 cursor-pointer hover:opacity-90 transition-opacity"
              title={`Click to open ${mediaSession.source_title}`}
            >
              {mediaSession.source_title}
            </button>
            {mediaSession.is_pinned && (
              <span className="text-[8.5px] font-semibold text-amber-300 bg-amber-500/20 border border-amber-400/30 px-1.5 py-0.5 rounded shrink-0">
                PINNED
              </span>
            )}
          </div>
          <div className="flex items-center space-x-1.5 shrink-0 ml-auto">
            {/* Header Privacy Indicator & Call Deck Switcher */}
            {isCallOrPrivacyActive && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setHomeSubView("call");
                }}
                className="px-2 py-0.5 rounded-full bg-emerald-950/55 border border-emerald-400/40 flex items-center space-x-1 text-emerald-300 hover:bg-emerald-900/60 transition-colors cursor-pointer shrink-0"
                title="Active Call / Sensor — Click to view Call Deck"
              >
                {privacyState?.cameraActive && (
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                )}
                {privacyState?.microphoneActive && (
                  <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse" />
                )}
                <span className="text-[9px] font-bold">
                  {callDurationSecs !== null
                    ? `${Math.floor(callDurationSecs / 60)}:${String(callDurationSecs % 60).padStart(2, "0")}`
                    : "Call"}
                </span>
              </button>
            )}
            {timerRemainingSecs > 0 && (
              <div
                className="px-2 py-0.5 rounded-full bg-indigo-950/55 border border-amber-400/40 shadow-[inset_0_1px_1px_rgba(255,255,255,0.22)] flex items-center space-x-1 text-amber-300 shrink-0"
              >
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    setShowTimerOverlay(true);
                  }}
                  className="flex items-center space-x-1 hover:text-white transition-colors cursor-pointer"
                  title="Click to open full Timer view"
                >
                  <span
                    className={`w-1.5 h-1.5 rounded-full ${
                      isTimerPaused ? "bg-amber-400" : "bg-amber-400 animate-pulse"
                    }`}
                  />
                  <span className="text-[9.5px] font-bold tabular-nums">
                    {formatSeconds(timerRemainingSecs)}
                  </span>
                </button>
                {/* Inline Pause / Resume */}
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    if (isTimerPaused) {
                      onResumeTimer?.();
                    } else {
                      onPauseTimer?.();
                    }
                  }}
                  className="p-0.5 rounded hover:bg-white/15 text-amber-200 hover:text-white transition-colors cursor-pointer"
                  title={isTimerPaused ? "Resume timer" : "Pause timer"}
                >
                  {isTimerPaused ? (
                    <svg className="w-2.5 h-2.5" viewBox="0 0 24 24" fill="currentColor">
                      <path d="M8 5v14l11-7z" />
                    </svg>
                  ) : (
                    <svg className="w-2.5 h-2.5" viewBox="0 0 24 24" fill="currentColor">
                      <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                    </svg>
                  )}
                </button>
                {/* Inline Reset */}
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onResetTimer?.();
                  }}
                  className="p-0.5 rounded hover:bg-white/15 text-amber-200/80 hover:text-white transition-colors cursor-pointer"
                  title="Reset timer"
                >
                  <svg
                    className="w-2.5 h-2.5"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                    <path d="M3 3v5h5" />
                  </svg>
                </button>
                {/* Inline Stop */}
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onStopTimer?.();
                  }}
                  className="px-0.5 rounded hover:bg-rose-500/25 text-white/60 hover:text-rose-300 text-[9px] leading-none transition-colors cursor-pointer"
                  title="Stop timer"
                >
                  ✕
                </button>
              </div>
            )}
            {/* Clock Pill */}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setHomeSubView("clock");
              }}
              className="px-3 py-0.5 rounded-full bg-indigo-950/55 border border-indigo-400/40 shadow-[inset_0_1px_1px_rgba(255,255,255,0.22)] flex items-baseline space-x-1 hover:opacity-90 transition-opacity cursor-pointer shrink-0"
              title="View Home Clock, Date & Widgets"
            >
              <span className="text-[11.5px] font-extrabold text-white orbit-keep-white tabular-nums tracking-tight">
                {timeDisplay.timeStr}
              </span>
              {timeDisplay.ampm && (
                <span className="text-[9.5px] font-extrabold text-[#818cf8]">
                  {timeDisplay.ampm}
                </span>
              )}
            </button>
            {notificationSnapshot &&
              notificationSnapshot.totalCount > 0 &&
              config.notificationsEnabled && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onViewNotification(notificationSnapshot?.notifications[0]);
                  }}
                  className="px-2.5 py-0.5 rounded-full bg-indigo-950/55 border border-indigo-400/40 shadow-[inset_0_1px_1px_rgba(255,255,255,0.22)] flex items-center space-x-1 hover:opacity-90 transition-opacity cursor-pointer shrink-0"
                  title={`${notificationSnapshot.totalCount} notification${
                    notificationSnapshot.totalCount > 1 ? "s" : ""
                  }`}
                >
                  <svg
                    className="w-3 h-3 text-[#60a5fa]"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path d="M12 22c1.1 0 2-.9 2-2h-4c0 1.1.9 2 2 2zm6-6v-5c0-3.07-1.63-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.64 5.36 6 7.92 6 11v5l-2 2v1h16v-1l-2-2zm-2 1H8v-6c0-2.48 1.51-4.5 4-4.5s4 2.02 4 4.5v6z" />
                  </svg>
                  <span className="text-[9.5px] font-extrabold text-white orbit-keep-white tabular-nums">
                    {notificationSnapshot.totalCount}
                  </span>
                </button>
              )}
            <button
              type="button"
              title={
                mediaSession.is_pinned
                  ? "Unpin media source"
                  : "Pin this media source"
              }
              disabled={isMediaCommandPending}
              onClick={(e) => {
                e.stopPropagation();
                onTogglePin();
              }}
              className={`p-1 rounded-md transition-colors cursor-pointer text-white/80 hover:text-white orbit-keep-white ${
                isMediaCommandPending ? "opacity-50 cursor-wait" : ""
              } ${
                mediaSession.is_pinned
                  ? "text-amber-300 bg-amber-400/20"
                  : ""
              }`}
            >
              <svg
                className="w-3.5 h-3.5"
                viewBox="0 0 24 24"
                fill={mediaSession.is_pinned ? "currentColor" : "none"}
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M16 4l4 4-2.5 2.5 2.5 7.5-6.5-6.5L6 19l1-6.5-6.5-6.5 7.5 2.5L16 4z"
                />
              </svg>
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                ipc.openSettingsWindow().catch(() => {});
              }}
              className="p-1 rounded-md text-white/80 hover:text-white orbit-keep-white transition-colors cursor-pointer"
              title="Open ORBIT Settings (or Right-Click Island)"
            >
              <svg
                className="w-3.5 h-3.5"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth={2}
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </button>
          </div>
        </div>

        {/* Middle Track Info Row (Album Cover Art + Title/Artist/Album) */}
        <div
          onClick={() => {
            ipc.activateAppSource(
              mediaSession.source_id,
              mediaSession.source_title,
              mediaSession.track.title || mediaSession.source_title
            );
          }}
          className="flex items-center gap-3 flex-1 min-h-0 cursor-pointer hover:opacity-95 transition-opacity relative z-10"
          title={`Click to open ${mediaSession.source_title}`}
        >
          <div className="w-[52px] h-[52px] rounded-xl overflow-hidden shrink-0 border border-white/25 shadow-[0_6px_16px_rgba(0,0,0,0.65)] flex items-center justify-center bg-black/40">
            {mediaSession.track.thumbnail_base64 ? (
              <img
                src={mediaSession.track.thumbnail_base64}
                alt="Album Cover"
                className="w-full h-full object-cover"
              />
            ) : (
              <svg
                className="w-6 h-6 text-[#93c5fd]"
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
              </svg>
            )}
          </div>
          <div className="flex flex-col min-w-0 flex-1 overflow-hidden justify-center">
            <span
              className="text-[14.5px] font-extrabold text-white orbit-keep-white leading-tight truncate tracking-tight"
              title={mediaSession.track.title}
            >
              {mediaSession.track.title || "Unknown Track"}
            </span>
            <span
              className="text-[12px] font-bold text-[#93c5fd] leading-tight truncate mt-0.5"
              title={mediaSession.track.artist}
            >
              {mediaSession.track.artist || "Unknown Artist"}
            </span>
            <span
              className="text-[9.5px] font-bold uppercase tracking-wider text-slate-300/75 leading-tight truncate mt-0.5"
              title={mediaSession.track.album || mediaSession.source_title}
            >
              {mediaSession.track.album || mediaSession.source_title}
            </span>
          </div>
        </div>

        {/* Progress Scrubber Row */}
        <div className="flex items-center space-x-2 w-full relative z-10">
          <span className="text-[10px] font-bold tabular-nums text-white orbit-keep-white w-7 text-right shrink-0">
            {formatSeconds(displayPos)}
          </span>
          <div className="relative flex-1 flex items-center h-4">
            {/* Visual Track Bar */}
            <div className="w-full h-[3.5px] rounded-full bg-white/25 relative overflow-visible">
              {/* Filled Blue Bar */}
              <div
                className="h-full rounded-full bg-gradient-to-r from-blue-600 via-blue-500 to-sky-400 shadow-[0_0_10px_rgba(59,130,246,0.85)]"
                style={{ width: `${progressPercent}%` }}
              />
              {/* Sleek Elongated Blue Liquid-Glass Wave around Thumb */}
              <div
                style={{
                  position: "absolute",
                  left: `${progressPercent}%`,
                  top: "50%",
                  transform: "translate(-50%, -50%)",
                  width: "110px",
                  height: "14px",
                  borderRadius: "9999px",
                  background:
                    "radial-gradient(ellipse at center, rgba(59, 130, 246, 0.55) 0%, rgba(59, 130, 246, 0.20) 55%, transparent 85%)",
                  boxShadow:
                    "inset 0 1px 1px rgba(255, 255, 255, 0.6), inset 0 -1px 1px rgba(147, 197, 253, 0.4)",
                  pointerEvents: "none",
                }}
              />
              {/* Glowing White Circular Thumb with Translucent Blue Halo Ring */}
              <div
                style={{
                  position: "absolute",
                  left: `${progressPercent}%`,
                  top: "50%",
                  transform: "translate(-50%, -50%)",
                  width: "9px",
                  height: "9px",
                  borderRadius: "9999px",
                  background: "#ffffff",
                  boxShadow:
                    "0 0 0 4.5px rgba(59, 130, 246, 0.38), 0 0 12px rgba(96, 165, 250, 0.95)",
                  pointerEvents: "none",
                }}
              />
            </div>
            {/* Interactive Range Input Overlay */}
            <input
              type="range"
              min={0}
              max={duration || 100}
              step={1}
              disabled={!canSeek || !isTimelineReliable}
              value={displayPos}
              onPointerDown={onSeekPointerDown}
              onChange={onSeekChange}
              onPointerUp={onSeekPointerUp}
              onPointerCancel={onSeekPointerCancel}
              onKeyUp={onSeekKeyUp}
              aria-label="Track progress"
              aria-valuemin={0}
              aria-valuemax={duration}
              aria-valuenow={Math.round(displayPos)}
              aria-valuetext={`${formatSeconds(displayPos)} of ${formatSeconds(
                duration
              )}`}
              className="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10 disabled:cursor-default"
            />
          </div>
          <span className="text-[10px] font-bold tabular-nums text-white orbit-keep-white w-7 text-left shrink-0">
            {formatSeconds(duration)}
          </span>
        </div>

        {/* Bottom Row — Full-Width Music Frequency Waves + Increased Gap Transport Controls */}
        <div className="relative flex items-center justify-center h-[38px] w-full shrink-0">
          {/* Full-Width Music Frequency Waves */}
          <div className="pointer-events-none absolute inset-x-0 bottom-0 h-[34px] flex items-end justify-between px-0.5 z-0">
            {Array.from({ length: 40 }).map((_, idx) => {
              const isPlaying = mediaSession.playback_status === "playing";
              const isCenter = idx >= 12 && idx <= 27;

              // Natural 40-bar alternating peak-and-valley musical envelope
              const waveEnvelope = [
                0.42, 0.88, 0.54, 0.96, 0.48, 0.78, 0.38, 0.84, 0.62, 0.92, 0.44, 0.72,
                0.36, 0.52, 0.30, 0.45, 0.26, 0.38, 0.24, 0.34, 0.26, 0.40, 0.28, 0.46,
                0.32, 0.54, 0.38, 0.68, 0.46, 0.86, 0.52, 0.94, 0.44, 0.76, 0.58, 0.88,
                0.42, 0.74, 0.50, 0.36
              ];

              let height = 0.12;
              if (isPlaying) {
                if (hasAudioSignal && audioSpectrum?.waveBars && audioSpectrum.waveBars.length > 0) {
                  const sourceIdx = Math.floor((idx / 39) * (audioSpectrum.waveBars.length - 1));
                  const val = audioSpectrum.waveBars[sourceIdx] ?? 0;
                  const baseFactor = 0.35 + val * 0.75;
                  height = Math.max(0.20, Math.min(0.96, baseFactor * waveEnvelope[idx]));
                } else {
                  height = waveEnvelope[idx];
                }
              }

              if (isCenter) {
                height = height * 0.28;
              }

              const opacity = isCenter ? 0.22 : isPlaying ? 0.95 : 0.35;

              return (
                <div
                  key={idx}
                  className="w-[2.5px] h-full rounded-full origin-bottom transition-transform duration-100"
                  style={{
                    transform: `scaleY(${height.toFixed(2)})`,
                    opacity,
                    background:
                      "linear-gradient(180deg, #93c5fd 0%, #3b82f6 55%, rgba(30, 58, 138, 0.2) 100%)",
                    boxShadow: "0 0 6px rgba(59, 130, 246, 0.65)",
                  }}
                />
              );
            })}
          </div>

          {/* Center Transport Buttons with Increased Gap (gap-6 = 24px) */}
          <div className="relative z-10 flex items-center justify-center gap-6">
            <button
              type="button"
              title="Previous"
              aria-label="Previous track"
              disabled={
                isMediaCommandPending || !mediaSession.controls.can_skip_previous
              }
              onClick={(e) => {
                e.stopPropagation();
                onMediaCommand("previous");
              }}
              className={`w-8 h-8 rounded-full bg-indigo-950/65 hover:bg-indigo-900/80 border border-indigo-300/40 shadow-[0_2px_8px_rgba(0,0,0,0.45),inset_0_1px_1px_rgba(255,255,255,0.28)] flex items-center justify-center text-white orbit-keep-white transition-transform active:scale-90 cursor-pointer disabled:opacity-35 ${
                isMediaCommandPending ? "pointer-events-none opacity-40" : ""
              }`}
            >
              <svg
                className="w-4 h-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.4"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <polyline points="11 17 6 12 11 7" />
                <polyline points="18 17 13 12 18 7" />
              </svg>
            </button>

            <button
              type="button"
              title={
                mediaSession.playback_status === "playing" ? "Pause" : "Play"
              }
              aria-label={
                mediaSession.playback_status === "playing"
                  ? "Pause playback"
                  : "Start playback"
              }
              disabled={
                isMediaCommandPending ||
                (mediaSession.playback_status === "playing"
                  ? !mediaSession.controls.can_pause
                  : !mediaSession.controls.can_play)
              }
              onClick={(e) => {
                e.stopPropagation();
                onMediaCommand("toggle");
              }}
              className={`w-10 h-10 rounded-full bg-gradient-to-b from-blue-500/55 to-indigo-600/70 hover:from-blue-400/65 hover:to-indigo-500/80 border-[1.5px] border-blue-300/90 shadow-[0_0_18px_rgba(59,130,246,0.85),inset_0_1.5px_2px_rgba(255,255,255,0.65)] flex items-center justify-center text-white orbit-keep-white transition-all active:scale-90 cursor-pointer disabled:opacity-35 ${
                isMediaCommandPending
                  ? "opacity-60 cursor-wait pointer-events-none"
                  : ""
              }`}
            >
              {mediaSession.playback_status === "playing" ? (
                <svg className="w-5 h-5 fill-current" viewBox="0 0 24 24">
                  <rect x="6" y="5" width="4" height="14" rx="1.5" />
                  <rect x="14" y="5" width="4" height="14" rx="1.5" />
                </svg>
              ) : (
                <svg className="w-5 h-5 fill-current translate-x-0.5" viewBox="0 0 24 24">
                  <polygon points="6 4 20 12 6 20 6 4" />
                </svg>
              )}
            </button>

            <button
              type="button"
              title="Next"
              aria-label="Next track"
              disabled={
                isMediaCommandPending || !mediaSession.controls.can_skip_next
              }
              onClick={(e) => {
                e.stopPropagation();
                onMediaCommand("next");
              }}
              className={`w-8 h-8 rounded-full bg-indigo-950/65 hover:bg-indigo-900/80 border border-indigo-300/40 shadow-[0_2px_8px_rgba(0,0,0,0.45),inset_0_1px_1px_rgba(255,255,255,0.28)] flex items-center justify-center text-white orbit-keep-white transition-transform active:scale-90 cursor-pointer disabled:opacity-35 ${
                isMediaCommandPending ? "pointer-events-none opacity-40" : ""
              }`}
            >
              <svg
                className="w-4 h-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.4"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <polyline points="13 17 18 12 13 7" />
                <polyline points="6 17 11 12 6 7" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    );
  }

  /* Expanded Idle Home: restrained clock, date, and privacy controls. */
  return (
    <div className="flex flex-col justify-between h-full text-center select-none relative pt-0.5 pb-1">
      {/* Top Header Row with Left, Center (Privacy Pill / Indicator), and Right Controls */}
      <div className="relative z-10 flex items-center justify-between w-full h-6 shrink-0 px-0.5">
        {/* Top-Left File Shelf Toggle Pill & Optional Back-to-Music Pill */}
        <div className="flex items-center space-x-1.5 min-w-0">
          {isMediaActive && mediaSession && (showTimerOverlay || homeSubView === "clock") && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setShowTimerOverlay(false);
                setHomeSubView("auto");
              }}
              className="px-2.5 py-0.5 rounded-full orbit-pill-badge-3d flex items-center space-x-1.5 text-amber-300 border border-amber-400/40 hover:bg-white/10 hover:text-white transition-all cursor-pointer shadow-sm shrink-0"
              title="Return to Media Player"
            >
              <span className="text-[10.5px]">←</span>
              <span className="text-[11px] font-bold">Music</span>
              <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse" />
            </button>
          )}
          {isCallOrPrivacyActive && (
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setHomeSubView("call");
              }}
              className="px-2.5 py-0.5 rounded-full orbit-pill-badge-3d flex items-center space-x-1 text-emerald-300 border border-emerald-400/35 bg-emerald-500/15 hover:bg-emerald-500/25 transition-colors cursor-pointer shrink-0"
              title="Active Call / Sensor — Click to view Call Deck"
            >
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
              <span className="text-[11px] font-bold">Call</span>
            </button>
          )}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setShowFileShelf?.(true);
            }}
            className="px-2.5 py-0.5 rounded-full orbit-top-control-pill orbit-pill-badge-3d flex items-center space-x-1 hover:brightness-110 transition-colors cursor-pointer shrink-0"
            style={{ color: activeThemeMeta.accent || "var(--theme-accent)" }}
            title={`Open File Shelf (${shelfItems.length} item${
              shelfItems.length === 1 ? "" : "s"
            })`}
          >
            <svg
              className="w-3.5 h-3.5"
              style={{ color: activeThemeMeta.accent || "var(--theme-accent)" }}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
            </svg>
            <span
              className="text-[11px] font-bold tabular-nums"
              style={{ color: "var(--island-text)" }}
            >
              Shelf {shelfItems.length > 0 ? `(${shelfItems.length})` : ""}
            </span>
          </button>
        </div>

        {/* Top-Right Notifications & Visible Settings Button */}
        <div className="flex items-center space-x-1.5 shrink-0">
          {notificationSnapshot &&
            notificationSnapshot.totalCount > 0 &&
            config.notificationsEnabled && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  if (
                    notificationSnapshot?.notifications &&
                    notificationSnapshot.notifications.length > 0
                  ) {
                    onViewNotification(notificationSnapshot.notifications[0]);
                  } else {
                    setShowHomeNotifications(true);
                  }
                }}
                className="px-2.5 py-0.5 rounded-full orbit-top-control-pill orbit-pill-badge-3d flex items-center space-x-1 hover:brightness-110 transition-colors cursor-pointer"
                style={{ color: activeThemeMeta.accent || "var(--theme-accent)" }}
                title={`${notificationSnapshot.totalCount} unread notification${
                  notificationSnapshot.totalCount > 1 ? "s" : ""
                }`}
              >
                <svg
                  className="w-3.5 h-3.5"
                  style={{ color: activeThemeMeta.accent || "var(--theme-accent)" }}
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                >
                  <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
                  <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
                </svg>
                <span
                  className="text-[11px] font-bold tabular-nums"
                  style={{ color: "var(--island-text)" }}
                >
                  {notificationSnapshot.totalCount}
                </span>
              </button>
            )}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              ipc.openSettingsWindow().catch(() => {});
            }}
            className="px-2 py-0.5 rounded-full orbit-top-control-pill orbit-pill-badge-3d flex items-center space-x-1 hover:brightness-110 transition-colors cursor-pointer shrink-0"
            style={{ color: activeThemeMeta.accent || "var(--theme-accent)" }}
            title="Open ORBIT Settings (or Right-Click Island)"
          >
            <svg
              className="w-3.5 h-3.5"
              style={{ color: activeThemeMeta.accent || "var(--theme-accent)" }}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
        </div>
      </div>

      {/* Main Center Stage: 3D Floating Clock Typography & Date Subtitle */}
      <div className="flex-1 flex flex-col items-center justify-center my-0.5 relative">
        {/* 3D Floating Clock Typography */}
        <div
          className="relative z-10 text-[50px] font-bold tracking-tight flex items-baseline justify-center leading-none select-none tabular-nums orbit-clock-3d"
          style={{
            color: "var(--island-text)",
            transform: "translateZ(14px)",
          }}
        >
          <span>{timeDisplay.timeStr}</span>
          {timeDisplay.ampm && (
            <span
              className="text-[15px] font-semibold uppercase ml-2 align-baseline"
              style={{ color: "var(--island-muted)" }}
            >
              {timeDisplay.ampm}
            </span>
          )}
        </div>

        {/* Date Subtitle */}
        <div
          className="relative z-10 text-[13.5px] font-bold tracking-normal mt-0.5"
          style={{ color: "var(--island-text)", opacity: 0.92 }}
        >
          {formattedDate}
        </div>

        {/* Repositioned Today's Schedule Event Pill with ample room */}
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
      </div>

      {/* Bottom Controls Area: Detailed Privacy Pill (when active) AND Focus Timers */}
      <div className="relative z-10 flex flex-col items-center justify-center space-y-1 shrink-0 pb-0.5">
        {/* 3D Frosted Privacy & Call Duration Pill */}
        {expandedPrivacySentence && (
          <div
            onClick={(e) => {
              e.stopPropagation();
              const app = privacyState?.microphoneApp || privacyState?.cameraApp || "";
              if (app) {
                ipc.activateAppSource("", app, null).catch(() => {});
              }
            }}
            className="px-3 py-0.5 rounded-full orbit-pill-badge-3d flex items-center space-x-1.5 text-[10.5px] font-semibold max-w-[300px] truncate cursor-pointer hover:bg-white/15 transition-colors"
            style={{ color: "var(--island-muted)" }}
            title="Click to jump to active call/app window"
          >
          {privacyState?.cameraActive && (
            <svg
              className="w-3.5 h-3.5 shrink-0"
              style={{ color: "#22c55e", filter: "drop-shadow(0 0 4px #22c55e80)" }}
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M4 7a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v1.6l3.4-2.1a.8.8 0 0 1 1.2.7v9.6a.8.8 0 0 1-1.2.7L15 15.4V17a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7z" />
            </svg>
          )}
          {privacyState?.microphoneActive && (
            <svg
              className="w-3.5 h-3.5 shrink-0"
              style={{ color: "#facc15", filter: "drop-shadow(0 0 4px #facc1580)" }}
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M12 3a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zm-6 8a1 1 0 1 1 2 0 4 4 0 0 0 8 0 1 1 0 1 1 2 0 6 6 0 0 1-5 5.9V20h2a1 1 0 1 1 0 2H9a1 1 0 1 1 0-2h2v-3.1A6 6 0 0 1 6 11z" />
            </svg>
          )}
          <span className="truncate">{expandedPrivacySentence}</span>
          {callDurationSecs !== null && callDurationSecs !== undefined && callDurationSecs > 0 && (
            <span className="text-[10.5px] font-bold text-emerald-400 tabular-nums ml-1 pl-1.5 border-l border-white/20 shrink-0">
              📞 {formatSeconds(callDurationSecs)}
            </span>
          )}
        </div>
      )}

        {/* Quick-Timer Bar / Active Countdown Bar */}
        <div className="flex items-center justify-center">
        {timerRemainingSecs > 0 ? (
          <div className="flex items-center space-x-1.5 px-3 py-0.5 rounded-full orbit-pill-badge-3d text-[11px] font-bold text-amber-300 shadow-[0_0_8px_rgba(245,158,11,0.25)] select-none">
            {/* Cycle label button */}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                const labels = ["Focus", "Break", "Workout"];
                const nextIdx = (labels.indexOf(timerLabel) + 1) % labels.length;
                onChangeTimerLabel?.(labels[nextIdx]);
              }}
              className="px-2 py-0.5 rounded text-[10px] uppercase font-extrabold tracking-wider bg-amber-500/20 text-amber-300 hover:bg-amber-500/30 transition-colors cursor-pointer"
              title="Click to cycle timer label"
            >
              {timerLabel}
            </button>

            <span className={`w-2 h-2 rounded-full ${isTimerPaused ? "bg-amber-400" : "bg-amber-400 animate-pulse"}`} />
            <span className="tabular-nums font-bold tracking-tight">
              {formatSeconds(timerRemainingSecs)}
            </span>

            {/* Pause / Resume */}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                if (isTimerPaused) {
                  onResumeTimer?.();
                } else {
                  onPauseTimer?.();
                }
              }}
              className="p-1 rounded hover:bg-white/10 text-white/80 hover:text-white transition-colors cursor-pointer"
              title={isTimerPaused ? "Resume timer" : "Pause timer"}
            >
              {isTimerPaused ? (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z" />
                </svg>
              ) : (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                </svg>
              )}
            </button>

            {/* Reset */}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onResetTimer?.();
              }}
              className="p-1 rounded hover:bg-white/10 text-white/80 hover:text-white transition-colors cursor-pointer"
              title="Reset countdown"
            >
              <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                <path d="M3 3v5h5" />
              </svg>
            </button>

            {/* Stop */}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onStopTimer?.();
              }}
              className="p-1 rounded hover:bg-rose-500/20 text-white/60 hover:text-rose-300 transition-colors cursor-pointer"
              title="Stop timer"
            >
              ✕
            </button>
          </div>
        ) : (
          <div className="flex items-center space-x-1.5 py-0.5">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                const labels = ["Focus", "Break", "Workout"];
                const nextIdx = (labels.indexOf(timerLabel) + 1) % labels.length;
                onChangeTimerLabel?.(labels[nextIdx]);
              }}
              className="px-2 py-0.5 rounded-full text-[11px] font-extrabold uppercase tracking-wider orbit-pill-badge-3d hover:bg-white/20 transition-colors cursor-pointer"
              style={{ color: "var(--island-text)" }}
              title="Click to cycle timer category"
            >
              {timerLabel}:
            </button>
            {[
              { label: "5m", secs: 300 },
              { label: "15m", secs: 900 },
              { label: "25m", secs: 1500 },
              { label: "45m", secs: 2700 },
            ].map(({ label, secs }) => {
              const isPresetActive = timerRemainingSecs > 0 && timerTotalSecs === secs;
              return (
                <button
                  key={label}
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onStartTimer?.(secs, timerLabel);
                  }}
                  className={`px-2.5 py-0.5 rounded-full text-[11px] font-bold orbit-pill-badge-3d transition-all cursor-pointer group ${
                    isPresetActive
                      ? "orbit-keep-white !text-white shadow-md ring-1 ring-white/60"
                      : "hover:!bg-[var(--theme-accent)] hover:!text-white hover:border-transparent hover:orbit-keep-white"
                  }`}
                  style={{
                    color: isPresetActive ? "#ffffff" : "var(--island-text)",
                    backgroundColor: isPresetActive
                      ? (activeThemeMeta.accent || "var(--theme-accent)")
                      : undefined,
                  }}
                  title={`Start ${label} ${timerLabel.toLowerCase()} timer`}
                >
                  <span className={isPresetActive ? "orbit-keep-white" : "group-hover:orbit-keep-white"}>
                    {label}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </div>
  </div>
  );
};
