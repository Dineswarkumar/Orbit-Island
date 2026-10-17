import React from "react";
import { AppConfig } from "../../core/types/config";
import { ThemeMeta } from "../../core/theme";
import { MediaSessionSnapshot } from "../../core/types/media";
import { NotificationItem } from "../../core/types/notifications";
import { NotificationBrandIcon } from "./NotificationBrandIcon";

export interface SystemUtilityAlert {
  kind: "charger_on" | "charger_off" | "wifi_on" | "wifi_off" | "bt_on" | "bt_off" | "hotspot_on" | "hotspot_off";
  label: string;
  detail: string;
  color: string;
}

interface CompactCapsuleProps {
  isExpanded: boolean;
  prefersReducedMotion: boolean;
  config: AppConfig;
  activeThemeMeta: ThemeMeta;
  transientNotif: NotificationItem | null;
  isNotifEntering: boolean;
  isNotifHovered: boolean;
  setIsNotifHovered: (hovered: boolean) => void;
  onViewNotification: (notif: NotificationItem) => void;
  onDismissNotification: (id: number) => void;
  isMediaActive: boolean;
  mediaSession: MediaSessionSnapshot | null;
  timeDisplay: { timeStr: string; ampm: string };
  privacyLight: React.ReactNode;
  timerRemainingSecs?: number;
  timerTotalSecs?: number;
  onStopTimer?: () => void;
  utilityAlert?: SystemUtilityAlert | null;
}

const formatTimerCountdown = (totalSecs: number) => {
  if (isNaN(totalSecs) || totalSecs <= 0) return "0:00";
  const mins = Math.floor(totalSecs / 60);
  const secs = Math.floor(totalSecs % 60);
  return `${mins}:${String(secs).padStart(2, "0")}`;
};

export const CompactCapsule: React.FC<CompactCapsuleProps> = ({
  isExpanded,
  prefersReducedMotion,
  config,
  activeThemeMeta,
  transientNotif,
  isNotifEntering,
  isNotifHovered,
  setIsNotifHovered,
  onViewNotification,
  onDismissNotification,
  isMediaActive,
  mediaSession,
  timeDisplay,
  privacyLight,
  timerRemainingSecs = 0,
  timerTotalSecs = 0,
  onStopTimer,
  utilityAlert = null,
}) => {
  return (
    <div
      className="absolute inset-0 flex items-center justify-between"
      style={{
        opacity: isExpanded ? 0 : 1,
        transform: isExpanded
          ? "translate3d(0, -6px, -32px) rotateX(22deg) scale(0.85)"
          : "translate3d(0, 0, 0) scale(1)",
        pointerEvents: isExpanded ? "none" : "auto",
        transition: prefersReducedMotion
          ? "none"
          : isExpanded
          ? "opacity 110ms ease-out 0ms, transform 200ms cubic-bezier(0.32, 0, 0.67, 0) 0ms"
          : "opacity 180ms ease-out 90ms, transform 280ms cubic-bezier(0.22, 1.25, 0.36, 1) 70ms",
      }}
    >
      {transientNotif && config.notificationsEnabled ? (
        /* Intermediate Transient 3D Notification Capsule (316x56 px) */
        <div
          onClick={() => onViewNotification(transientNotif)}
          onMouseEnter={() => setIsNotifHovered(true)}
          onMouseLeave={() => setIsNotifHovered(false)}
          className={`flex items-center w-full h-full pl-3 pr-3.5 select-none relative cursor-pointer ${
            isNotifEntering ? "notification-content-enter" : ""
          }`}
          title="Click to view notification details in ORBIT"
        >
          {/* Full-Bleed 36x36 Brand Icon */}
          <NotificationBrandIcon
            notification={transientNotif}
            size="compact"
            fallbackColor={activeThemeMeta.accent}
          />

          {/* Text Snippet + Bottom 3D Progress Bar (spans full remaining capsule width) */}
          <div
            className={`flex flex-col min-w-0 flex-1 overflow-hidden leading-tight text-left pl-3 justify-center ${
              privacyLight ? "pr-5" : "pr-1"
            }`}
          >
            {config.privacyMode ? (
              <>
                <div className="flex items-center space-x-1.5">
                  <span
                    className="text-[10px] font-semibold uppercase tracking-wider truncate"
                    style={{ color: "var(--island-muted)" }}
                  >
                    {transientNotif.appName}
                  </span>
                  {transientNotif.count > 1 && (
                    <span className="text-[8.5px] px-1 rounded-full bg-white/10 text-white/80 font-bold tabular-nums">
                      +{transientNotif.count}
                    </span>
                  )}
                  {transientNotif.isCritical && (
                    <span className="text-[7.5px] px-1 rounded bg-rose-500/30 text-rose-300 font-bold uppercase">
                      Alert
                    </span>
                  )}
                </div>
                <span
                  className="text-[11px] truncate font-bold mt-0.5"
                  style={{ color: "var(--island-text)" }}
                >
                  New notification
                </span>
              </>
            ) : (
              <>
                {/* Line 1: Sender / Title + App Name Tag */}
                <div className="flex items-center space-x-1.5 min-w-0">
                  <span className="text-[11px] font-bold text-white truncate">
                    {transientNotif.title || transientNotif.appName}
                  </span>
                  {transientNotif.title &&
                    transientNotif.title.toLowerCase() !==
                      transientNotif.appName.toLowerCase() && (
                      <span className="text-[9.5px] text-white/45 truncate font-medium shrink-0 max-w-[90px]">
                        · {transientNotif.appName}
                      </span>
                    )}
                  {transientNotif.count > 1 && (
                    <span className="text-[8.5px] px-1 rounded-full bg-white/10 text-white/80 font-bold tabular-nums shrink-0">
                      +{transientNotif.count}
                    </span>
                  )}
                  {transientNotif.isCritical && (
                    <span className="text-[7.5px] px-1 rounded bg-rose-500/30 text-rose-300 font-bold uppercase shrink-0">
                      Alert
                    </span>
                  )}
                </div>
                {/* Line 2: Actual Received Message Body! */}
                <span className="text-[10.5px] font-medium text-slate-300 truncate mt-0.5">
                  {transientNotif.body || transientNotif.title}
                </span>
              </>
            )}

            {/* 3D Expiry Progress Tube */}
            <div className="w-full h-[2.5px] rounded-full bg-white/10 mt-1.5 overflow-hidden">
              <div
                key={transientNotif.id}
                className="h-full rounded-full notification-progress-bar"
                style={
                  {
                    backgroundColor: activeThemeMeta.accent,
                    opacity: 0.85,
                    "--notif-duration": `${
                      (config.notificationPreviewDurationSec || 4) * 1000
                    }ms`,
                  } as React.CSSProperties
                }
              />
            </div>
          </div>

          {/* Hover actions overlay: View & Dismiss (absolute so they never squish text when unhovered) */}
          <div
            className={`absolute right-3 top-1/2 -translate-y-1/2 flex items-center space-x-1.5 px-1.5 py-1 rounded-lg bg-slate-950/90 border border-white/15 shadow-lg z-20 transition-opacity duration-150 ${
              isNotifHovered
                ? "opacity-100 pointer-events-auto"
                : "opacity-0 pointer-events-none"
            }`}
          >
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onViewNotification(transientNotif);
              }}
              className="px-2 py-0.5 rounded-md text-[9.5px] font-semibold orbit-pill-badge-3d text-white hover:bg-white/25 transition-colors cursor-pointer"
              title="View details in expanded island"
            >
              View
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onDismissNotification(transientNotif.id);
              }}
              className="px-2 py-0.5 rounded-md text-[9.5px] font-medium bg-white/10 text-white/75 hover:bg-white/20 hover:text-white transition-colors cursor-pointer"
              title="Dismiss from ORBIT"
            >
              Dismiss
            </button>
          </div>

          {/* Reserved right-edge privacy indicator */}
          {privacyLight && (
            <div className="absolute right-2.5 top-1/2 -translate-y-1/2 flex items-center justify-center pointer-events-auto z-10">
              {privacyLight}
            </div>
          )}
        </div>
      ) : utilityAlert ? (
        /* Sleek 3D Compact Utility Pill (140x40 px) */
        <div className="w-full h-full flex items-center justify-between px-3 select-none">
          <div className="flex items-center space-x-2.5 min-w-0">
            <div
              className="w-6 h-6 rounded-full flex items-center justify-center shrink-0"
              style={{
                backgroundColor: `${utilityAlert.color}22`,
                border: `1px solid ${utilityAlert.color}66`,
                color: utilityAlert.color,
              }}
            >
              {(utilityAlert.kind === "charger_on" || utilityAlert.kind === "charger_off") && (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
                </svg>
              )}
              {(utilityAlert.kind === "wifi_on" || utilityAlert.kind === "wifi_off") && (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M5 12.55a11 11 0 0 1 14.08 0" />
                  <path d="M1.42 9a16 16 0 0 1 21.16 0" />
                  <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
                  <line x1="12" y1="20" x2="12.01" y2="20" />
                </svg>
              )}
              {(utilityAlert.kind === "bt_on" || utilityAlert.kind === "bt_off") && (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="6.5 6.5 17.5 17.5 12 23 12 1 17.5 6.5 6.5 17.5" />
                </svg>
              )}
              {(utilityAlert.kind === "hotspot_on" || utilityAlert.kind === "hotspot_off") && (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="12" cy="12" r="2" />
                  <path d="M16.24 7.76a6 6 0 0 1 0 8.49m-8.48-.01a6 6 0 0 1 0-8.49" />
                  <path d="M19.07 4.93a10 10 0 0 1 0 14.14m-14.14 0a10 10 0 0 1 0-14.14" />
                </svg>
              )}
            </div>
            <div className="flex flex-col min-w-0 leading-tight">
              <span className="text-[10.5px] font-bold text-white leading-tight truncate">
                {utilityAlert.label}
              </span>
              <span
                className="text-[9px] font-semibold leading-tight truncate"
                style={{ color: utilityAlert.color }}
              >
                {utilityAlert.detail}
              </span>
            </div>
          </div>
          {privacyLight && (
            <div className="shrink-0 flex items-center justify-center pointer-events-auto">
              {privacyLight}
            </div>
          )}
        </div>
      ) : timerRemainingSecs > 0 ? (
        /* Compact 3D Timer Pill (140x40 px) - Matches reference-timer-and-media.png */
        <div className="flex items-center justify-between w-full h-full space-x-2 px-2.5 relative select-none">
          {/* Circular progress squircle */}
          <div
            onClick={(e) => {
              e.stopPropagation();
              onStopTimer?.();
            }}
            className="w-6 h-6 rounded-lg overflow-hidden shrink-0 flex items-center justify-center orbit-well-3d bg-white/[0.08] cursor-pointer"
            title="Click to cancel timer"
            style={{
              boxShadow: "0 2px 6px rgba(0,0,0,0.65), 0 0 8px rgba(245, 158, 11, 0.25)",
            }}
          >
            <svg className="w-4 h-4 -rotate-90" viewBox="0 0 20 20">
              <circle
                cx="10"
                cy="10"
                r="7.5"
                stroke="rgba(255, 255, 255, 0.18)"
                strokeWidth="2"
                fill="none"
              />
              <circle
                cx="10"
                cy="10"
                r="7.5"
                stroke="#f59e0b"
                strokeWidth="2"
                fill="none"
                strokeDasharray={47.12}
                strokeDashoffset={
                  47.12 *
                  (1 -
                    Math.min(
                      1,
                      Math.max(
                        0,
                        timerRemainingSecs / (timerTotalSecs || 1)
                      )
                    ))
                }
                strokeLinecap="round"
                style={{
                  transition: "stroke-dashoffset 900ms linear",
                }}
              />
            </svg>
          </div>

          {/* Timer Label + Remaining Countdown */}
          <div className="flex flex-col min-w-0 flex-1 overflow-hidden leading-tight text-left">
            <span className="text-[8.5px] font-medium truncate text-white/55">
              Focus Timer
            </span>
            <span className="font-bold text-[13px] tracking-tight text-white tabular-nums">
              {formatTimerCountdown(timerRemainingSecs)}
            </span>
          </div>

          {/* Mini Equalizer indicator if media is concurrently active */}
          {isMediaActive && mediaSession && (
            <div
              className="flex items-center space-x-[2px] h-3 shrink-0 pr-1"
              title={`${mediaSession.track.title} (${mediaSession.playback_status})`}
            >
              {[1, 2, 3].map((idx) => (
                <div
                  key={idx}
                  className={`eq-bar eq-bar-${idx} ${
                    mediaSession.playback_status === "playing"
                      ? "eq-bar-animate"
                      : ""
                  }`}
                  style={{
                    backgroundColor: activeThemeMeta.color,
                    width: "2px",
                    height: "100%",
                  }}
                />
              ))}
            </div>
          )}

          {/* Right-anchored compact privacy indicator */}
          {privacyLight && (
            <div className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center justify-center pointer-events-auto z-10">
              {privacyLight}
            </div>
          )}
        </div>
      ) : isMediaActive && mediaSession ? (
        /* Compact 3D Media Pill (140x40 px) - Matches reference-timer-and-media.png */
        <div className="flex items-center justify-between w-full h-full space-x-1.5 px-2 relative select-none">
          <div
            className="w-6 h-6 rounded-full overflow-hidden shrink-0 flex items-center justify-center orbit-well-3d"
            style={{
              boxShadow: `0 2px 6px rgba(0,0,0,0.65), 0 0 8px ${activeThemeMeta.borderGlow}`,
            }}
          >
            {mediaSession.track.thumbnail_base64 ? (
              <img
                src={mediaSession.track.thumbnail_base64}
                alt="Album Art"
                className="w-full h-full object-cover"
              />
            ) : (
              <svg
                className="w-3.5 h-3.5"
                style={{ color: activeThemeMeta.color }}
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
              </svg>
            )}
          </div>

          <div className="flex flex-col min-w-0 flex-1 overflow-hidden leading-tight text-left pr-1">
            <span
              className="font-semibold text-[9.5px] truncate tracking-tight"
              style={{ color: "var(--island-text)" }}
              title={mediaSession.track.title}
            >
              {mediaSession.track.title || "Unknown Track"}
            </span>
            <span className="text-[8px] font-medium truncate text-white/55">
              {mediaSession.track.artist || mediaSession.source_title || "Media"}
            </span>
          </div>

          {/* 3D Glowing Equalizer Bars */}
          <div className="flex items-center space-x-[2px] h-4 shrink-0 pr-3">
            {[1, 2, 3, 4].map((idx) => {
              const isPlaying = mediaSession.playback_status === "playing";
              return (
                <div
                  key={idx}
                  className={`eq-bar eq-bar-${idx} ${isPlaying ? "eq-bar-animate" : ""}`}
                  style={{
                    backgroundColor: activeThemeMeta.color,
                    transform: isPlaying
                      ? undefined
                      : `scaleY(${0.28 + ((idx % 3) * 0.16)})`,
                  }}
                />
              );
            })}
          </div>

          {/* Right-anchored compact privacy indicator */}
          {privacyLight && (
            <div className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center justify-center pointer-events-auto z-10">
              {privacyLight}
            </div>
          )}
        </div>
      ) : (
        /* Compact 3D Clock Pill (140x40 px: volumetric core glow + sculpted clock + AM/PM) */
        <div className="flex items-center justify-center w-full h-full relative select-none">
          <div className="flex items-baseline space-x-1.5 tabular-nums">
            <span
              className="font-bold text-[15px] tracking-tight leading-none orbit-clock-3d"
              style={{ color: "var(--island-text)" }}
            >
              {timeDisplay.timeStr}
            </span>
            {timeDisplay.ampm && (
              <span
                className="text-[9.5px] font-bold leading-none uppercase tracking-wider"
                style={{
                  color: activeThemeMeta.color,
                  textShadow: `0 0 8px ${activeThemeMeta.borderGlow}`,
                }}
              >
                {timeDisplay.ampm}
              </span>
            )}
          </div>

          {/* Right-anchored compact privacy indicator */}
          {privacyLight && (
            <div className="absolute right-2.5 top-1/2 -translate-y-1/2 flex items-center justify-center pointer-events-auto z-10">
              {privacyLight}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
