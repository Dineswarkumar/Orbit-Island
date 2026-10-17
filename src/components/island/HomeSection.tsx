import React from "react";
import { AppConfig } from "../../core/types/config";
import { ThemeMeta } from "../../core/theme";
import { MediaSessionSnapshot } from "../../core/types/media";
import { NotificationItem, NotificationSnapshot, PrivacyState } from "../../core/types/notifications";
import { NotificationBrandIcon } from "./NotificationBrandIcon";
import { FileShelfDrawer } from "./FileShelfDrawer";
import { ShelfItem } from "../../core/types/shelf";

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
  onStartTimer?: (durationSecs: number) => void;
  onStopTimer?: () => void;
  shelfItems?: ShelfItem[];
  showFileShelf?: boolean;
  setShowFileShelf?: (show: boolean) => void;
  onRefreshShelf?: () => void;
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
  onStartTimer,
  onStopTimer,
  shelfItems = [],
  showFileShelf = false,
  setShowFileShelf,
  onRefreshShelf,
}) => {
  if (showFileShelf) {
    return (
      <FileShelfDrawer
        items={shelfItems}
        onBack={() => setShowFileShelf?.(false)}
        onRefresh={() => onRefreshShelf?.()}
      />
    );
  }

  if (showHomeNotifications && displayNotif && config.notificationsEnabled) {
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
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onDismissNotification(displayNotif.id);
            }}
            className="p-1 rounded-md text-white/50 hover:text-white hover:bg-white/10 transition-colors cursor-pointer"
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

        {/* 3D Notification Body Card */}
        <div
          className="my-auto py-1.5 px-2.5 rounded-xl orbit-card-3d space-y-0.5 overflow-hidden select-text"
          style={{ borderLeft: `2px solid ${activeThemeMeta.accent}` }}
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
          <div className="flex items-center space-x-1.5 text-white/45 truncate">
            <span>{formatRelativeTime(displayNotif.timestampMs)}</span>
            {notificationSnapshot && notificationSnapshot.totalCount > 1 && (
              <span>· {notificationSnapshot.totalCount} in queue</span>
            )}
            {notificationSnapshot && notificationSnapshot.overflowCount > 0 && (
              <span>(+{notificationSnapshot.overflowCount} more)</span>
            )}
          </div>
          <div className="flex items-center space-x-1.5 shrink-0">
            {notificationSnapshot && notificationSnapshot.totalCount > 1 && (
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onClearAllNotifications();
                }}
                className="px-2 py-0.5 rounded-md bg-white/5 hover:bg-white/15 text-white/65 hover:text-white transition-colors cursor-pointer text-[9.5px]"
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
              className="px-2 py-0.5 rounded-md font-medium orbit-pill-badge-3d hover:bg-white/20 text-white transition-colors cursor-pointer text-[9.5px]"
            >
              Dismiss
            </button>
          </div>
        </div>
      </div>
    );
  }

  if (isMediaActive && mediaSession) {
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

    return (
      <div className="flex flex-col justify-between h-full select-none px-1 py-0.5 relative">
        {/* Source & Pin Header */}
        <div className="flex items-center justify-between w-full overflow-hidden">
          <div className="flex items-center space-x-1.5 min-w-0 flex-1 mr-2 overflow-hidden">
            <span
              className="text-[9.5px] font-bold uppercase tracking-wider px-2.5 py-0.5 rounded-full orbit-pill-badge-3d truncate max-w-[135px] inline-block shrink-0"
              style={{ color: activeThemeMeta.color }}
              title={mediaSession.source_title}
            >
              {mediaSession.source_title}
            </span>
            {mediaSession.is_pinned && (
              <span className="text-[8.5px] font-semibold text-amber-300 bg-amber-500/20 border border-amber-400/30 px-1.5 py-0.5 rounded shrink-0">
                PINNED
              </span>
            )}
          </div>
          <div className="flex items-center space-x-1.5 shrink-0 ml-auto">
            {notificationSnapshot &&
              notificationSnapshot.totalCount > 0 &&
              config.notificationsEnabled && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onViewNotification(notificationSnapshot?.notifications[0]);
                  }}
                  className="px-2 py-0.5 rounded-full orbit-pill-badge-3d flex items-center space-x-1 hover:text-white transition-colors cursor-pointer"
                  style={{ color: "var(--island-text)" }}
                  title={`${notificationSnapshot.totalCount} notification${
                    notificationSnapshot.totalCount > 1 ? "s" : ""
                  }`}
                >
                  <svg
                    className="w-3 h-3"
                    style={{ color: activeThemeMeta.accent }}
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                  >
                    <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
                    <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
                  </svg>
                  <span className="text-[9.5px] font-bold tabular-nums">
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
              className={`p-1 rounded-md transition-colors cursor-pointer ${
                isMediaCommandPending ? "opacity-50 cursor-wait" : ""
              } ${
                mediaSession.is_pinned
                  ? "text-amber-400 bg-amber-400/20 border border-amber-400/30"
                  : "text-white/45 hover:text-white/85 hover:bg-white/10"
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
          </div>
        </div>

        {/* 3D Floating Track Details Row */}
        <div className="flex items-center space-x-3 my-auto">
          <div
            className="w-12 h-12 rounded-xl overflow-hidden shrink-0 orbit-card-3d flex items-center justify-center"
            style={{
              boxShadow: `0 6px 16px -2px rgba(0, 0, 0, 0.75), 0 0 14px ${activeThemeMeta.borderGlow}`,
            }}
          >
            {mediaSession.track.thumbnail_base64 ? (
              <img
                src={mediaSession.track.thumbnail_base64}
                alt="Album Cover"
                className="w-full h-full object-cover"
              />
            ) : (
              <svg
                className="w-5 h-5"
                style={{ color: activeThemeMeta.color }}
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
              </svg>
            )}
          </div>
          <div className="flex flex-col min-w-0 flex-1 overflow-hidden">
            <span
              className="font-bold text-[12.5px] leading-tight truncate tracking-tight"
              style={{ color: "var(--island-text)" }}
              title={mediaSession.track.title}
            >
              {mediaSession.track.title || "Unknown Track"}
            </span>
            <span
              className="text-[11px] font-medium truncate mt-0.5"
              style={{ color: "var(--island-muted)" }}
            >
              {mediaSession.track.artist || "Unknown Artist"}
            </span>
            {mediaSession.track.album && (
              <span className="text-[9px] text-white/40 truncate mt-0.5">
                {mediaSession.track.album}
              </span>
            )}
          </div>
        </div>

        {/* Scrubber & 3D Playback Controls */}
        <div className="flex flex-col space-y-1">
          {isTimelineReliable && (
            <div className="flex items-center space-x-2">
              <span className="text-[9px] tabular-nums text-white/55 w-6 text-right shrink-0">
                {formatSeconds(displayPos)}
              </span>
              <input
                type="range"
                min={0}
                max={duration}
                step={1}
                disabled={!canSeek}
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
                className={`media-slider flex-1 ${
                  !canSeek ? "cursor-default opacity-60" : ""
                }`}
                style={
                  {
                    background: `linear-gradient(to right, ${activeThemeMeta.color} 0%, ${activeThemeMeta.color} ${progressPercent}%, rgba(255, 255, 255, 0.16) ${progressPercent}%, rgba(255, 255, 255, 0.16) 100%)`,
                    "--slider-accent": activeThemeMeta.color,
                    "--slider-glow": activeThemeMeta.borderGlow,
                  } as React.CSSProperties
                }
              />
              <span className="text-[9px] tabular-nums text-white/55 w-6 text-left shrink-0">
                {formatSeconds(duration)}
              </span>
            </div>
          )}

          <div className="flex items-center justify-center space-x-6 pt-0.5">
            <button
              type="button"
              title="Previous"
              disabled={
                isMediaCommandPending || !mediaSession.controls.can_skip_previous
              }
              onClick={(e) => {
                e.stopPropagation();
                onMediaCommand("previous");
              }}
              className={`text-white/80 hover:text-white disabled:opacity-20 transition-transform active:scale-90 p-1 rounded-full hover:bg-white/10 cursor-pointer ${
                isMediaCommandPending ? "pointer-events-none opacity-40" : ""
              }`}
            >
              <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                <path d="M6 6h2v12H6zm3.5 6l8.5 6V6z" />
              </svg>
            </button>

            <button
              type="button"
              title={
                mediaSession.playback_status === "playing" ? "Pause" : "Play"
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
              className={`w-9 h-9 rounded-full orbit-pill-badge-3d hover:bg-white/25 text-white flex items-center justify-center transition-all active:scale-90 disabled:opacity-30 cursor-pointer ${
                isMediaCommandPending
                  ? "opacity-60 cursor-wait pointer-events-none"
                  : ""
              }`}
              style={{
                boxShadow: `0 3px 10px rgba(0,0,0,0.6), 0 0 12px ${activeThemeMeta.borderGlow}`,
              }}
            >
              {mediaSession.playback_status === "playing" ? (
                <svg
                  className="w-[18px] h-[18px]"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                >
                  <path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z" />
                </svg>
              ) : (
                <svg
                  className="w-[18px] h-[18px] translate-x-0.5"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                >
                  <path d="M8 5v14l11-7z" />
                </svg>
              )}
            </button>

            <button
              type="button"
              title="Next"
              disabled={
                isMediaCommandPending || !mediaSession.controls.can_skip_next
              }
              onClick={(e) => {
                e.stopPropagation();
                onMediaCommand("next");
              }}
              className={`text-white/80 hover:text-white disabled:opacity-20 transition-transform active:scale-90 p-1 rounded-full hover:bg-white/10 cursor-pointer ${
                isMediaCommandPending ? "pointer-events-none opacity-40" : ""
              }`}
            >
              <svg className="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                <path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z" />
              </svg>
            </button>
          </div>
        </div>
      </div>
    );
  }

  /* Expanded Idle Home: Volumetric 3D Core Aura + Sculpted Clock & Date + 3D Privacy Pill
     Matches reference-idle-clock.png and reference-microphone-status.png */
  return (
    <div className="flex flex-col items-center justify-center h-full text-center select-none py-1 relative">
      {/* 3D Volumetric Center Radial Aura (from reference-idle-clock.png) */}
      <div className="absolute inset-0 orbit-core-aura rounded-2xl" />

      {/* Top-Left File Shelf Toggle Pill */}
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          setShowFileShelf?.(true);
        }}
        className="absolute top-0.5 left-1 px-2 py-0.5 rounded-full orbit-pill-badge-3d flex items-center space-x-1 hover:text-white transition-colors cursor-pointer z-10"
        style={{ color: "var(--island-text)" }}
        title={`Open File Shelf (${shelfItems.length} item${
          shelfItems.length === 1 ? "" : "s"
        })`}
      >
        <svg
          className="w-3 h-3 text-blue-400"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
        </svg>
        <span className="text-[10px] font-bold tabular-nums">
          Shelf {shelfItems.length > 0 ? `(${shelfItems.length})` : ""}
        </span>
      </button>

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
            className="absolute top-0.5 right-1 px-2 py-0.5 rounded-full orbit-pill-badge-3d flex items-center space-x-1 hover:text-white transition-colors cursor-pointer z-10"
            style={{ color: "var(--island-text)" }}
            title={`${notificationSnapshot.totalCount} unread notification${
              notificationSnapshot.totalCount > 1 ? "s" : ""
            }`}
          >
            <svg
              className="w-3 h-3"
              style={{ color: activeThemeMeta.accent }}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
            >
              <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
              <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
            </svg>
            <span className="text-[10px] font-bold tabular-nums">
              {notificationSnapshot.totalCount}
            </span>
          </button>
        )}

      {/* 3D Floating Clock Typography */}
      <div
        className="relative z-10 text-[52px] font-bold tracking-tight flex items-baseline justify-center leading-none select-none tabular-nums orbit-clock-3d"
        style={{
          color: "var(--island-text)",
          transform: "translateZ(14px)",
        }}
      >
        <span>{timeDisplay.timeStr}</span>
        {timeDisplay.ampm && (
          <span
            className="text-[16px] font-semibold uppercase ml-2 align-baseline"
            style={{ color: "var(--island-muted)" }}
          >
            {timeDisplay.ampm}
          </span>
        )}
      </div>

      {/* Date Subtitle */}
      <div
        className="relative z-10 text-[13px] font-semibold tracking-normal mt-1.5"
        style={{
          color: "var(--island-muted)",
          textShadow: "0 1px 3px rgba(0, 0, 0, 0.7)",
        }}
      >
        {formattedDate}
      </div>

      {/* 3D Frosted Privacy Pill matching reference-microphone-status.png */}
      {expandedPrivacySentence && (
        <div
          className="relative z-10 mt-2.5 px-3 py-1 rounded-full orbit-pill-badge-3d flex items-center space-x-1.5 text-[10.5px] font-medium max-w-[280px] truncate"
          style={{ color: "var(--island-muted)" }}
        >
          <span
            className="w-2 h-2 rounded-full shrink-0"
            style={{
              backgroundColor:
                privacyState?.cameraActive && privacyState?.microphoneActive
                  ? "#06b6d4"
                  : privacyState?.cameraActive
                  ? "#22c55e"
                  : "#f59e0b",
              boxShadow:
                privacyState?.cameraActive && privacyState?.microphoneActive
                  ? "0 0 8px #06b6d4"
                  : privacyState?.cameraActive
                  ? "0 0 8px #22c55e"
                  : "0 0 8px #f59e0b",
            }}
          />
          <span className="truncate">{expandedPrivacySentence}</span>
        </div>
      )}

      {/* Quick-Timer Bar / Active Countdown Bar */}
      <div className="relative z-10 mt-2 flex items-center justify-center">
        {timerRemainingSecs > 0 ? (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onStopTimer?.();
            }}
            className="px-3 py-1 rounded-full orbit-pill-badge-3d flex items-center space-x-1.5 text-[11px] font-semibold text-amber-300 hover:bg-amber-500/20 transition-all cursor-pointer shadow-[0_0_8px_rgba(245,158,11,0.25)]"
            title="Click to cancel timer"
          >
            <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
            <span>Timer: {formatSeconds(timerRemainingSecs)}</span>
            <span className="text-[9.5px] text-white/50 ml-1">✕ Stop</span>
          </button>
        ) : !expandedPrivacySentence ? (
          <div className="flex items-center space-x-1.5 py-0.5">
            <span className="text-[10px] text-white/40 font-medium mr-0.5">Timer:</span>
            {[
              { label: "5m", secs: 300 },
              { label: "15m", secs: 900 },
              { label: "25m", secs: 1500 },
              { label: "45m", secs: 2700 },
            ].map(({ label, secs }) => (
              <button
                key={label}
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onStartTimer?.(secs);
                }}
                className="px-2 py-0.5 rounded-md text-[9.5px] font-semibold orbit-pill-badge-3d text-white/70 hover:text-white hover:bg-white/15 transition-colors cursor-pointer"
                title={`Start ${label} countdown timer`}
              >
                {label}
              </button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
};
