import React, { useLayoutEffect, useRef } from "react";
import { AppConfig } from "../../core/types/config";
import { ThemeMeta } from "../../core/theme";
import { MediaSessionSnapshot, AudioSpectrumState } from "../../core/types/media";
import { NotificationItem } from "../../core/types/notifications";
import { HardwareMetrics } from "../../core/types/hardware";
import { NotificationBrandIcon } from "./NotificationBrandIcon";
import { ipc } from "../../core/ipc/client";

export interface WeatherGlance {
  tempC: number;
  tempF: number;
  conditionCode: "clear" | "partly_cloudy" | "cloudy" | "rain" | "snow" | "storm";
  label: string;
}

export interface SystemUtilityAlert {
  kind:
    | "charger_on"
    | "charger_off"
    | "wifi_on"
    | "wifi_off"
    | "bt_on"
    | "bt_off"
    | "hotspot_on"
    | "hotspot_off"
    | "timer_done"
    | "download_done"
    | "stealth_on"
    | "stealth_off"
    | "event_due";
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
  weatherInfo?: WeatherGlance | null;
  isTimerPaused?: boolean;
  timerLabel?: string;
  onTogglePauseTimer?: () => void;
  onResetTimer?: () => void;
  onStartTimer?: (durationSecs: number, label?: string) => void;
  onDismissUtilityAlert?: () => void;
  hardwareMetrics?: HardwareMetrics | null;
  callDurationSecs?: number | null;
  onMeasuredCompactWidth?: (width: number) => void;
  audioSpectrum?: AudioSpectrumState | null;
  onOpenHome?: () => void;
}

const formatTimerCountdown = (totalSecs: number) => {
  if (isNaN(totalSecs) || totalSecs <= 0) return "0:00";
  const mins = Math.floor(totalSecs / 60);
  const secs = Math.floor(totalSecs % 60);
  return `${mins}:${String(secs).padStart(2, "0")}`;
};

const formatTransferByteSpeed = (bps: number): string => {
  if (bps >= 1024 * 1024) {
    return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`;
  }
  return `${Math.max(1, Math.round(bps / 1024))} KB/s`;
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
  weatherInfo = null,
  isTimerPaused = false,
  timerLabel = "Focus",
  onTogglePauseTimer,
  onResetTimer,
  onStartTimer,
  onDismissUtilityAlert,
  hardwareMetrics = null,
  callDurationSecs = null,
  onMeasuredCompactWidth,
  audioSpectrum = null,
  onOpenHome,
}) => {
  const isIncomingCall = Boolean(
    transientNotif &&
      /incoming call|voice call|video call|is calling|meeting started/i.test(
        `${transientNotif.title} ${transientNotif.body}`
      )
  );

  const isDownloading =
    config.networkActivityMode !== "off" &&
    (hardwareMetrics?.networkDownloadSpeedBps ?? 0) >= 250 * 1024;

  const isUploading =
    config.networkActivityMode !== "off" &&
    (hardwareMetrics?.networkUploadSpeedBps ?? 0) >= 150 * 1024;

  const compactContentRef = useRef<HTMLDivElement | null>(null);
  const lastReportedWidthRef = useRef<number>(0);

  useLayoutEffect(() => {
    if (!onMeasuredCompactWidth) return;

    const measureAndReport = () => {
      const el = compactContentRef.current;
      if (!el) return;
      // Use untransformed layout width (offsetWidth/scrollWidth) so 3D collapse transforms (scale(0.85) / rotateX) never clip content!
      const rawWidth = Math.ceil(Math.max(el.offsetWidth, el.scrollWidth));
      if (rawWidth <= 0) return;

      const padding = 26; // 13px left + 13px right padding
      const finalWidth = Math.max(82, Math.min(336, rawWidth + padding));
      if (Math.abs(lastReportedWidthRef.current - finalWidth) >= 2) {
        lastReportedWidthRef.current = finalWidth;
        onMeasuredCompactWidth(finalWidth);
      }
    };

    measureAndReport();

    const el = compactContentRef.current;
    if (!el) return;

    const ro = new ResizeObserver(() => {
      measureAndReport();
    });
    ro.observe(el);

    return () => {
      ro.disconnect();
    };
  }, [
    onMeasuredCompactWidth,
    isExpanded,
    timeDisplay.timeStr,
    timeDisplay.ampm,
    weatherInfo?.tempC,
    weatherInfo?.tempF,
    weatherInfo?.conditionCode,
    config.compactRightSlot,
    config.customRightText,
    config.customRightTextStyle,
    config.fontFamily,
    config.weatherUnit,
    hardwareMetrics?.screenRecordingActive,
    hardwareMetrics?.networkDownloadSpeedBps,
    hardwareMetrics?.networkUploadSpeedBps,
    Boolean(privacyLight),
    isMediaActive,
    mediaSession?.track.title,
    mediaSession?.track.artist,
    timerRemainingSecs,
    utilityAlert?.kind,
    callDurationSecs,
    isDownloading,
    isUploading,
  ]);

  const renderCompactRightSlot = () => {
    const effectiveMode =
      config.compactRightSlot === "smart"
        ? config.customRightText &&
          config.customRightText.trim() !== "" &&
          config.customRightText.trim() !== "ORBIT"
          ? "custom_text"
          : weatherInfo
          ? "weather"
          : "orbit_logo"
        : config.compactRightSlot;

    switch (effectiveMode) {
      case "orbit_logo":
        return (
          <div
            className="flex items-center border-l border-white/15 pl-1.5 shrink-0"
            title="ORBIT Dynamic Island"
          >
            <svg
              className="w-4 h-4 shrink-0 transition-transform hover:scale-110"
              viewBox="0 0 24 24"
              fill="none"
              style={{ filter: `drop-shadow(0 0 6px ${activeThemeMeta.color})` }}
            >
              <circle cx="12" cy="12" r="6" fill="url(#orbit-emblem-grad)" />
              <ellipse
                cx="12"
                cy="12"
                rx="10"
                ry="3.4"
                stroke={activeThemeMeta.accent}
                strokeWidth="1.6"
                transform="rotate(-24 12 12)"
              />
              <defs>
                <radialGradient id="orbit-emblem-grad" cx="40%" cy="40%" r="65%">
                  <stop offset="0%" stopColor="#ffffff" />
                  <stop offset="55%" stopColor={activeThemeMeta.color} />
                  <stop offset="100%" stopColor="#090d16" />
                </radialGradient>
              </defs>
            </svg>
          </div>
        );

      case "windows_logo":
        return (
          <div
            className="flex items-center border-l border-white/15 pl-1.5 shrink-0"
            title="Windows 11"
          >
            <svg
              className="w-3.5 h-3.5 shrink-0"
              viewBox="0 0 24 24"
              fill="none"
              style={{ filter: "drop-shadow(0 0 5px rgba(56, 189, 248, 0.45))" }}
            >
              <rect x="2.5" y="2.5" width="8.5" height="8.5" rx="1" fill="#38bdf8" />
              <rect x="13" y="2.5" width="8.5" height="8.5" rx="1" fill="#0ea5e9" />
              <rect x="2.5" y="13" width="8.5" height="8.5" rx="1" fill="#0284c7" />
              <rect x="13" y="13" width="8.5" height="8.5" rx="1" fill="#38bdf8" />
            </svg>
          </div>
        );

      case "apple_logo":
        return (
          <div
            className="flex items-center border-l border-white/15 pl-1.5 shrink-0"
            title="Apple HIG"
          >
            <svg
              className="w-3.5 h-3.5 shrink-0"
              viewBox="0 0 24 24"
              fill="#f8fafc"
              style={{ filter: "drop-shadow(0 0 4px rgba(255, 255, 255, 0.35))" }}
            >
              <path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.81-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M15.97 6.37c.62-.77 1.04-1.85.93-2.92-.93.04-2.07.63-2.73 1.4-.58.67-1.09 1.76-.95 2.81 1.04.08 2.12-.52 2.75-1.29z" />
            </svg>
          </div>
        );

      case "linux_logo":
        return (
          <div
            className="flex items-center border-l border-white/15 pl-1.5 shrink-0"
            title="Linux Tux"
          >
            <svg className="w-3.5 h-3.5 shrink-0" viewBox="0 0 24 24" fill="none">
              <path
                d="M12 2C9.5 2 7.5 4 7.5 6.5V11C6.1 11.8 5 13.3 5 15.5C5 18 7 19.5 8 20C9 20.5 10 21 12 21C14 21 15 20.5 16 20C17 19.5 19 18 19 15.5C19 13.3 17.9 11.8 16.5 11V6.5C16.5 4 14.5 2 12 2Z"
                fill="#f8fafc"
              />
              <path
                d="M12 4C10.5 4 9.5 5 9.5 6.5V10.5H14.5V6.5C14.5 5 13.5 4 12 4Z"
                fill="#0f172a"
              />
              <ellipse cx="10.5" cy="7" rx="0.75" ry="1" fill="#ffffff" />
              <ellipse cx="13.5" cy="7" rx="0.75" ry="1" fill="#ffffff" />
              <polygon points="12,8 10.5,10 13.5,10" fill="#fbbf24" />
              <ellipse cx="12" cy="15" rx="3.5" ry="4.5" fill="#ffffff" />
              <path
                d="M7 21C8 22 10 22 10 21"
                stroke="#fbbf24"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
              <path
                d="M14 21C14 22 16 22 17 21"
                stroke="#fbbf24"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            </svg>
          </div>
        );

      case "custom_text":
        return (
          <div className="flex items-center border-l border-white/15 pl-1.5 shrink-0">
            <span
              className={`text-[9.5px] font-extrabold tracking-wider uppercase orbit-right-badge-${
                config.customRightTextStyle || "neon_glow"
              }`}
              style={{ "--badge-glow": activeThemeMeta.color } as React.CSSProperties}
            >
              {(config.customRightText || "ORBIT").slice(0, 5)}
            </span>
          </div>
        );

      case "custom_image":
        if (config.customRightImageBase64) {
          return (
            <div className="flex items-center border-l border-white/15 pl-1.5 shrink-0">
              <div className="w-5 h-5 rounded-full overflow-hidden orbit-well-3d border border-white/25 shrink-0">
                <img
                  src={config.customRightImageBase64}
                  alt="Custom Avatar"
                  className="w-full h-full object-cover"
                />
              </div>
            </div>
          );
        }
        return (
          <div
            className="flex items-center border-l border-white/15 pl-1.5 shrink-0"
            title="ORBIT Dynamic Island"
          >
            <svg
              className="w-4 h-4 shrink-0"
              viewBox="0 0 24 24"
              fill="none"
              style={{ filter: `drop-shadow(0 0 6px ${activeThemeMeta.color})` }}
            >
              <circle cx="12" cy="12" r="6" fill="url(#orbit-emblem-grad)" />
              <ellipse
                cx="12"
                cy="12"
                rx="10"
                ry="3.4"
                stroke={activeThemeMeta.accent}
                strokeWidth="1.6"
                transform="rotate(-24 12 12)"
              />
            </svg>
          </div>
        );

      case "weather":
      default:
        if (
          (config.showWeather ||
            config.compactRightSlot === "weather" ||
            config.compactRightSlot === "smart") &&
          weatherInfo
        ) {
          return (
            <div
              className="flex items-center space-x-1 text-[11px] font-bold leading-none shrink-0 border-l border-white/15 pl-1.5"
              title={`${weatherInfo.label} (${weatherInfo.tempC}°C / ${weatherInfo.tempF}°F)`}
              style={{ color: "var(--island-muted)" }}
            >
              {weatherInfo.conditionCode === "rain" ||
              weatherInfo.conditionCode === "storm" ? (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96z"
                    fill="#ffffff"
                  />
                  {weatherInfo.conditionCode === "storm" ? (
                    <path d="M11 15h3l-2 5 4-3h-3l2-4h-4z" fill="#fbbf24" />
                  ) : (
                    <>
                      <line
                        x1="8"
                        y1="21"
                        x2="7"
                        y2="23"
                        stroke="#38bdf8"
                        strokeWidth="2"
                        strokeLinecap="round"
                      />
                      <line
                        x1="12"
                        y1="21"
                        x2="11"
                        y2="23"
                        stroke="#38bdf8"
                        strokeWidth="2"
                        strokeLinecap="round"
                      />
                      <line
                        x1="16"
                        y1="21"
                        x2="15"
                        y2="23"
                        stroke="#38bdf8"
                        strokeWidth="2"
                        strokeLinecap="round"
                      />
                    </>
                  )}
                </svg>
              ) : weatherInfo.conditionCode === "snow" ? (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96z"
                    fill="#ffffff"
                  />
                  <circle cx="8" cy="22" r="1" fill="#bae6fd" />
                  <circle cx="12" cy="22" r="1" fill="#bae6fd" />
                  <circle cx="16" cy="22" r="1" fill="#bae6fd" />
                </svg>
              ) : weatherInfo.conditionCode === "partly_cloudy" ? (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none">
                  <circle cx="15.5" cy="8.5" r="4.5" fill="#fbbf24" />
                  <path
                    d="M17.5 13c-.34 0-.67.04-.98.11C15.63 10.74 13.52 9 11 9c-2.9 0-5.3 2.1-5.87 4.9C3.15 14.34 1.5 16.2 1.5 18.5 1.5 21 3.5 23 6 23h11.5c2.49 0 4.5-2.01 4.5-4.5S19.99 13 17.5 13z"
                    fill="#ffffff"
                  />
                </svg>
              ) : weatherInfo.conditionCode === "cloudy" ? (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none">
                  <path
                    d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96z"
                    fill="#ffffff"
                  />
                </svg>
              ) : (
                <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none">
                  <circle cx="12" cy="12" r="5" fill="#fde047" stroke="#fbbf24" strokeWidth="1" />
                  <path
                    d="M12 2v2.5m0 15V22M4.22 4.22l1.77 1.77m12.02 12.02l1.77 1.77M2 12h2.5m15 0H22M4.22 19.78l1.77-1.77m12.02-12.02l1.77-1.77"
                    stroke="#fbbf24"
                    strokeWidth="2"
                    strokeLinecap="round"
                  />
                </svg>
              )}
              <span>
                {config.weatherUnit === "F" ? `${weatherInfo.tempF}°` : `${weatherInfo.tempC}°`}
              </span>
            </div>
          );
        }
        return null;
    }
  };

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
        /* Intermediate Transient 3D Notification Capsule */
        <div
          onClick={() => {
            ipc
              .activateAppSource(
                transientNotif.appId,
                transientNotif.appName,
                transientNotif.title
              )
              .catch(() => {});
            onDismissNotification(transientNotif.id);
          }}
          onMouseMove={() => {
            if (!isNotifHovered && setIsNotifHovered) {
              setIsNotifHovered(true);
            }
          }}
          className={`flex flex-col justify-between w-full h-full px-3.5 ${
            isNotifHovered ? "py-2.5" : "py-1.5"
          } select-none relative cursor-pointer ${
            isNotifEntering ? "notification-content-enter" : ""
          }`}
          title={`Click to open ${transientNotif.appName}`}
        >
          {/* Top Row: Brand Icon + Enlarged Sender/Message Text */}
          <div className="flex items-center w-full min-w-0 flex-1 overflow-hidden">
            <NotificationBrandIcon
              notification={transientNotif}
              size="compact"
              fallbackColor={activeThemeMeta.accent}
            />

            {/* Text Snippet */}
            <div
              className={`flex flex-col min-w-0 flex-1 overflow-hidden leading-tight text-left pl-3 justify-center ${
                privacyLight ? "pr-2" : "pr-1"
              }`}
            >
              {config.privacyMode ? (
                <>
                  <div className="flex items-center space-x-1.5">
                    <span
                      className={`${
                        isNotifHovered ? "text-[13px]" : "text-[10px]"
                      } font-semibold uppercase tracking-wider truncate transition-all duration-150`}
                      style={{ color: "var(--island-muted)" }}
                    >
                      {transientNotif.appName}
                    </span>
                    {transientNotif.count > 1 && (
                      <span className="text-[9.5px] px-1 rounded-full bg-white/10 text-white/80 font-bold tabular-nums">
                        +{transientNotif.count}
                      </span>
                    )}
                    {isIncomingCall ? (
                      <span className="text-[8.5px] px-1 rounded bg-emerald-500/30 text-emerald-300 font-bold uppercase animate-pulse">
                        Call
                      </span>
                    ) : (
                      transientNotif.isCritical && (
                        <span className="text-[8.5px] px-1 rounded bg-rose-500/30 text-rose-300 font-bold uppercase">
                          Alert
                        </span>
                      )
                    )}
                  </div>
                  <span
                    className={`${
                      isNotifHovered ? "text-[14.5px]" : "text-[11px]"
                    } truncate font-bold mt-0.5 transition-all duration-150`}
                    style={{ color: "var(--island-text)" }}
                  >
                    {isIncomingCall ? "Incoming Call" : "New notification"}
                  </span>
                </>
              ) : (
                <>
                  {/* Line 1: Sender / Title + App Name Tag */}
                  <div className="flex items-center space-x-1.5 min-w-0">
                    <span
                      className={`${
                        isNotifHovered ? "text-[14.5px]" : "text-[11px]"
                      } font-bold text-white truncate transition-all duration-150`}
                    >
                      {transientNotif.title || transientNotif.appName}
                    </span>
                    {transientNotif.title &&
                      transientNotif.title.toLowerCase() !==
                        transientNotif.appName.toLowerCase() && (
                        <span
                          className={`${
                            isNotifHovered ? "text-[12px]" : "text-[9.5px]"
                          } text-white/55 truncate font-medium shrink-0 max-w-[115px] transition-all duration-150`}
                        >
                          · {transientNotif.appName}
                        </span>
                      )}
                    {transientNotif.count > 1 && (
                      <span className="text-[9.5px] px-1.5 rounded-full bg-white/10 text-white/85 font-bold tabular-nums shrink-0">
                        +{transientNotif.count}
                      </span>
                    )}
                    {isIncomingCall ? (
                      <span className="text-[8.5px] px-1.5 rounded-full bg-emerald-500/30 text-emerald-300 font-bold uppercase tracking-wider animate-pulse shrink-0">
                        📞 Call
                      </span>
                    ) : (
                      transientNotif.isCritical && (
                        <span className="text-[8.5px] px-1 rounded bg-rose-500/30 text-rose-300 font-bold uppercase shrink-0">
                          Alert
                        </span>
                      )
                    )}
                  </div>
                  {/* Line 2: Actual Received Message Body */}
                  <span
                    className={`${
                      isNotifHovered
                        ? "text-[13px] text-slate-100 line-clamp-2 whitespace-normal leading-snug mt-0.5 pb-0.5"
                        : "text-[10.5px] text-slate-300 truncate mt-0.5"
                    } font-medium transition-all duration-150`}
                  >
                    {transientNotif.body || transientNotif.title}
                  </span>
                </>
              )}

              {/* Resting Progress Bar */}
              {!isNotifHovered && (
                <div className="w-full h-[2.5px] rounded-full bg-white/10 mt-1.5 overflow-hidden">
                  <div
                    key={transientNotif.id}
                    className="h-full rounded-full notification-progress-bar"
                    style={
                      {
                        backgroundColor: isIncomingCall ? "#10b981" : activeThemeMeta.accent,
                        opacity: 0.85,
                        animationPlayState: "running",
                        "--notif-duration": `${
                          (config.notificationPreviewDurationSec || 4) * 1000
                        }ms`,
                      } as React.CSSProperties
                    }
                  />
                </div>
              )}
            </div>

            {/* Inline Privacy Indicator */}
            {privacyLight && (
              <div className="ml-auto pl-1.5 shrink-0 flex items-center">
                {privacyLight}
              </div>
            )}
          </div>

          {/* Bottom Extra-Opening Action Row */}
          {isNotifHovered && (
            <div
              onPointerDown={(e) => e.stopPropagation()}
              className="flex items-center justify-between w-full pt-1.5 mt-1 border-t border-white/15 shrink-0 z-20"
            >
              <div className="flex items-center space-x-1.5">
                <button
                  type="button"
                  onPointerDown={(e) => e.stopPropagation()}
                  onClick={(e) => {
                    e.stopPropagation();
                    ipc
                      .activateAppSource(
                        transientNotif.appId,
                        transientNotif.appName,
                        transientNotif.title
                      )
                      .catch(() => {});
                    onDismissNotification(transientNotif.id);
                  }}
                  className={`px-3.5 py-1 rounded-lg text-[11.5px] font-bold ${
                    isIncomingCall
                      ? "bg-emerald-500/25 text-emerald-200 border border-emerald-400/40 hover:bg-emerald-500/40"
                      : "bg-cyan-500/20 text-cyan-200 border border-cyan-400/35 hover:bg-cyan-500/35"
                  } transition-colors cursor-pointer`}
                  title={isIncomingCall ? "Answer / Open Call window" : `Open ${transientNotif.appName}`}
                >
                  {isIncomingCall ? "Answer / Open Call" : "Open"}
                </button>
                <button
                  type="button"
                  onPointerDown={(e) => e.stopPropagation()}
                  onClick={(e) => {
                    e.stopPropagation();
                    onViewNotification?.(transientNotif);
                  }}
                  className="px-2.5 py-1 rounded-lg text-[11.5px] font-medium bg-white/10 text-white/80 hover:bg-white/20 hover:text-white transition-colors cursor-pointer"
                  title="View full details inside ORBIT"
                >
                  View
                </button>
              </div>
              <button
                type="button"
                onPointerDown={(e) => e.stopPropagation()}
                onClick={(e) => {
                  e.stopPropagation();
                  onDismissNotification(transientNotif.id);
                }}
                className="px-3.5 py-1 rounded-lg text-[11.5px] font-medium bg-white/10 text-white/80 hover:bg-rose-500/25 hover:text-rose-200 transition-colors cursor-pointer"
                title="Dismiss from ORBIT"
              >
                Dismiss
              </button>
            </div>
          )}
        </div>
      ) : utilityAlert ? (
        utilityAlert.kind === "timer_done" ? (
          /* Prominent Timer Completed Celebration */
          <div
            onClick={(e) => {
              e.stopPropagation();
              onDismissUtilityAlert?.();
            }}
            className="w-full h-full flex flex-col justify-between px-3 py-1.5 select-none relative cursor-pointer"
            style={{
              background:
                "radial-gradient(ellipse 80% 90% at 20% 50%, rgba(245, 158, 11, 0.24) 0%, transparent 75%)",
            }}
          >
            <div className="flex items-center justify-between w-full flex-1 min-w-0">
              <div className="flex items-center space-x-2.5 min-w-0 flex-1 mr-2">
                <div
                  className="w-8 h-8 rounded-full flex items-center justify-center shrink-0 animate-bounce"
                  style={{
                    backgroundColor: "rgba(245, 158, 11, 0.25)",
                    border: "1.5px solid rgba(251, 191, 36, 0.8)",
                    boxShadow: "0 0 14px rgba(245, 158, 11, 0.65)",
                    color: "#fbbf24",
                  }}
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
                    <path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9" />
                    <path d="M10.3 21a1.94 1.94 0 0 0 3.4 0" />
                    <path d="M4 2C2.8 3.7 2 5.7 2 8" />
                    <path d="M22 8c0-2.3-.8-4.3-2-6" />
                  </svg>
                </div>
                <div className="flex flex-col min-w-0 leading-tight">
                  <span className="text-[12.5px] font-extrabold text-amber-300 tracking-tight truncate">
                    Time&apos;s Up! · Timer Done
                  </span>
                  <span className="text-[10.5px] font-semibold text-white/85 truncate mt-0.5">
                    {utilityAlert.label} finished
                  </span>
                </div>
              </div>

              <div className="flex items-center space-x-1.5 shrink-0">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onDismissUtilityAlert?.();
                    onStartTimer?.(60);
                  }}
                  className="px-2 py-0.5 rounded-md text-[10.5px] font-bold bg-amber-500/25 text-amber-200 border border-amber-400/45 hover:bg-amber-500/40 transition-colors cursor-pointer"
                  title="Add 1 minute"
                >
                  +1m
                </button>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onDismissUtilityAlert?.();
                  }}
                  className="px-2 py-0.5 rounded-md text-[10.5px] font-semibold bg-white/15 text-white hover:bg-white/25 transition-colors cursor-pointer"
                  title="Dismiss timer alert"
                >
                  OK
                </button>
              </div>
            </div>

            {/* 6-Second Auto-Revert Progress Bar */}
            <div className="w-full h-[2.5px] rounded-full bg-white/10 mt-1 overflow-hidden shrink-0">
              <div
                className="h-full rounded-full notification-progress-bar"
                style={
                  {
                    backgroundColor: "#fbbf24",
                    opacity: 0.9,
                    "--notif-duration": "6000ms",
                  } as React.CSSProperties
                }
              />
            </div>
          </div>
        ) : (
          /* Sleek 3D Compact Utility Pill */
          <div className="w-full h-full flex items-center justify-center px-3 select-none relative">
            <div
              ref={compactContentRef}
              className="inline-flex items-center space-x-2.5 min-w-0 shrink-0"
            >
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
                {utilityAlert.kind === "download_done" && (
                  <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="7 10 12 15 17 10" />
                    <line x1="12" y1="15" x2="12" y2="3" />
                  </svg>
                )}
                {utilityAlert.kind === "stealth_on" && (
                  <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
                    <line x1="1" y1="1" x2="23" y2="23" />
                  </svg>
                )}
                {utilityAlert.kind === "stealth_off" && (
                  <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                    <circle cx="12" cy="12" r="3" />
                  </svg>
                )}
                {utilityAlert.kind === "event_due" && (
                  <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
                    <line x1="16" y1="2" x2="16" y2="6" />
                    <line x1="8" y1="2" x2="8" y2="6" />
                    <line x1="3" y1="10" x2="21" y2="10" />
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
              {privacyLight && (
                <div
                  className="flex items-center justify-center shrink-0 pl-1.5 border-l border-white/15"
                  title="Camera/Microphone Active"
                >
                  {privacyLight}
                </div>
              )}
            </div>
          </div>
        )
      ) : timerRemainingSecs > 0 ? (
        isMediaActive && mediaSession ? (
          /* Simultaneous Dual 3D Pill (256x40 px) */
          <div className="flex items-center justify-center w-full h-full px-2.5 select-none relative">
            <div
              ref={compactContentRef}
              className="inline-flex items-center justify-between space-x-1.5 shrink-0 min-w-0"
            >
              {/* Screen Recording Active Dot */}
              {hardwareMetrics?.screenRecordingActive && (
                <div
                  className="flex items-center space-x-1 shrink-0 pr-1.5 border-r border-white/15"
                  title="Screen Recording active"
                >
                  <span
                    className="w-2 h-2 rounded-full bg-rose-500 animate-pulse"
                    style={{ boxShadow: "0 0 6px #f43f5e" }}
                  />
                </div>
              )}

              {/* Left: Interactive Compact Timer Module */}
            <div className="flex items-center space-x-1 shrink-0">
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  onTogglePauseTimer ? onTogglePauseTimer() : onStopTimer?.();
                }}
                className="flex items-center space-x-1.5 px-1 py-0.5 rounded-md hover:bg-white/10 transition-colors cursor-pointer"
                title={isTimerPaused ? "Click to resume timer" : "Click to pause timer"}
              >
                <div
                  className="w-5 h-5 rounded-md overflow-hidden shrink-0 flex items-center justify-center orbit-well-3d bg-white/[0.08]"
                  style={{
                    boxShadow: isTimerPaused
                      ? "0 2px 6px rgba(0,0,0,0.65), 0 0 8px rgba(245, 158, 11, 0.4)"
                      : "0 2px 6px rgba(0,0,0,0.65), 0 0 8px rgba(245, 158, 11, 0.25)",
                  }}
                >
                  <svg className="w-3.5 h-3.5 -rotate-90" viewBox="0 0 20 20">
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
                      stroke={isTimerPaused ? "#eab308" : "#f59e0b"}
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
                <span
                  className={`font-bold text-[11.5px] tracking-tight tabular-nums ${
                    isTimerPaused ? "text-amber-400/75" : "text-amber-300"
                  }`}
                >
                  {formatTimerCountdown(timerRemainingSecs)}
                </span>
              </button>
              {onResetTimer && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onResetTimer();
                  }}
                  className="p-0.5 rounded text-white/45 hover:text-amber-300 hover:bg-white/10 transition-colors cursor-pointer"
                  title="Reset timer"
                >
                  <svg className="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                    <path d="M3 3v5h5" />
                  </svg>
                </button>
              )}
            </div>

            {/* Vertical 3D Divider */}
            <div className="w-[1px] h-4 bg-white/15 shrink-0" />

            {/* Right: Spacious Media Module */}
            <div className="flex items-center space-x-1.5 min-w-0 flex-1 justify-between pl-1">
              <div
                className="w-5 h-5 rounded-full overflow-hidden shrink-0 flex items-center justify-center orbit-well-3d"
                style={{
                  boxShadow: `0 1px 4px rgba(0,0,0,0.5), 0 0 6px ${activeThemeMeta.borderGlow}`,
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
                    className="w-3 h-3"
                    style={{ color: activeThemeMeta.color }}
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path d="M12 3v10.55c-.59-.34-1.27-.55-2-.55-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4V7h4V3h-6z" />
                  </svg>
                )}
              </div>
              <div className="flex flex-col min-w-0 flex-1 text-left px-1 overflow-hidden leading-tight max-w-[78px]">
                <span
                  className="text-[9.5px] font-semibold truncate text-white leading-tight"
                  title={mediaSession.track.title}
                >
                  {mediaSession.track.title || "Media"}
                </span>
                <span className="text-[8px] font-medium truncate text-white/50 leading-tight">
                  {mediaSession.track.artist || mediaSession.source_title || "Playing"}
                </span>
              </div>
              <div className="flex items-center space-x-[2px] h-3.5 shrink-0">
                {[1, 2, 3].map((idx) => {
                  const isPlaying = mediaSession.playback_status === "playing";
                  const barVal = isPlaying ? (audioSpectrum?.compactBars?.[idx - 1] ?? 0) : 0;
                  const hasFreq = isPlaying && (audioSpectrum?.peak ?? 0) >= 0.005 && barVal > 0.005;
                  const scaleY = hasFreq
                    ? Math.max(0.18, Math.min(1, 0.12 + barVal * 0.88))
                    : 0.02;
                  return (
                    <div
                      key={idx}
                      className="eq-bar"
                      style={{
                        backgroundColor: activeThemeMeta.color,
                        width: "2px",
                        height: "100%",
                        transform: `scaleY(${scaleY.toFixed(2)})`,
                        transition: "transform 95ms ease-out",
                        opacity: hasFreq ? 0.95 : 0.0,
                      }}
                    />
                  );
                })}
              </div>
              {privacyLight && (
                <div
                  className="flex items-center justify-center shrink-0 pl-1.5 border-l border-white/15"
                  title="Camera/Microphone Active"
                >
                  {privacyLight}
                </div>
              )}
            </div>
          </div>
        </div>
        ) : (
          /* Compact 3D Timer Pill (140x40 px) */
          <div className="flex items-center justify-center w-full h-full px-2.5 relative select-none">
            <div
              ref={compactContentRef}
              className="inline-flex items-center justify-between space-x-2 shrink-0 min-w-0"
            >
              {/* Screen Recording Active Dot */}
              {hardwareMetrics?.screenRecordingActive && (
                <div
                  className="flex items-center space-x-1 shrink-0 pr-1.5 border-r border-white/15"
                  title="Screen Recording active"
                >
                  <span
                    className="w-2 h-2 rounded-full bg-rose-500 animate-pulse"
                    style={{ boxShadow: "0 0 6px #f43f5e" }}
                  />
                </div>
              )}

              {/* Circular progress squircle */}
            <div
              onClick={(e) => {
                e.stopPropagation();
                onTogglePauseTimer ? onTogglePauseTimer() : onStopTimer?.();
              }}
              className="w-6 h-6 rounded-lg overflow-hidden shrink-0 flex items-center justify-center orbit-well-3d bg-white/[0.08] cursor-pointer"
              title={isTimerPaused ? "Resume timer" : "Click to pause timer"}
              style={{
                boxShadow: isTimerPaused
                  ? "0 2px 6px rgba(0,0,0,0.65), 0 0 8px rgba(245, 158, 11, 0.4)"
                  : "0 2px 6px rgba(0,0,0,0.65), 0 0 8px rgba(245, 158, 11, 0.25)",
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
                  stroke={isTimerPaused ? "#eab308" : "#f59e0b"}
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
            <div className="flex flex-col min-w-0 flex-1 overflow-hidden leading-tight text-left max-w-[78px]">
              <span className="text-[8.5px] font-medium truncate text-white/55">
                {isTimerPaused ? `${timerLabel || "Timer"} (Paused)` : `${timerLabel || "Focus"} Timer`}
              </span>
              <span className="font-bold text-[13px] tracking-tight text-white tabular-nums">
                {formatTimerCountdown(timerRemainingSecs)}
              </span>
            </div>

            {/* Stop timer button */}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onStopTimer?.();
              }}
              className="w-4 h-4 rounded-full flex items-center justify-center text-white/40 hover:text-white hover:bg-white/15 transition-colors cursor-pointer shrink-0"
              title="Stop timer"
            >
              ✕
            </button>

            {privacyLight && (
              <div
                className="flex items-center justify-center shrink-0 pl-1.5 border-l border-white/15"
                title="Camera/Microphone Active"
              >
                {privacyLight}
              </div>
            )}
          </div>
        </div>
      )
    ) : isMediaActive && mediaSession ? (
        /* Compact 3D Media Pill (140x40 px) */
        <div className="flex items-center justify-center w-full h-full px-2.5 relative select-none">
          <div
            ref={compactContentRef}
            className="inline-flex items-center justify-between space-x-1.5 shrink-0 min-w-0"
          >
            {/* Screen Recording Active Dot */}
            {hardwareMetrics?.screenRecordingActive && (
              <div
                className="flex items-center space-x-1 shrink-0 pr-1.5 border-r border-white/15"
                title="Screen Recording active"
              >
                <span
                  className="w-2 h-2 rounded-full bg-rose-500 animate-pulse"
                  style={{ boxShadow: "0 0 6px #f43f5e" }}
                />
              </div>
            )}

            {/* Direct Time Display (clicking opens Expanded Island Home) */}
            <div
              onClick={(e) => {
                e.stopPropagation();
                onOpenHome?.();
              }}
              className="flex items-baseline space-x-0.5 shrink-0 pr-1.5 border-r border-white/15 cursor-pointer hover:opacity-80 transition-opacity"
              title="Click to open Home"
            >
              <span
                className="font-bold tracking-tight leading-none orbit-clock-3d text-[12px]"
                style={{ color: "var(--island-text)" }}
              >
                {timeDisplay.timeStr}
              </span>
              {timeDisplay.ampm && (
                <span
                  className="text-[8px] font-bold leading-none uppercase tracking-wider"
                  style={{
                    color: activeThemeMeta.color,
                    textShadow: `0 0 6px ${activeThemeMeta.borderGlow}`,
                  }}
                >
                  {timeDisplay.ampm}
                </span>
              )}
            </div>

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

          <div className="flex flex-col min-w-0 flex-1 overflow-hidden leading-tight text-left pr-1 max-w-[96px]">
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

          {/* 3D Glowing Frequency-Reactive Equalizer Bars */}
          <div className="flex items-center space-x-[2px] h-4 shrink-0 pr-1">
            {[1, 2, 3, 4].map((idx) => {
              const isPlaying = mediaSession.playback_status === "playing";
              const barVal = isPlaying ? (audioSpectrum?.compactBars?.[idx - 1] ?? 0) : 0;
              const hasFreq = isPlaying && (audioSpectrum?.peak ?? 0) >= 0.005 && barVal > 0.005;
              const scaleY = hasFreq
                ? Math.max(0.18, Math.min(1, 0.12 + barVal * 0.88))
                : 0.02;
              return (
                <div
                  key={idx}
                  className="eq-bar"
                  style={{
                    backgroundColor: activeThemeMeta.color,
                    transform: `scaleY(${scaleY.toFixed(2)})`,
                    transition: "transform 95ms ease-out",
                    opacity: hasFreq ? 0.95 : 0.0,
                  }}
                />
              );
            })}
          </div>

          {privacyLight && (
            <div
              className="flex items-center justify-center shrink-0 pl-1.5 border-l border-white/15"
              title="Camera/Microphone Active"
            >
              {privacyLight}
            </div>
          )}
        </div>
      </div>
    ) : (
      /* Compact 3D Clock Pill with Right Slot & Live Upload/Download Activity */
      <div className="flex items-center justify-center w-full h-full relative select-none px-3">
        {/* If BOTH Upload AND Download are active simultaneously: Full Pill Flip */}
        {isDownloading && isUploading ? (
          <div
            ref={compactContentRef}
            className="inline-flex items-center justify-center space-x-2 text-[10.5px] font-bold tabular-nums shrink-0"
          >
            {hardwareMetrics?.screenRecordingActive && (
              <div
                className="flex items-center space-x-1 shrink-0 pr-1 border-r border-white/15"
                title="Screen Recording active"
              >
                <span
                  className="w-2 h-2 rounded-full bg-rose-500 animate-pulse"
                  style={{ boxShadow: "0 0 6px #f43f5e" }}
                />
              </div>
            )}
            <div
              className="flex items-center space-x-1 text-emerald-400"
              title="Download speed"
            >
              <span className="animate-bounce text-[9.5px]">⬇</span>
              <span>
                {formatTransferByteSpeed(
                  hardwareMetrics?.networkDownloadSpeedBps ?? 0
                )}
              </span>
            </div>
            <div className="w-[1px] h-3 bg-white/20 shrink-0" />
            <div
              className="flex items-center space-x-1 text-sky-400"
              title="Upload speed"
            >
              <span className="animate-bounce text-[9.5px]">⬆</span>
              <span>
                {formatTransferByteSpeed(
                  hardwareMetrics?.networkUploadSpeedBps ?? 0
                )}
              </span>
            </div>
            {privacyLight && (
              <div
                className="flex items-center justify-center shrink-0 pl-1.5 border-l border-white/15"
                title="Camera/Microphone Active"
              >
                {privacyLight}
              </div>
            )}
          </div>
        ) : (
          /* Standard Compact Clock + Right Slot (or Single Transfer Speed Flip) */
          <div
            ref={compactContentRef}
            className="inline-flex items-center space-x-2 tabular-nums min-w-0 shrink-0"
          >
            {/* Screen Recording Active Dot */}
            {hardwareMetrics?.screenRecordingActive && (
              <div
                className="flex items-center space-x-1 shrink-0 pr-1.5 border-r border-white/15"
                title="Screen Recording active"
              >
                <span
                  className="w-2 h-2 rounded-full bg-rose-500 animate-pulse"
                  style={{ boxShadow: "0 0 6px #f43f5e" }}
                />
              </div>
            )}

            {/* Clock on Left */}
            <div className="flex items-baseline space-x-1 shrink-0">
              <span
                className="font-bold tracking-tight leading-none orbit-clock-3d text-[13.5px]"
                style={{ color: "var(--island-text)" }}
              >
                {timeDisplay.timeStr}
              </span>
              {timeDisplay.ampm && (
                <span
                  className="text-[9px] font-bold leading-none uppercase tracking-wider"
                  style={{
                    color: activeThemeMeta.color,
                    textShadow: `0 0 8px ${activeThemeMeta.borderGlow}`,
                  }}
                >
                  {timeDisplay.ampm}
                </span>
              )}
            </div>

            {/* Single Transfer Active (Download XOR Upload active): Flip only Right Slot */}
            {isDownloading || isUploading ? (
              <div className="flex items-center space-x-1 text-[10px] font-bold tabular-nums border-l border-white/15 pl-1.5 shrink-0">
                {isDownloading ? (
                  <div
                    className="flex items-center space-x-1 text-emerald-400"
                    title="Downloading"
                  >
                    <span className="animate-pulse">⬇</span>
                    <span>
                      {formatTransferByteSpeed(
                        hardwareMetrics?.networkDownloadSpeedBps ?? 0
                      )}
                    </span>
                  </div>
                ) : (
                  <div
                    className="flex items-center space-x-1 text-sky-400"
                    title="Uploading"
                  >
                    <span className="animate-pulse">⬆</span>
                    <span>
                      {formatTransferByteSpeed(
                        hardwareMetrics?.networkUploadSpeedBps ?? 0
                      )}
                    </span>
                  </div>
                )}
              </div>
            ) : (
              /* Otherwise: Customizable Compact Right Slot */
              renderCompactRightSlot()
            )}

            {/* Privacy Light Indicator */}
            {privacyLight && (
              <div
                className="flex items-center justify-center shrink-0 pl-1.5 border-l border-white/15"
                title="Camera/Microphone Active"
              >
                {privacyLight}
              </div>
            )}
          </div>
        )}
      </div>
      )}
    </div>
  );
};
