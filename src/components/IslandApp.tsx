import React, { useEffect, useState, useRef, useMemo } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc, setCompactWidthHint } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";
import { AppConfig } from "../core/types/config";
import { getThemeMeta } from "../core/theme";
import { MediaSessionSnapshot, AudioSpectrumState } from "../core/types/media";
import {
  NotificationItem,
  NotificationSnapshot,
  PrivacyState,
} from "../core/types/notifications";
import { ReminderItem } from "../core/types/reminders";
import { HardwareMetrics } from "../core/types/hardware";
import { ShelfItem } from "../core/types/shelf";
import { CompactCapsule, SystemUtilityAlert, WeatherGlance } from "./island/CompactCapsule";
import { OnboardingOverlay } from "./island/OnboardingOverlay";
import { HomeSection } from "./island/HomeSection";
import { CalendarSection } from "./island/CalendarSection";
import { HardwareSection } from "./island/HardwareSection";
import { MultiTaskSection } from "./island/MultiTaskSection";

const formatSeconds = (totalSecs: number) => {
  if (isNaN(totalSecs) || totalSecs < 0) return "0:00";
  const mins = Math.floor(totalSecs / 60);
  const secs = Math.floor(totalSecs % 60);
  return `${mins}:${String(secs).padStart(2, "0")}`;
};

const formatNetworkSpeed = (bps: number): string => {
  const mbps = (bps * 8) / 1_000_000;
  if (mbps >= 0.1) return `${mbps.toFixed(1)} Mbps`;
  if (bps < 1024) return `0.0 Mbps`;
  return `${(bps / 1024).toFixed(0)} KB/s`;
};

export function estimateCompactWidth({
  config,
  weatherInfo,
  timeDisplay,
  isMediaActive,
  hasMediaSession,
  timerRemainingSecs,
  utilityAlert,
  callDurationSecs,
  hardwareMetrics,
  hasPrivacyLight,
  hasBothPrivacyLights = false,
}: {
  config: AppConfig;
  weatherInfo: WeatherGlance | null;
  timeDisplay: { timeStr: string; ampm: string };
  isMediaActive: boolean;
  hasMediaSession: boolean;
  timerRemainingSecs: number;
  utilityAlert: SystemUtilityAlert | null;
  callDurationSecs: number | null;
  hardwareMetrics: HardwareMetrics | null;
  hasPrivacyLight: boolean;
  hasBothPrivacyLights?: boolean;
}): number {
  const isRec = Boolean(hardwareMetrics?.screenRecordingActive);
  const privacyWidth = hasBothPrivacyLights ? 36 : hasPrivacyLight ? 18 : 0;
  const indicatorExtra = privacyWidth + (isRec ? 14 : 0);

  if (utilityAlert) {
    if (utilityAlert.kind === "timer_done") return 316;
    return 120 + indicatorExtra;
  }

  const isDual = timerRemainingSecs > 0 && isMediaActive && hasMediaSession;
  if (isDual) {
    return 242 + indicatorExtra;
  }

  if (timerRemainingSecs > 0) {
    return 132 + indicatorExtra;
  }

  // Active call: show duration on right slot → wider pill
  if (callDurationSecs !== null && callDurationSecs >= 0) {
    return 148 + indicatorExtra;
  }

  if (isMediaActive && hasMediaSession) {
    return (timeDisplay.ampm ? 186 : 172) + indicatorExtra;
  }

  const isDownloading =
    config.networkActivityMode !== "off" &&
    (hardwareMetrics?.networkDownloadSpeedBps ?? 0) >= 250 * 1024;
  const isUploading =
    config.networkActivityMode !== "off" &&
    (hardwareMetrics?.networkUploadSpeedBps ?? 0) >= 150 * 1024;

  if (isDownloading && isUploading) {
    return hasPrivacyLight ? 178 + (hasBothPrivacyLights ? 18 : 0) : 164;
  }

  // Base clock width: 12h clock with AM/PM ~ 56px, 24h clock ~ 42px
  let contentWidth = timeDisplay.ampm ? 56 : 42;

  // Add right slot
  if (isDownloading || isUploading) {
    contentWidth += 54;
  } else {
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
      case "weather":
        if (
          (config.showWeather ||
            config.compactRightSlot === "weather" ||
            config.compactRightSlot === "smart") &&
          weatherInfo
        ) {
          contentWidth += 38;
        }
        break;
      case "orbit_logo":
      case "windows_logo":
      case "apple_logo":
      case "linux_logo":
      case "custom_image":
        contentWidth += 24;
        break;
      case "custom_text": {
        const textLen = Math.min(5, (config.customRightText?.trim().length) || 4);
        contentWidth += textLen * 7.5 + 8;
        break;
      }
      default:
        break;
    }
  }

  let totalPadding = 26; // 13px left + 13px right
  if (hardwareMetrics?.screenRecordingActive) {
    totalPadding += 16;
  }
  if (hasPrivacyLight) {
    totalPadding += hasBothPrivacyLights ? 38 : 20;
  }

  return Math.max(82, Math.min(240, Math.round(contentWidth + totalPadding)));
}

export const IslandApp: React.FC = () => {
  const { config, loadConfig } = useConfigStore();

  const islandRef = useRef<HTMLDivElement>(null);
  const [isExpanded, setIsExpanded] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
  const [measuredCompactWidth, setMeasuredCompactWidth] = useState<number | null>(null);
  const [activeSection, setActiveSection] = useState(0);
  const [currentTime, setCurrentTime] = useState(new Date());
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false);

  // Notification and Privacy state
  const [notificationSnapshot, setNotificationSnapshot] =
    useState<NotificationSnapshot | null>(null);
  const [privacyState, setPrivacyState] = useState<PrivacyState | null>(null);
  const [transientNotif, setTransientNotif] = useState<NotificationItem | null>(
    null
  );
  const [isNotifHovered, setIsNotifHovered] = useState(false);
  const [showHomeNotifications, setShowHomeNotifications] = useState(false);
  const [selectedNotifId, setSelectedNotifId] = useState<number | null>(null);
  const [isNotifEntering, setIsNotifEntering] = useState(false);
  const notifEntranceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
    null
  );
  const notifQueueRef = useRef<NotificationItem[]>([]);
  const previewTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const seenNotifIdsRef = useRef<Set<number>>(new Set());

  // Media session state
  const [mediaSession, setMediaSession] = useState<MediaSessionSnapshot | null>(
    null
  );
  const [audioSpectrum, setAudioSpectrum] = useState<AudioSpectrumState | null>(
    null
  );
  const [pinnedSource, setPinnedSource] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragPos, setDragPos] = useState(0);
  const isDraggingRef = useRef(false);
  const dragPosRef = useRef(0);
  const [pendingSeekPos, setPendingSeekPos] = useState<number | null>(null);
  const pendingSeekPosRef = useRef<number | null>(null);
  const seekTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [isMediaCommandPending, setIsMediaCommandPending] = useState(false);
  const isMediaCommandPendingRef = useRef(false);
  const [, setLocalTick] = useState(0);
  const lastServerPosRef = useRef(0);
  const lastServerTsRef = useRef(0);

  // Reminders & Calendar state
  const [reminders, setReminders] = useState<ReminderItem[]>([]);
  const [calendarMonth, setCalendarMonth] = useState<Date>(new Date());
  const [selectedCalendarDate, setSelectedCalendarDate] = useState<Date>(
    new Date()
  );
  const [newReminderTitle, setNewReminderTitle] = useState("");
  const [newReminderCategory] = useState("task");
  const notifiedReminderIdsRef = useRef<Set<string>>(new Set());

  const todayNextEvent = useMemo(() => {
    const todayStr = currentTime.toDateString();
    const todayEvents = reminders.filter(
      (r) => !r.isCompleted && new Date(r.dueTimestampMs).toDateString() === todayStr
    );
    if (todayEvents.length === 0) return null;
    todayEvents.sort((a, b) => a.dueTimestampMs - b.dueTimestampMs);
    return todayEvents[0];
  }, [reminders, currentTime]);

  // Hardware monitor state
  const [hardwareMetrics, setHardwareMetrics] =
    useState<HardwareMetrics | null>(null);

  // Call duration tracking (Sub-Phase 9.3)
  const callStartMsRef = useRef<number | null>(null);
  const [callDurationSecs, setCallDurationSecs] = useState<number | null>(null);
  const pointerInsideRef = useRef(false);
  const privacyTestUntilRef = useRef(0);
  const nativeStateRef = useRef<
    "compact" | "compact_dual" | "notification" | "notification_hover" | "expanded" | "suppressed"
  >("compact");

  // 2-second compact utility alert state
  const [utilityAlert, setUtilityAlert] = useState<SystemUtilityAlert | null>(null);
  const utilityTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevUtilityRef = useRef<{
    isCharging: boolean;
    wifiConnected: boolean;
    bluetoothEnabled: boolean;
    hotspotActive: boolean;
  } | null>(null);
  const prevDownloadRef = useRef<string | null>(null);

  const playTimerCompleteChime = () => {
    try {
      const AudioCtx =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
      if (!AudioCtx) return;
      const ctx = new AudioCtx();
      const now = ctx.currentTime;
      const notes = [587.33, 880.0]; // D5 -> A5 pleasant bell fifth
      notes.forEach((freq, idx) => {
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.type = "sine";
        osc.frequency.setValueAtTime(freq, now + idx * 0.16);
        gain.gain.setValueAtTime(0.001, now + idx * 0.16);
        gain.gain.exponentialRampToValueAtTime(0.18, now + idx * 0.16 + 0.03);
        gain.gain.exponentialRampToValueAtTime(0.0001, now + idx * 0.16 + 0.55);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start(now + idx * 0.16);
        osc.stop(now + idx * 0.16 + 0.6);
      });
    } catch {
      // Ignore audio errors
    }
  };

  const dismissUtilityAlert = () => {
    if (utilityTimerRef.current) {
      clearTimeout(utilityTimerRef.current);
      utilityTimerRef.current = null;
    }
    utilityAlertRef.current = null;
    setUtilityAlert(null);
    if (!isExpandedRef.current && !transientNotifRef.current) {
      setTimeout(() => {
        if (
          !isExpandedRef.current &&
          !transientNotifRef.current &&
          !utilityAlertRef.current
        ) {
          const targetState = isDualCompactRef.current ? "compact_dual" : "compact";
          nativeStateRef.current = targetState;
          ipc.setIslandState(targetState).catch(() => {});
        }
      }, 280);
    }
  };

  const triggerUtilityAlert = (alert: SystemUtilityAlert) => {
    const isTimerDone = alert.kind === "timer_done";
    if (!isTimerDone && isFullscreenSuppressedRef.current) return;
    utilityAlertRef.current = alert;
    setUtilityAlert(alert);
    if (isTimerDone) {
      playTimerCompleteChime();
    }
    if (!isExpandedRef.current && !transientNotifRef.current) {
      const stateForAlert = isTimerDone ? "notification" : "compact_alert";
      nativeStateRef.current = stateForAlert === "notification" ? "notification" : "compact";
      ipc.setIslandState(stateForAlert).catch(() => {});
    } else if (isTimerDone && isExpandedRef.current) {
      // Re-assert topmost even when already expanded and alwaysOnTop is OFF
      ipc.setIslandState("expanded").catch(() => {});
    }
    if (utilityTimerRef.current) {
      clearTimeout(utilityTimerRef.current);
    }
    utilityTimerRef.current = setTimeout(
      () => {
        dismissUtilityAlert();
      },
      isTimerDone ? 6000 : 2000
    );
  };
  const triggerUtilityAlertRef = useRef(triggerUtilityAlert);
  triggerUtilityAlertRef.current = triggerUtilityAlert;
  const utilityAlertRef = useRef<SystemUtilityAlert | null>(null);
  utilityAlertRef.current = utilityAlert;

  // Media active calculation
  const isMediaActive = Boolean(
    mediaSession &&
      (mediaSession.playback_status === "playing" ||
        (mediaSession.playback_status === "paused" &&
          mediaSession.track.title.trim().length > 0))
  );

  // Weather state (stale-while-revalidate cache + instant fallback so right slot never flickers blank)
  const [weatherInfo, setWeatherInfo] = useState<WeatherGlance | null>(() => {
    try {
      const cached = localStorage.getItem("orbit_weather_cache");
      if (cached) {
        const parsed = JSON.parse(cached);
        if (parsed.data && typeof parsed.data.tempC === "number") {
          return parsed.data;
        }
      }
    } catch {}
    return {
      tempC: 24,
      tempF: 75,
      conditionCode: "partly_cloudy",
      label: "Partly Cloudy",
    };
  });

  const shouldFetchWeather =
    config.showWeather ||
    config.compactRightSlot === "weather" ||
    config.compactRightSlot === "smart";

  useEffect(() => {
    if (!shouldFetchWeather) return;

    let isMounted = true;
    let activeController: AbortController | null = null;

    const fetchWeather = async () => {
      if (activeController) {
        activeController.abort();
      }
      const controller = new AbortController();
      activeController = controller;
      const timeoutId = setTimeout(() => controller.abort(), 6500);

      try {
        const res = await fetch("https://wttr.in/?format=j1", {
          signal: controller.signal,
        });
        if (!res.ok) throw new Error("HTTP error " + res.status);
        const data = await res.json();
        const current = data?.current_condition?.[0];
        if (!current) throw new Error("No current condition");

        const tempC = parseInt(current.temp_C, 10) || 24;
        const tempF = parseInt(current.temp_F, 10) || Math.round((tempC * 9) / 5 + 32);
        const code = parseInt(current.weatherCode, 10) || 113;

        let conditionCode: WeatherGlance["conditionCode"] = "clear";
        if ([200, 386, 389, 392, 395].includes(code)) {
          conditionCode = "storm";
        } else if ([176, 293, 296, 299, 302, 305, 308, 311, 353, 356, 359].includes(code)) {
          conditionCode = "rain";
        } else if ([179, 182, 227, 230, 323, 326, 329, 332, 335, 338, 368, 371].includes(code)) {
          conditionCode = "snow";
        } else if ([116, 119, 122, 143, 248].includes(code)) {
          conditionCode = code === 116 ? "partly_cloudy" : "cloudy";
        }

        const label = current.weatherDesc?.[0]?.value || "Clear";
        const glance: WeatherGlance = { tempC, tempF, conditionCode, label };

        if (isMounted) {
          setWeatherInfo(glance);
          localStorage.setItem(
            "orbit_weather_cache",
            JSON.stringify({ data: glance, timestamp: Date.now() })
          );
        }
      } catch {
        // Keep existing cached/fallback weatherInfo so right slot never disappears
      } finally {
        clearTimeout(timeoutId);
      }
    };

    fetchWeather();
    const interval = setInterval(fetchWeather, 1800000);

    return () => {
      isMounted = false;
      if (activeController) {
        activeController.abort();
      }
      clearInterval(interval);
    };
  }, [shouldFetchWeather]);

  // Enhanced Timer state (persisted in localStorage per PRD Section 5.3)
  const [timerDeadlineMs, setTimerDeadlineMs] = useState<number | null>(() => {
    try {
      const saved = localStorage.getItem("orbit_timer_state");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.deadlineMs && parsed.deadlineMs > Date.now()) {
          return parsed.deadlineMs;
        }
      }
    } catch {}
    return null;
  });
  const [timerPausedRemainingSecs, setTimerPausedRemainingSecs] = useState<number | null>(() => {
    try {
      const saved = localStorage.getItem("orbit_timer_state");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (typeof parsed.pausedRemainingSecs === "number" && parsed.pausedRemainingSecs > 0) {
          return parsed.pausedRemainingSecs;
        }
      }
    } catch {}
    return null;
  });
  const [timerTotalSecs, setTimerTotalSecs] = useState<number>(() => {
    try {
      const saved = localStorage.getItem("orbit_timer_state");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.totalSecs) return parsed.totalSecs;
      }
    } catch {}
    return 0;
  });
  const [timerLabel, setTimerLabel] = useState<string>(() => {
    try {
      const saved = localStorage.getItem("orbit_timer_state");
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed.label) return parsed.label;
      }
    } catch {}
    return "Focus";
  });

  const isTimerPaused = timerPausedRemainingSecs !== null && timerPausedRemainingSecs > 0;
  const timerRemainingSecs = isTimerPaused
    ? (timerPausedRemainingSecs || 0)
    : timerDeadlineMs
    ? Math.max(0, Math.ceil((timerDeadlineMs - currentTime.getTime()) / 1000))
    : 0;

  const startTimer = (durationSecs: number, label = timerLabel) => {
    const deadline = Date.now() + durationSecs * 1000;
    setTimerDeadlineMs(deadline);
    setTimerTotalSecs(durationSecs);
    setTimerPausedRemainingSecs(null);
    setTimerLabel(label);
    localStorage.setItem(
      "orbit_timer_state",
      JSON.stringify({ deadlineMs: deadline, totalSecs: durationSecs, label })
    );
  };

  const pauseTimer = () => {
    if (timerDeadlineMs && !isTimerPaused) {
      const rem = Math.max(0, Math.ceil((timerDeadlineMs - Date.now()) / 1000));
      setTimerPausedRemainingSecs(rem);
      setTimerDeadlineMs(null);
      localStorage.setItem(
        "orbit_timer_state",
        JSON.stringify({ pausedRemainingSecs: rem, totalSecs: timerTotalSecs, label: timerLabel })
      );
    }
  };

  const resumeTimer = () => {
    if (timerPausedRemainingSecs && timerPausedRemainingSecs > 0) {
      const deadline = Date.now() + timerPausedRemainingSecs * 1000;
      setTimerDeadlineMs(deadline);
      setTimerPausedRemainingSecs(null);
      localStorage.setItem(
        "orbit_timer_state",
        JSON.stringify({ deadlineMs: deadline, totalSecs: timerTotalSecs, label: timerLabel })
      );
    }
  };

  const togglePauseTimer = () => {
    if (isTimerPaused) {
      resumeTimer();
    } else if (timerDeadlineMs) {
      pauseTimer();
    }
  };

  const resetTimer = () => {
    if (timerTotalSecs > 0) {
      startTimer(timerTotalSecs, timerLabel);
    }
  };

  const stopTimer = () => {
    setTimerDeadlineMs(null);
    setTimerPausedRemainingSecs(null);
    setTimerTotalSecs(0);
    localStorage.removeItem("orbit_timer_state");
  };

  // Timer completion notification
  useEffect(() => {
    if (timerDeadlineMs !== null && currentTime.getTime() >= timerDeadlineMs) {
      stopTimer();
      triggerUtilityAlertRef.current({
        kind: "timer_done",
        label: `${timerLabel} Timer`,
        detail: "Time's Up!",
        color: "#f59e0b",
      });
    }
  }, [currentTime, timerDeadlineMs, timerLabel]);

  // Upcoming Event / Exam / Meeting countdown alert (within 15 minutes)
  useEffect(() => {
    const now = Date.now();
    const alertWindowMs = 15 * 60 * 1000;
    for (const r of reminders) {
      if (r.isCompleted) continue;
      const timeDiff = r.dueTimestampMs - now;
      if (timeDiff > 0 && timeDiff <= alertWindowMs && !notifiedReminderIdsRef.current.has(r.id)) {
        notifiedReminderIdsRef.current.add(r.id);
        const minsLeft = Math.max(1, Math.round(timeDiff / 60000));
        const cat = (r.category || "event").toLowerCase();
        let catColor = "#38bdf8";
        let catLabel = "Upcoming Event";
        if (cat === "exam" || cat === "deadline") {
          catColor = "#fb7185";
          catLabel = cat === "exam" ? "Exam Starting Soon" : "Deadline Approaching";
        } else if (cat === "meeting") {
          catColor = "#fbbf24";
          catLabel = "Meeting Starting Soon";
        } else if (cat === "class") {
          catColor = "#38bdf8";
          catLabel = "Class Starting Soon";
        } else if (cat === "holiday") {
          catColor = "#34d399";
          catLabel = "Holiday Today";
        }
        triggerUtilityAlertRef.current({
          kind: "event_due",
          label: `${catLabel} (${minsLeft}m)`,
          detail: r.title,
          color: catColor,
        });
        break;
      }
    }
  }, [reminders, currentTime]);

  // Dual compact mode (Simultaneous Timer + Media)
  const isDualCompact =
    !isExpanded && !transientNotif && !utilityAlert && timerRemainingSecs > 0 && isMediaActive;

  const isDualCompactRef = useRef(false);
  isDualCompactRef.current = isDualCompact;

  // Single source of truth for the compact pill width: drives both the CSS width and the
  // native hit region, so the visible capsule and the clickable area can never disagree.
  const compactPillWidth =
    measuredCompactWidth !== null
      ? isDualCompact
        ? Math.max(measuredCompactWidth, 242)
        : measuredCompactWidth
      : isDualCompact
      ? 256
      : estimateCompactWidth({
          config,
          weatherInfo,
          timeDisplay: {
            timeStr: "",
            ampm: config.timeFormat === "12h" ? "PM" : "",
          },
          isMediaActive,
          hasMediaSession: Boolean(mediaSession),
          timerRemainingSecs,
          utilityAlert,
          callDurationSecs,
          hardwareMetrics,
          hasPrivacyLight: Boolean(
            privacyState?.cameraActive || privacyState?.microphoneActive
          ),
          hasBothPrivacyLights: Boolean(
            privacyState?.cameraActive && privacyState?.microphoneActive
          ),
        });

  // Keep the native region glued to the pill. Grow immediately (so the growing capsule is never
  // clipped); shrink after the CSS width transition so the shrinking capsule isn't cut mid-animation.
  const lastRegionWidthRef = useRef(0);
  useEffect(() => {
    setCompactWidthHint(compactPillWidth);
    const ns = nativeStateRef.current;
    if (ns !== "compact" && ns !== "compact_dual") return;
    if (isExpanded || transientNotif || utilityAlert?.kind === "timer_done") return;

    const apply = () => {
      lastRegionWidthRef.current = compactPillWidth;
      ipc.updateIslandRegion(ns, compactPillWidth).catch(() => {});
    };
    if (compactPillWidth >= lastRegionWidthRef.current) {
      apply();
      return;
    }
    const t = setTimeout(apply, 340);
    return () => clearTimeout(t);
  }, [compactPillWidth, isExpanded, transientNotif, utilityAlert?.kind]);

  const [isPinnedExpanded, setIsPinnedExpanded] = useState(false);
  const isPinnedExpandedRef = useRef(false);
  isPinnedExpandedRef.current = isPinnedExpanded;

  const [albumAccentColor, setAlbumAccentColor] = useState<string | null>(null);

  const [systemIsLight, setSystemIsLight] = useState(() => {
    if (typeof window !== "undefined" && window.matchMedia) {
      return window.matchMedia("(prefers-color-scheme: light)").matches;
    }
    return false;
  });

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const handler = (e: MediaQueryListEvent) => setSystemIsLight(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  // Adaptive Album Art Accent Extraction (Sub-Phase 6.1)
  useEffect(() => {
    if (!config.adaptiveAlbumAccent || !mediaSession?.track.thumbnail_base64) {
      setAlbumAccentColor(null);
      return;
    }

    let isMounted = true;
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      if (!isMounted) return;
      try {
        const canvas = document.createElement("canvas");
        canvas.width = 16;
        canvas.height = 16;
        const ctx = canvas.getContext("2d", { willReadFrequently: true });
        if (!ctx) return;
        ctx.drawImage(img, 0, 0, 16, 16);
        const imgData = ctx.getImageData(0, 0, 16, 16).data;

        let maxSat = 0;
        let bestR = 56,
          bestG = 189,
          bestB = 248;
        let rSum = 0,
          gSum = 0,
          bSum = 0,
          count = 0;

        for (let i = 0; i < imgData.length; i += 4) {
          const r = imgData[i];
          const g = imgData[i + 1];
          const b = imgData[i + 2];
          const max = Math.max(r, g, b);
          const min = Math.min(r, g, b);
          const lum = (max + min) / 2;
          if (lum > 25 && lum < 235) {
            const delta = max - min;
            const sat = delta / (1 - Math.abs((2 * lum) / 255 - 1));
            if (sat > maxSat) {
              maxSat = sat;
              bestR = r;
              bestG = g;
              bestB = b;
            }
            rSum += r;
            gSum += g;
            bSum += b;
            count++;
          }
        }

        if (maxSat > 0.15) {
          setAlbumAccentColor(`rgb(${bestR}, ${bestG}, ${bestB})`);
        } else if (count > 0) {
          setAlbumAccentColor(
            `rgb(${Math.round(rSum / count)}, ${Math.round(gSum / count)}, ${Math.round(bSum / count)})`
          );
        } else {
          setAlbumAccentColor(null);
        }
      } catch {
        setAlbumAccentColor(null);
      }
    };
    img.onerror = () => {
      if (isMounted) setAlbumAccentColor(null);
    };
    img.src = mediaSession.track.thumbnail_base64;

    return () => {
      isMounted = false;
    };
  }, [config.adaptiveAlbumAccent, mediaSession?.track.thumbnail_base64]);

  // Meeting Auto-Suppression (Sub-Phase 9.3)
  const isMeetingSuppressed = Boolean(
    config.autoHideInMeetings &&
      privacyState?.cameraActive &&
      privacyState?.microphoneActive &&
      !config.showPrivacyIndicators &&
      !isExpanded &&
      !transientNotif &&
      timerRemainingSecs <= 0 &&
      !utilityAlert &&
      !pointerInsideRef.current
  );

  useEffect(() => {
    if (isMeetingSuppressed) {
      if (nativeStateRef.current !== "suppressed") {
        nativeStateRef.current = "suppressed";
        ipc.setIslandState("suppressed").catch(() => {});
      }
    } else if (nativeStateRef.current === "suppressed") {
      const targetState = isDualCompactRef.current ? "compact_dual" : "compact";
      nativeStateRef.current = targetState;
      ipc.setIslandState(targetState).catch(() => {});
    }
  }, [isMeetingSuppressed]);

  // Call duration counter (Sub-Phase 9.3)
  useEffect(() => {
    const isCallActive = Boolean(
      privacyState && (privacyState.microphoneActive || privacyState.cameraActive)
    );
    if (isCallActive) {
      if (callStartMsRef.current === null) {
        callStartMsRef.current = Date.now();
      }
      setCallDurationSecs(
        Math.max(1, Math.floor((Date.now() - callStartMsRef.current) / 1000))
      );
      const timer = setInterval(() => {
        if (callStartMsRef.current !== null) {
          setCallDurationSecs(
            Math.max(1, Math.floor((Date.now() - callStartMsRef.current) / 1000))
          );
        }
      }, 1000);
      return () => clearInterval(timer);
    } else {
      callStartMsRef.current = null;
      setCallDurationSecs(null);
    }
  }, [privacyState?.microphoneActive, privacyState?.cameraActive]);

  // Fix 1: Only change native state when already in compact mode, NEVER while collapsing from expanded!
  useEffect(() => {
    if (
      !isExpanded &&
      !transientNotif &&
      !utilityAlert &&
      !isMeetingSuppressed &&
      (nativeStateRef.current === "compact" || nativeStateRef.current === "compact_dual")
    ) {
      const targetState = isDualCompact ? "compact_dual" : "compact";
      if (nativeStateRef.current !== targetState) {
        nativeStateRef.current = targetState;
        ipc.setIslandState(targetState).catch(() => {});
      }
    }
  }, [isDualCompact, isExpanded, transientNotif, utilityAlert, isMeetingSuppressed]);

  // File Shelf state
  const [shelfItems, setShelfItems] = useState<ShelfItem[]>([]);
  const [showFileShelf, setShowFileShelf] = useState(false);

  // Dynamic Section 2 (Option A: Multi-Task Hub appears at Dot 2 when 2+ activities are active)
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

  // Onboarding Guided Tour state (Sub-Phase 8.1)
  const [showOnboarding, setShowOnboarding] = useState<boolean>(
    () => {
      try {
        if (localStorage.getItem("orbit_onboarding_seen") === "true") return false;
      } catch (_) {}
      return false;
    }
  );
  const showOnboardingRef = useRef<boolean>(showOnboarding);
  showOnboardingRef.current = showOnboarding;

  // Automatically expand the island whenever the Onboarding Guide is active so first-time users see it immediately
  useEffect(() => {
    if (showOnboarding) {
      setIsClosing(false);
      setIsExpanded(true);
      if (nativeStateRef.current !== "expanded") {
        nativeStateRef.current = "expanded";
        ipc.setIslandState("expanded").catch(() => {});
      }
    }
  }, [showOnboarding]);

  // Sync onboarding state once config is loaded from disk
  useEffect(() => {
    try {
      const seenLocally = localStorage.getItem("orbit_onboarding_seen") === "true";
      if (config.onboardingCompleted || seenLocally) {
        if (showOnboarding) {
          setShowOnboarding(false);
          showOnboardingRef.current = false;
        }
        if (!seenLocally && config.onboardingCompleted) {
          localStorage.setItem("orbit_onboarding_seen", "true");
        }
      } else if (!config.onboardingCompleted && !seenLocally) {
        setShowOnboarding(true);
        showOnboardingRef.current = true;
      }
    } catch (_) {}
  }, [config.onboardingCompleted]);

  // Reset calendar month and selected date to today whenever the island compacts
  useEffect(() => {
    if (!isExpanded) {
      const now = new Date();
      setCalendarMonth(now);
      setSelectedCalendarDate(now);
    }
  }, [isExpanded]);

  const handleCompleteOnboarding = async (openSettings = false) => {
    try {
      localStorage.setItem("orbit_onboarding_seen", "true");
      showOnboardingRef.current = false;
      setShowOnboarding(false);
      await useConfigStore.getState().updateConfig({ onboardingCompleted: true });
      if (openSettings) {
        await ipc.openSettingsWindow();
      }
    } catch (err) {
      console.error("Failed to mark onboarding completed:", err);
    }
  };

  // Live refs to avoid stale closures in long-lived Tauri event listeners
  const configRef = useRef<AppConfig>(config);
  configRef.current = config;
  const isExpandedRef = useRef<boolean>(isExpanded);
  isExpandedRef.current = isExpanded;
  const transientNotifRef = useRef<NotificationItem | null>(transientNotif);
  transientNotifRef.current = transientNotif;
  const isNotifHoveredRef = useRef<boolean>(isNotifHovered);
  isNotifHoveredRef.current = isNotifHovered;
  const notifHoverGraceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
    null
  );

  // Transition tracking and coordination refs
  const transitionIdRef = useRef<number>(0);
  const contextMenuOpenRef = useRef(false);
  const collapseGraceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(
    null
  );
  const watchdogTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hideTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastScrollTimeRef = useRef<number>(0);
  const lastPlayingTrackRef = useRef<string>("");
  const autoExpandTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isFullscreenSuppressedRef = useRef<boolean>(false);

  const reset3DTilt = () => {
    // Cursor-move 3D tilt removed to keep small island text razor-sharp at all times
  };

  // Playback engine for sequential notification previews in transient intermediate capsule
  const playNextNotification = () => {
    if (previewTimerRef.current) {
      clearTimeout(previewTimerRef.current);
      previewTimerRef.current = null;
    }

    if (isFullscreenSuppressedRef.current) {
      notifQueueRef.current = [];
      setTransientNotif(null);
      return;
    }

    const next = notifQueueRef.current.shift();
    if (next) {
      transientNotifRef.current = next;
      setTransientNotif(next);
      setIsNotifEntering(true);
      if (notifEntranceTimerRef.current)
        clearTimeout(notifEntranceTimerRef.current);
      notifEntranceTimerRef.current = setTimeout(() => {
        setIsNotifEntering(false);
      }, 35);

      const staysHovered = pointerInsideRef.current && !isExpandedRef.current;
      isNotifHoveredRef.current = staysHovered;
      setIsNotifHovered(staysHovered);

      if (!isExpandedRef.current) {
        const targetRegion = staysHovered ? "notification_hover" : "notification";
        if (nativeStateRef.current !== targetRegion) {
          nativeStateRef.current = targetRegion;
          ipc.setIslandState(targetRegion).catch((e) => console.error(e));
        }
      }
      if (!staysHovered) {
        const dur = (configRef.current.notificationPreviewDurationSec || 4) * 1000;
        previewTimerRef.current = setTimeout(() => {
          playNextNotificationRef.current();
        }, dur);
      }
    } else {
      transientNotifRef.current = null;
      setTransientNotif(null);
      isNotifHoveredRef.current = false;
      setIsNotifHovered(false);
      setIsNotifEntering(false);
      if (!isExpandedRef.current) {
        pointerInsideRef.current = false;
        const target = isDualCompactRef.current ? "compact_dual" : "compact";
        if (nativeStateRef.current !== target) {
          nativeStateRef.current = target;
          setTimeout(() => {
            if (
              !isExpandedRef.current &&
              !transientNotifRef.current &&
              !utilityAlertRef.current
            ) {
              const finalTarget = isDualCompactRef.current ? "compact_dual" : "compact";
              ipc.setIslandState(finalTarget).catch((e) => console.error(e));
            }
          }, 280);
        }
      }
    }
  };

  const playNextNotificationRef = useRef(playNextNotification);
  playNextNotificationRef.current = playNextNotification;

  useEffect(() => {
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    setPrefersReducedMotion(mediaQuery.matches);
    const handler = (event: MediaQueryListEvent) =>
      setPrefersReducedMotion(event.matches);
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, []);

  // Smooth local timeline progression ticker for playing video and audio sessions
  useEffect(() => {
    if (mediaSession?.playback_status === "playing" && mediaSession?.timeline) {
      const interval = setInterval(() => {
        setLocalTick(Date.now());
      }, 250);
      return () => clearInterval(interval);
    }
  }, [mediaSession?.playback_status, mediaSession?.timeline]);

  useEffect(() => {
    loadConfig();

    ipc
      .getMediaState()
      .then((initial) => {
        if (initial) {
          setMediaSession(initial);
          lastServerPosRef.current = initial.timeline?.position_secs ?? 0;
          lastServerTsRef.current = Date.now();
          if (initial.is_pinned) {
            setPinnedSource(initial.source_id);
          }
        }
      })
      .catch((err) =>
        console.error("Failed to load initial media session:", err)
      );

    const handleIncomingNotifications = (snap: NotificationSnapshot) => {
      setNotificationSnapshot(snap);
      if (!configRef.current.notificationsEnabled) return;

      let hasNew = false;
      for (const item of snap.notifications) {
        if (!seenNotifIdsRef.current.has(item.id)) {
          seenNotifIdsRef.current.add(item.id);
          if (!isFullscreenSuppressedRef.current) {
            notifQueueRef.current.push(item);
            hasNew = true;
          }
        }
      }

      if (
        hasNew &&
        !isFullscreenSuppressedRef.current &&
        ((!previewTimerRef.current && !transientNotifRef.current) ||
          isNotifHoveredRef.current)
      ) {
        playNextNotificationRef.current();
      }
    };

    ipc
      .getNotificationState()
      .then((snap) => {
        setNotificationSnapshot(snap);
        for (const item of snap.notifications) {
          seenNotifIdsRef.current.add(item.id);
        }
      })
      .catch((err) =>
        console.error("Failed to load initial notifications:", err)
      );

    ipc
      .getPrivacyState()
      .then((priv) => {
        setPrivacyState(priv);
      })
      .catch((err) =>
        console.error("Failed to load initial privacy state:", err)
      );

    ipc.getReminders().then(setReminders).catch((e) => console.error(e));
    ipc.getShelfItems().then(setShelfItems).catch((e) => console.error(e));
    ipc
      .getHardwareMetrics()
      .then(setHardwareMetrics)
      .catch((e) => console.error(e));

    const unlistenFullscreen = listen<boolean>(
      "fullscreen-state-changed",
      (event) => {
        isFullscreenSuppressedRef.current = Boolean(event.payload);
        if (event.payload) {
          notifQueueRef.current = [];
          if (previewTimerRef.current) {
            clearTimeout(previewTimerRef.current);
            previewTimerRef.current = null;
          }
          setTransientNotif(null);
          setIsExpanded(false);
          nativeStateRef.current = "compact";
        }
      }
    );

    const unlistenHardware = listen<HardwareMetrics>(
      "hardware-metrics-changed",
      (event) => {
        const m = event.payload;
        setHardwareMetrics(m);

        if (!prevUtilityRef.current) {
          prevUtilityRef.current = {
            isCharging: m.isCharging,
            wifiConnected: m.wifiConnected,
            bluetoothEnabled: m.bluetoothEnabled,
            hotspotActive: m.hotspotActive,
          };
        } else {
          const prev = prevUtilityRef.current;
          let alertToTrigger: SystemUtilityAlert | null = null;

          if (m.isCharging !== prev.isCharging) {
            alertToTrigger = m.isCharging
              ? {
                  kind: "charger_on",
                  label: "Charging",
                  detail: `${m.batteryPercentage ?? 100}%`,
                  color: "#22c55e",
                }
              : {
                  kind: "charger_off",
                  label: "On Battery",
                  detail: `${m.batteryPercentage ?? 100}%`,
                  color: "#f59e0b",
                };
          } else if (m.wifiConnected !== prev.wifiConnected) {
            alertToTrigger = m.wifiConnected
              ? {
                  kind: "wifi_on",
                  label: "Wi-Fi",
                  detail: "Connected",
                  color: "#38bdf8",
                }
              : {
                  kind: "wifi_off",
                  label: "Wi-Fi",
                  detail: "Disconnected",
                  color: "#94a3b8",
                };
          } else if (m.bluetoothEnabled !== prev.bluetoothEnabled) {
            alertToTrigger = m.bluetoothEnabled
              ? {
                  kind: "bt_on",
                  label: "Bluetooth",
                  detail: "Connected",
                  color: "#60a5fa",
                }
              : {
                  kind: "bt_off",
                  label: "Bluetooth",
                  detail: "Off",
                  color: "#94a3b8",
                };
          } else if (m.hotspotActive !== prev.hotspotActive) {
            alertToTrigger = m.hotspotActive
              ? {
                  kind: "hotspot_on",
                  label: "Hotspot",
                  detail: "Active",
                  color: "#34d399",
                }
              : {
                  kind: "hotspot_off",
                  label: "Hotspot",
                  detail: "Off",
                  color: "#94a3b8",
                };
          }

          if (
            !alertToTrigger &&
            config.networkActivityMode === "full" &&
            m.lastCompletedDownload &&
            m.lastCompletedDownload !== prevDownloadRef.current
          ) {
            prevDownloadRef.current = m.lastCompletedDownload;
            alertToTrigger = {
              kind: "download_done",
              label: "Download Done",
              detail: m.lastCompletedDownload,
              color: "#10b981",
            };
          }

          prevUtilityRef.current = {
            isCharging: m.isCharging,
            wifiConnected: m.wifiConnected,
            bluetoothEnabled: m.bluetoothEnabled,
            hotspotActive: m.hotspotActive,
          };

          if (alertToTrigger) {
            triggerUtilityAlertRef.current(alertToTrigger);
          }
        }
      }
    );

    const unlistenDrop = listen<{ paths: string[] }>(
      "tauri://drag-drop",
      async (event) => {
        const paths = event.payload?.paths;
        if (!paths || paths.length === 0) return;
        try {
          let current = await ipc.getShelfItems();
          for (const p of paths) {
            if (current.length >= 10) break;
            await ipc.addShelfItem(p);
            current = await ipc.getShelfItems();
          }
          setShelfItems(current);
          setActiveSection(0);
          setShowFileShelf(true);
          setIsClosing(false);
          setIsExpanded(true);
          if (nativeStateRef.current !== "expanded") {
            nativeStateRef.current = "expanded";
            ipc.setIslandState("expanded").catch((e) => console.error(e));
          }
        } catch (err) {
          console.error("Drop file error:", err);
        }
      }
    );

    const unlistenTimer = listen<{ action: string; durationSecs?: number }>(
      "timer-command",
      (event) => {
        if (event.payload.action === "start" && event.payload.durationSecs) {
          startTimer(event.payload.durationSecs);
        } else if (event.payload.action === "stop") {
          stopTimer();
        }
      }
    );

    const unlistenConfig = listen<AppConfig>("config-changed", (event) => {
      useConfigStore.setState({ config: event.payload });
    });

    const unlistenCollapse = listen("collapse-island", () => {
      setIsPinnedExpanded(false);
      pointerInsideRef.current = false;
      reset3DTilt();
      triggerCollapse(++transitionIdRef.current);
    });

    const unlistenPin = listen("toggle-pin-expanded", () => {
      setIsPinnedExpanded((prev) => {
        const next = !prev;
        if (next && !isExpandedRef.current) {
          setIsExpanded(true);
          nativeStateRef.current = "expanded";
          ipc.setIslandState("expanded").catch(() => {});
        }
        return next;
      });
    });

    // 3D Closing / Retract animation before hiding native window
    const unlistenHide = listen("request-hide-island", () => {
      pointerInsideRef.current = false;
      reset3DTilt();
      setIsClosing(true);
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
      hideTimerRef.current = setTimeout(() => {
        setIsExpanded(false);
        nativeStateRef.current = "compact";
        ipc
          .hideIslandWindow()
          .catch((e) => console.error("Failed to hide island:", e))
          .finally(() => {
            setIsClosing(false);
          });
      }, 240);
    });

    // 3D Spring Materialize when shown from tray
    const unlistenShown = listen("island-shown", () => {
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
      setIsClosing(false);
      reset3DTilt();
    });

    const unlistenNotifs = listen<NotificationSnapshot>(
      "notification-state-changed",
      (event) => {
        handleIncomingNotifications(event.payload);
      }
    );

    const unlistenPrivacy = listen<PrivacyState>(
      "privacy-state-changed",
      (event) => {
        const p = event.payload;
        if (p.cameraApp?.includes("(Test)") || p.microphoneApp?.includes("(Test)")) {
          // Hold off the periodic hardware poll so it can't overwrite the simulated state mid-cycle
          privacyTestUntilRef.current = Date.now() + 5000;
        } else if (Date.now() < privacyTestUntilRef.current) {
          // Respect active simulated test until timer elapses
          return;
        }
        setPrivacyState(p);
      }
    );

    const unlistenMedia = listen<MediaSessionSnapshot | null>(
      "media-state-changed",
      (event) => {
        const session = event.payload;
        setMediaSession(session);

        if (session) {
          lastServerPosRef.current = session.timeline?.position_secs ?? 0;
          lastServerTsRef.current = Date.now();
          if (session.is_pinned) {
            setPinnedSource(session.source_id);
          }

          if (pendingSeekPosRef.current !== null && session.timeline) {
            const target = pendingSeekPosRef.current;
            const incoming = session.timeline.position_secs;
            if (
              Math.abs(incoming - target) <= 2.5 ||
              (session.playback_status === "playing" && incoming >= target)
            ) {
              pendingSeekPosRef.current = null;
              setPendingSeekPos(null);
              if (seekTimeoutRef.current) {
                clearTimeout(seekTimeoutRef.current);
                seekTimeoutRef.current = null;
              }
            }
          }

          const trackKey = `${session.source_id}:${session.track.title}:${session.track.artist}`;
          const isNowPlaying = session.playback_status === "playing";
          if (
            isNowPlaying &&
            session.track.title &&
            lastPlayingTrackRef.current !== trackKey
          ) {
            lastPlayingTrackRef.current = trackKey;
            if (
              !isFullscreenSuppressedRef.current &&
              !pointerInsideRef.current &&
              !isExpandedRef.current
            ) {
              setIsExpanded(true);
              if (nativeStateRef.current !== "expanded") {
                nativeStateRef.current = "expanded";
                ipc.setIslandState("expanded").catch((e) => console.error(e));
              }
              if (autoExpandTimerRef.current)
                clearTimeout(autoExpandTimerRef.current);
              autoExpandTimerRef.current = setTimeout(() => {
                if (!pointerInsideRef.current) {
                  triggerCollapse(++transitionIdRef.current);
                }
              }, 1800);
            }
          }
        }
      }
    );

    const unlistenSpectrum = listen<AudioSpectrumState>(
      "media-audio-spectrum",
      (event) => {
        setAudioSpectrum(event.payload);
      }
    );

    const unlistenReplayOnboarding = listen("replay-onboarding", () => {
      showOnboardingRef.current = true;
      setShowOnboarding(true);
      setIsClosing(false);
      setIsExpanded(true);
      if (nativeStateRef.current !== "expanded") {
        nativeStateRef.current = "expanded";
        ipc.setIslandState("expanded").catch((e) => console.error(e));
      }
    });

    // Phase 9 Global Hotkeys Listeners (Alt+O, Alt+S, Alt+M)
    const unlistenHotkeyToggle = listen("hotkey-toggle-expand", () => {
      if (isExpandedRef.current) {
        setIsPinnedExpanded(false);
        pointerInsideRef.current = false;
        reset3DTilt();
        triggerCollapse(++transitionIdRef.current);
      } else {
        pointerInsideRef.current = true;
        setIsClosing(false);
        setIsExpanded(true);
        if (nativeStateRef.current !== "expanded") {
          nativeStateRef.current = "expanded";
          ipc.setIslandState("expanded").catch(() => {});
        }
      }
    });

    const unlistenStealth = listen<boolean>("stealth-mode-toggled", (event) => {
      const isStealth = Boolean(event.payload);
      triggerUtilityAlertRef.current({
        kind: isStealth ? "stealth_on" : "stealth_off",
        label: isStealth ? "Stealth Active" : "Stealth Off",
        detail: isStealth ? "Hidden in Rec/Share" : "Visible in Captures",
        color: isStealth ? "#a855f7" : "#94a3b8",
      });
    });

    const unlistenHotkeyMedia = listen("hotkey-media-play-pause", () => {
      handleMediaCommand("PlayPause");
    });

    const unlistenReminders = listen<ReminderItem[]>("reminders-updated", (event) => {
      if (Array.isArray(event.payload)) {
        setReminders(event.payload);
      }
    });

    return () => {
      unlistenReminders.then((unlisten) => unlisten());
      unlistenHotkeyToggle.then((unlisten) => unlisten());
      unlistenStealth.then((unlisten) => unlisten());
      unlistenPrivacy.then((unlisten) => unlisten());
      unlistenHotkeyMedia.then((unlisten) => unlisten());
      unlistenSpectrum.then((unlisten) => unlisten());
      unlistenReplayOnboarding.then((unlisten) => unlisten());
      unlistenFullscreen.then((unlisten) => unlisten());
      unlistenConfig.then((unlisten) => unlisten());
      unlistenCollapse.then((unlisten) => unlisten());
      unlistenHide.then((unlisten) => unlisten());
      unlistenShown.then((unlisten) => unlisten());
      unlistenMedia.then((unlisten) => unlisten());
      unlistenNotifs.then((unlisten) => unlisten());
      unlistenHardware.then((unlisten) => unlisten());
      unlistenDrop.then((unlisten) => unlisten());
      unlistenTimer.then((unlisten) => unlisten());
      unlistenPin.then((unlisten) => unlisten());
      if (collapseGraceTimerRef.current)
        clearTimeout(collapseGraceTimerRef.current);
      if (watchdogTimerRef.current) clearTimeout(watchdogTimerRef.current);
      if (hideTimerRef.current) clearTimeout(hideTimerRef.current);
      if (autoExpandTimerRef.current) clearTimeout(autoExpandTimerRef.current);
      if (seekTimeoutRef.current) clearTimeout(seekTimeoutRef.current);
      if (previewTimerRef.current) clearTimeout(previewTimerRef.current);
      if (notifEntranceTimerRef.current)
        clearTimeout(notifEntranceTimerRef.current);
      if (utilityTimerRef.current) clearTimeout(utilityTimerRef.current);
    };
  }, [loadConfig]);

  // Live system clock ticker (1-second interval)
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // Completion-driven native collapse finalizer
  const finalizeNativeCollapse = async (transitionId: number) => {
    if (contextMenuOpenRef.current) {
      return;
    }

    if (watchdogTimerRef.current) {
      clearTimeout(watchdogTimerRef.current);
      watchdogTimerRef.current = null;
    }

    if (transitionIdRef.current !== transitionId || pointerInsideRef.current) {
      return;
    }

    const targetState =
      transientNotifRef.current
        ? isNotifHoveredRef.current
          ? "notification_hover"
          : "notification"
        : utilityAlertRef.current?.kind === "timer_done"
        ? "notification"
        : isDualCompactRef.current
        ? "compact_dual"
        : "compact";
    if (nativeStateRef.current !== targetState) {
      nativeStateRef.current = targetState;
      try {
        await ipc.setIslandState(targetState);
        if (
          (pointerInsideRef.current && !transientNotifRef.current && utilityAlertRef.current?.kind !== "timer_done") ||
          contextMenuOpenRef.current
        ) {
          nativeStateRef.current = "expanded";
          ipc.setIslandState("expanded").catch((err) => console.error(err));
          return;
        }
        if (transitionIdRef.current === transitionId) {
          setActiveSection(0);
          setShowHomeNotifications(false);
          setSelectedNotifId(null);
        }
      } catch (err) {
        console.error(
          "Failed to restore compact/notification island state:",
          err
        );
        nativeStateRef.current = "expanded";
      }
    }
  };

  // Initiates visual 3D collapse and sets up completion-driven synchronization
  const triggerCollapse = (transitionId: number) => {
    if (contextMenuOpenRef.current) {
      return;
    }

    reset3DTilt();
    if (isExpandedRef.current && showHomeNotifications) {
      transientNotifRef.current = null;
      setTransientNotif(null);
      isNotifHoveredRef.current = false;
      setIsNotifHovered(false);
    }
    setIsExpanded(false);

    if (prefersReducedMotion) {
      finalizeNativeCollapse(transitionId);
      return;
    }

    if (watchdogTimerRef.current) {
      clearTimeout(watchdogTimerRef.current);
    }
    watchdogTimerRef.current = setTimeout(() => {
      if (
        transitionIdRef.current === transitionId &&
        !pointerInsideRef.current &&
        !contextMenuOpenRef.current
      ) {
        finalizeNativeCollapse(transitionId);
      }
    }, 480);
  };

  // Immediate 3D visual expansion on pointer enter
  const handleMouseEnter = () => {
    pointerInsideRef.current = true;
    const currentTransitionId = ++transitionIdRef.current;

    if (collapseGraceTimerRef.current) {
      clearTimeout(collapseGraceTimerRef.current);
      collapseGraceTimerRef.current = null;
    }
    if (notifHoverGraceTimerRef.current) {
      clearTimeout(notifHoverGraceTimerRef.current);
      notifHoverGraceTimerRef.current = null;
    }
    if (watchdogTimerRef.current) {
      clearTimeout(watchdogTimerRef.current);
      watchdogTimerRef.current = null;
    }
    if (autoExpandTimerRef.current) {
      clearTimeout(autoExpandTimerRef.current);
      autoExpandTimerRef.current = null;
    }

    // Hotfix 1: If transient notification preview is active, hover semi-expands the notification pill itself
    // and unlocks the "notification_hover" GDI region so bottom Open/View/Dismiss buttons are 100% clickable!
    if (transientNotifRef.current && !isExpandedRef.current) {
      isNotifHoveredRef.current = true;
      setIsNotifHovered(true);
      if (nativeStateRef.current !== "notification_hover") {
        nativeStateRef.current = "notification_hover";
        ipc.setIslandState("notification_hover").catch(() => {});
      }
      if (previewTimerRef.current) {
        clearTimeout(previewTimerRef.current);
        previewTimerRef.current = null;
      }
      return;
    }

    // Hotfix 2: If Timer Completed celebration capsule is active, pause timer while hovered
    if (utilityAlertRef.current?.kind === "timer_done" && !isExpandedRef.current) {
      if (utilityTimerRef.current) {
        clearTimeout(utilityTimerRef.current);
        utilityTimerRef.current = null;
      }
      return;
    }

    // Expand trigger setting (Sub-Phase 6.2): if set to click, ignore hover enter
    if (configRef.current.expandTrigger === "click" && !isExpandedRef.current) {
      return;
    }

    // While timers are running, calls/meetings are active, or mic/cam is in use,
    // require an explicit click to expand so user can interact with compact buttons without hover jump
    const isInteractiveCompactActive =
      timerRemainingSecs > 0 ||
      callDurationSecs !== null ||
      Boolean(privacyState?.cameraActive || privacyState?.microphoneActive);

    if (isInteractiveCompactActive && !isExpandedRef.current) {
      return;
    }

    setIsClosing(false);
    setIsExpanded(true);

    if (nativeStateRef.current !== "expanded") {
      nativeStateRef.current = "expanded";
      ipc.setIslandState("expanded").catch((err) => {
        console.error("Failed to set native expanded state:", err);
        if (
          transitionIdRef.current === currentTransitionId &&
          !pointerInsideRef.current
        ) {
          nativeStateRef.current =
            transientNotifRef.current || notifQueueRef.current.length > 0
              ? "notification"
              : isDualCompactRef.current
              ? "compact_dual"
              : "compact";
        }
      });
    }
  };

  const handleMouseLeave = () => {
    if (
      contextMenuOpenRef.current ||
      isPinnedExpandedRef.current ||
      showOnboardingRef.current
    ) {
      return;
    }

    pointerInsideRef.current = false;
    reset3DTilt();

    // Hotfix 1: Grace period before collapsing hovered notification so moving cursor to bottom buttons never flickers
    if (transientNotifRef.current && !isExpandedRef.current) {
      if (notifHoverGraceTimerRef.current) {
        clearTimeout(notifHoverGraceTimerRef.current);
      }
      notifHoverGraceTimerRef.current = setTimeout(() => {
        if (pointerInsideRef.current || !transientNotifRef.current || isExpandedRef.current) {
          return;
        }
        isNotifHoveredRef.current = false;
        setIsNotifHovered(false);
        setTimeout(() => {
          if (
            !isNotifHoveredRef.current &&
            transientNotifRef.current &&
            !isExpandedRef.current
          ) {
            nativeStateRef.current = "notification";
            ipc.setIslandState("notification").catch(() => {});
          }
        }, 250);
        if (!previewTimerRef.current) {
          previewTimerRef.current = setTimeout(() => {
            playNextNotificationRef.current();
          }, 2200);
        }
      }, 220);
      return;
    }

    // Hotfix 2: If Timer Completed celebration capsule was hovered, always restart auto-revert countdown on leave!
    if (utilityAlertRef.current?.kind === "timer_done" && !isExpandedRef.current) {
      if (utilityTimerRef.current) {
        clearTimeout(utilityTimerRef.current);
      }
      utilityTimerRef.current = setTimeout(() => {
        dismissUtilityAlert();
      }, 3500);
      return;
    }

    if (collapseGraceTimerRef.current) {
      clearTimeout(collapseGraceTimerRef.current);
    }

    const currentTransitionId = ++transitionIdRef.current;

    collapseGraceTimerRef.current = setTimeout(() => {
      if (
        transitionIdRef.current !== currentTransitionId ||
        pointerInsideRef.current ||
        contextMenuOpenRef.current ||
        isPinnedExpandedRef.current
      ) {
        return;
      }
      triggerCollapse(currentTransitionId);
    }, 260);
  };

  const handleTransitionEnd = (e: React.TransitionEvent<HTMLDivElement>) => {
    if (contextMenuOpenRef.current) {
      return;
    }
    if (e.target !== islandRef.current) {
      return;
    }
    if (e.propertyName !== "width") {
      return;
    }
    if (
      !isExpanded &&
      !pointerInsideRef.current &&
      !transientNotifRef.current &&
      utilityAlertRef.current?.kind !== "timer_done"
    ) {
      finalizeNativeCollapse(transitionIdRef.current);
    }
  };

  const handleContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();

    contextMenuOpenRef.current = true;
    reset3DTilt();

    if (collapseGraceTimerRef.current) {
      clearTimeout(collapseGraceTimerRef.current);
      collapseGraceTimerRef.current = null;
    }
    if (watchdogTimerRef.current) {
      clearTimeout(watchdogTimerRef.current);
      watchdogTimerRef.current = null;
    }
    if (autoExpandTimerRef.current) {
      clearTimeout(autoExpandTimerRef.current);
      autoExpandTimerRef.current = null;
    }

    pointerInsideRef.current = true;
    setIsExpanded(true);
    if (nativeStateRef.current !== "expanded") {
      nativeStateRef.current = "expanded";
      try {
        await ipc.setIslandState("expanded");
      } catch (err) {
        console.error("Failed to set native expanded state before menu:", err);
      }
    }

    try {
      const isCursorInside = await ipc.showContextMenu();
      if (!isCursorInside) {
        pointerInsideRef.current = false;
        if (collapseGraceTimerRef.current) {
          clearTimeout(collapseGraceTimerRef.current);
        }
        const currentTransitionId = ++transitionIdRef.current;
        collapseGraceTimerRef.current = setTimeout(() => {
          if (
            transitionIdRef.current !== currentTransitionId ||
            pointerInsideRef.current ||
            contextMenuOpenRef.current
          ) {
            return;
          }
          triggerCollapse(currentTransitionId);
        }, 280);
      } else {
        pointerInsideRef.current = true;
      }
    } catch (err) {
      console.error("Failed to show native context menu:", err);
    } finally {
      contextMenuOpenRef.current = false;
    }
  };

  // Clamped 3D drum scroll handler between 0 (Home), 1 (Calendar), and 2 (Hardware Monitor)
  const handleWheel = (e: React.WheelEvent) => {
    if (!isExpanded) return;
    const now = Date.now();
    if (now - lastScrollTimeRef.current < 210) return;
    if (Math.abs(e.deltaY) < 10) return;

    if (e.deltaY > 0) {
      setActiveSection((prev) => {
        if (prev < dynamicSections.length - 1) {
          lastScrollTimeRef.current = now;
          return prev + 1;
        }
        return prev;
      });
    } else if (e.deltaY < 0) {
      setActiveSection((prev) => {
        if (prev > 0) {
          lastScrollTimeRef.current = now;
          return prev - 1;
        }
        return prev;
      });
    }
  };

  // Reminder and Productivity handlers
  const loadReminders = async () => {
    try {
      const items = await ipc.getReminders();
      setReminders(items);
    } catch (e) {
      console.error("Failed to load reminders:", e);
    }
  };

  const handleAddReminder = async (e?: React.FormEvent, customCat?: string) => {
    if (e) e.preventDefault();
    if (!newReminderTitle.trim()) return;
    try {
      const dueMs = selectedCalendarDate.getTime();
      const cat = customCat || newReminderCategory || "task";
      await ipc.addReminder(
        newReminderTitle.trim(),
        dueMs,
        cat,
        "local"
      );
      setNewReminderTitle("");
      await loadReminders();
    } catch (err) {
      console.error("Failed to add reminder:", err);
    }
  };

  const handleToggleReminder = async (id: string) => {
    try {
      await ipc.toggleReminder(id);
      await loadReminders();
    } catch (err) {
      console.error("Failed to toggle reminder:", err);
    }
  };

  const handleDeleteReminder = async (id: string) => {
    try {
      await ipc.deleteReminder(id);
      await loadReminders();
    } catch (err) {
      console.error("Failed to delete reminder:", err);
    }
  };

  // Media command handlers with mutual exclusion
  const handleMediaCommand = async (command: string) => {
    if (isMediaCommandPendingRef.current) return;
    isMediaCommandPendingRef.current = true;
    setIsMediaCommandPending(true);
    try {
      await ipc.sendMediaCommand(command, pinnedSource);
    } catch (err) {
      console.error(`Media command ${command} failed or timed out:`, err);
    } finally {
      isMediaCommandPendingRef.current = false;
      setIsMediaCommandPending(false);
    }
  };

  const handleTogglePin = async () => {
    if (!mediaSession || isMediaCommandPendingRef.current) return;
    isMediaCommandPendingRef.current = true;
    setIsMediaCommandPending(true);
    const nextPin = mediaSession.is_pinned ? null : mediaSession.source_id;
    setPinnedSource(nextPin);
    try {
      await ipc.pinMediaSource(nextPin);
    } catch (err) {
      console.error("Pin media source failed or timed out:", err);
    } finally {
      isMediaCommandPendingRef.current = false;
      setIsMediaCommandPending(false);
    }
  };

  const handleSeekPointerDown = (e: React.PointerEvent<HTMLInputElement>) => {
    const duration = mediaSession?.timeline?.duration_secs ?? 0;
    if (!mediaSession?.controls.can_seek || duration <= 0) return;
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // Ignore
    }
    isDraggingRef.current = true;
    setIsDragging(true);
    const val = parseFloat(e.currentTarget.value);
    dragPosRef.current = val;
    setDragPos(val);
  };

  const handleSeekChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseFloat(e.target.value);
    dragPosRef.current = val;
    setDragPos(val);
  };

  const handleSeekCommit = async (valToCommit: number) => {
    isDraggingRef.current = false;
    setIsDragging(false);

    const duration = mediaSession?.timeline?.duration_secs ?? 0;
    if (!mediaSession?.controls.can_seek || duration <= 0) return;

    pendingSeekPosRef.current = valToCommit;
    setPendingSeekPos(valToCommit);

    if (seekTimeoutRef.current) {
      clearTimeout(seekTimeoutRef.current);
    }
    seekTimeoutRef.current = setTimeout(() => {
      if (pendingSeekPosRef.current !== null) {
        pendingSeekPosRef.current = null;
        setPendingSeekPos(null);
      }
    }, 2500);

    try {
      const updated = await ipc.seekMedia(valToCommit, pinnedSource);
      if (updated?.timeline) {
        lastServerPosRef.current = updated.timeline.position_secs;
        lastServerTsRef.current = Date.now();
        if (Math.abs(updated.timeline.position_secs - valToCommit) <= 2.5) {
          pendingSeekPosRef.current = null;
          setPendingSeekPos(null);
          if (seekTimeoutRef.current) {
            clearTimeout(seekTimeoutRef.current);
            seekTimeoutRef.current = null;
          }
        }
        setMediaSession(updated);
      }
    } catch (err) {
      console.error("Seek failed or timed out:", err);
      pendingSeekPosRef.current = null;
      setPendingSeekPos(null);
      if (seekTimeoutRef.current) {
        clearTimeout(seekTimeoutRef.current);
        seekTimeoutRef.current = null;
      }
    }
  };

  const handleSeekPointerUp = (e: React.PointerEvent<HTMLInputElement>) => {
    try {
      if (e.currentTarget.hasPointerCapture(e.pointerId)) {
        e.currentTarget.releasePointerCapture(e.pointerId);
      }
    } catch {
      // Ignore
    }
    if (!isDraggingRef.current) return;
    handleSeekCommit(parseFloat(e.currentTarget.value));
  };

  const handleSeekPointerCancel = (e: React.PointerEvent<HTMLInputElement>) => {
    try {
      if (e.currentTarget.hasPointerCapture(e.pointerId)) {
        e.currentTarget.releasePointerCapture(e.pointerId);
      }
    } catch {
      // Ignore
    }
    isDraggingRef.current = false;
    setIsDragging(false);
    pendingSeekPosRef.current = null;
    setPendingSeekPos(null);
    if (seekTimeoutRef.current) {
      clearTimeout(seekTimeoutRef.current);
      seekTimeoutRef.current = null;
    }
  };

  const handleSeekKeyUp = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (
      [
        "ArrowLeft",
        "ArrowRight",
        "ArrowUp",
        "ArrowDown",
        "Home",
        "End",
        "PageUp",
        "PageDown",
      ].includes(e.key)
    ) {
      handleSeekCommit(parseFloat(e.currentTarget.value));
    }
  };

  const formatTime = (date: Date) => {
    const is12h = config.timeFormat === "12h";
    let hours = date.getHours();
    const minutes = String(date.getMinutes()).padStart(2, "0");

    if (is12h) {
      const ampm = hours >= 12 ? "PM" : "AM";
      hours = hours % 12 || 12;
      return { timeStr: `${hours}:${minutes}`, ampm };
    }
    return {
      timeStr: `${String(hours).padStart(2, "0")}:${minutes}`,
      ampm: "",
    };
  };

  const formatDate = (date: Date) => {
    return date.toLocaleDateString(undefined, {
      weekday: "long",
      month: "long",
      day: "numeric",
    });
  };

  const formatRelativeTime = (timestampMs: number) => {
    const diffSec = Math.floor((Date.now() - timestampMs) / 1000);
    if (diffSec < 10) return "Just now";
    if (diffSec < 60) return `${diffSec}s ago`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    return `${diffHr}h ago`;
  };

  const clearTransientNotification = (dismissedId?: number) => {
    if (previewTimerRef.current) {
      clearTimeout(previewTimerRef.current);
      previewTimerRef.current = null;
    }
    if (notifHoverGraceTimerRef.current) {
      clearTimeout(notifHoverGraceTimerRef.current);
      notifHoverGraceTimerRef.current = null;
    }
    if (dismissedId !== undefined) {
      notifQueueRef.current = notifQueueRef.current.filter((n) => n.id !== dismissedId);
    }
    transientNotifRef.current = null;
    setTransientNotif(null);
    isNotifHoveredRef.current = false;
    setIsNotifHovered(false);
    setIsNotifEntering(false);
  };

  const handleDismissNotification = async (id: number) => {
    try {
      notifQueueRef.current = notifQueueRef.current.filter((n) => n.id !== id);
      if (transientNotifRef.current?.id === id) {
        playNextNotification();
      }
      if (selectedNotifId === id) {
        setSelectedNotifId(null);
      }
      setNotificationSnapshot((prev) => {
        if (!prev) return null;
        const updated = prev.notifications.filter((n) => n.id !== id);
        if (updated.length === 0) {
          setShowHomeNotifications(false);
        }
        return {
          ...prev,
          notifications: updated,
          activeNotification:
            prev.activeNotification?.id === id ? null : prev.activeNotification,
          totalCount: Math.max(0, prev.totalCount - 1),
        };
      });

      await ipc.dismissNotification(id);
    } catch (err) {
      console.error("Failed to dismiss notification:", err);
    }
  };

  const handleClearAllNotifications = async () => {
    try {
      await ipc.clearAllNotifications();
      notifQueueRef.current = [];
      if (previewTimerRef.current) {
        clearTimeout(previewTimerRef.current);
        previewTimerRef.current = null;
      }
      setTransientNotif(null);
      setSelectedNotifId(null);
      setShowHomeNotifications(false);
      if (!pointerInsideRef.current && !isExpanded) {
        nativeStateRef.current = "compact";
        setTimeout(() => {
          if (
            !pointerInsideRef.current &&
            !isExpandedRef.current &&
            !transientNotifRef.current
          ) {
            ipc.setIslandState("compact").catch((e) => console.error(e));
          }
        }, 280);
      }
      setNotificationSnapshot((prev) =>
        prev
          ? {
              ...prev,
              notifications: [],
              activeNotification: null,
              totalCount: 0,
              overflowCount: 0,
            }
          : null
      );
    } catch (err) {
      console.error("Failed to clear notifications:", err);
    }
  };

  const handleViewNotification = (notif?: NotificationItem | null) => {
    const targetId = notif?.id ?? transientNotifRef.current?.id;
    clearTransientNotification(targetId);
    pointerInsideRef.current = true;
    setIsClosing(false);
    setSelectedNotifId(notif?.id ?? null);
    setActiveSection(0);
    setShowHomeNotifications(true);
    setIsExpanded(true);
    if (nativeStateRef.current !== "expanded") {
      nativeStateRef.current = "expanded";
      ipc.setIslandState("expanded").catch((err) => console.error(err));
    }
  };

  const handleOpenHome = () => {
    setActiveSection(0);
    setShowHomeNotifications(false);
    pointerInsideRef.current = true;
    setIsClosing(false);
    setIsExpanded(true);
    if (nativeStateRef.current !== "expanded") {
      nativeStateRef.current = "expanded";
      ipc.setIslandState("expanded").catch((err) => console.error(err));
    }
  };

  const timeDisplay = formatTime(currentTime);
  const resolvedThemeId =
    config.theme === "system" ? (systemIsLight ? "light" : "black") : config.theme;
  const baseThemeMeta = getThemeMeta(resolvedThemeId);

  const activeThemeMeta = useMemo(() => {
    if (config.adaptiveAlbumAccent && isMediaActive && albumAccentColor) {
      return {
        ...baseThemeMeta,
        color: albumAccentColor,
        accent: albumAccentColor,
        borderGlow: `${albumAccentColor}55`,
      };
    }
    return baseThemeMeta;
  }, [baseThemeMeta, config.adaptiveAlbumAccent, isMediaActive, albumAccentColor]);

  const displayNotif =
    (selectedNotifId !== null
      ? notificationSnapshot?.notifications.find(
          (n) => n.id === selectedNotifId
        ) ?? (transientNotif?.id === selectedNotifId ? transientNotif : null)
      : null) ||
    transientNotif ||
    (notificationSnapshot?.notifications &&
    notificationSnapshot.notifications.length > 0
      ? notificationSnapshot.notifications[0]
      : null);

  // Compact privacy status light (Camera=green icon, Mic=yellow icon, Both=both icons side-by-side)
  const getCompactPrivacyLight = () => {
    if (!config.showPrivacyIndicators || !privacyState) return null;
    const { cameraActive, microphoneActive } = privacyState;
    if (!cameraActive && !microphoneActive) return null;

    const camTitle = `Camera in use: ${privacyState.cameraApp || "Active"}`;
    const micTitle = `Microphone in use: ${privacyState.microphoneApp || "Active"}`;

    return (
      <span className="inline-flex items-center space-x-1 shrink-0">
        {cameraActive && (
          <span
            className="relative inline-flex items-center justify-center w-4 h-4 rounded-full shrink-0 animate-pulse"
            style={{
              backgroundColor: "#22c55e28",
              border: "1px solid #22c55eb3",
              boxShadow: "0 0 8px #22c55e80",
              color: "#22c55e",
            }}
            title={camTitle}
          >
            <svg
              className="w-2.5 h-2.5 shrink-0"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M4 7a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v1.6l3.4-2.1a.8.8 0 0 1 1.2.7v9.6a.8.8 0 0 1-1.2.7L15 15.4V17a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7z" />
            </svg>
          </span>
        )}
        {microphoneActive && (
          <span
            className="relative inline-flex items-center justify-center w-4 h-4 rounded-full shrink-0 animate-pulse"
            style={{
              backgroundColor: "#facc1528",
              border: "1px solid #facc15b3",
              boxShadow: "0 0 8px #facc1580",
              color: "#facc15",
            }}
            title={micTitle}
          >
            <svg
              className="w-2.5 h-2.5 shrink-0"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M12 3a3 3 0 0 0-3 3v5a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zm-6 8a1 1 0 1 1 2 0 4 4 0 0 0 8 0 1 1 0 1 1 2 0 6 6 0 0 1-5 5.9V20h2a1 1 0 1 1 0 2H9a1 1 0 1 1 0-2h2v-3.1A6 6 0 0 1 6 11z" />
            </svg>
          </span>
        )}
      </span>
    );
  };

  const handleFreeMemory = async () => {
    try {
      const result = await ipc.freeUpMemory();
      return { freedMb: result.freedMb };
    } catch (err) {
      console.error("Failed to free up working set memory:", err);
      throw err;
    }
  };

  const getExpandedPrivacySentence = () => {
    if (!config.showPrivacyIndicators || !privacyState) return null;
    const { cameraActive, microphoneActive, cameraApp, microphoneApp } =
      privacyState;
    if (!cameraActive && !microphoneActive) return null;

    const cam = cameraApp?.trim();
    const mic = microphoneApp?.trim();

    if (cameraActive && microphoneActive) {
      if (cam && mic && cam.toLowerCase() === mic.toLowerCase()) {
        return `${cam} is using your camera and microphone`;
      }
      if (cam && mic) {
        return `${cam} (cam) & ${mic} (mic) active`;
      }
      return "Camera and microphone are in use";
    }

    if (cameraActive) {
      if (cam) return `${cam} is using your camera`;
      return "Camera is in use";
    }

    if (microphoneActive) {
      if (mic) return `${mic} is using your microphone`;
      return "Microphone is in use";
    }

    return null;
  };

  const expandedPrivacySentence = getExpandedPrivacySentence();

  // Compute 3D shell transform (combining Closing retract, Notification drop-expand-rewind, and resting state without text-blurring cursor tilt)
  const shellTransform = isClosing
    ? "translate3d(0, -18px, -55px) rotateX(42deg) scale3d(0.42, 0.14, 0.75)"
    : !isExpanded && transientNotif && config.notificationsEnabled
    ? isNotifEntering
      ? "translate3d(0, -10px, -12px) scale3d(0.82, 0.78, 0.92)"
      : "translate3d(0, 7px, 0) scale3d(1, 1, 1)"
    : "translate3d(0, 0, 0) scale3d(1, 1, 1)";

  return (
    <div
      onContextMenu={handleContextMenu}
      onWheel={handleWheel}
      className={`island-container orbit-stage-3d w-full h-full flex items-start justify-center pt-[3px] overflow-hidden select-none relative bg-transparent font-sans theme-${resolvedThemeId} orbit-font-${config.fontFamily || "inter"}`}
    >
      {/* Primary 3D Sculpted Island Shell */}
      <div
        ref={islandRef}
        onMouseEnter={handleMouseEnter}
        onMouseLeave={handleMouseLeave}
        onTransitionEnd={handleTransitionEnd}
        onClick={() => {
          if (
            !isExpanded &&
            !transientNotif &&
            utilityAlert?.kind !== "timer_done"
          ) {
            pointerInsideRef.current = true;
            setIsClosing(false);
            setIsExpanded(true);
            if (nativeStateRef.current !== "expanded") {
              nativeStateRef.current = "expanded";
              ipc.setIslandState("expanded").catch(() => {});
            }
          }
        }}
        style={{
          width: isExpanded
            ? "356px"
            : transientNotif
            ? isNotifHovered
              ? "368px"
              : "316px"
            : utilityAlert?.kind === "timer_done"
            ? "316px"
            : `${compactPillWidth}px`,
          height: isExpanded
            ? "168px"
            : transientNotif
            ? isNotifHovered
              ? "112px"
              : "56px"
            : utilityAlert?.kind === "timer_done"
            ? "56px"
            : "40px",
          borderRadius: isExpanded
            ? "26px"
            : transientNotif
            ? isNotifHovered
              ? "26px"
              : "28px"
            : utilityAlert?.kind === "timer_done"
            ? "28px"
            : "20px",
          borderColor:
            utilityAlert?.kind === "timer_done"
              ? "rgba(251, 191, 36, 0.72)"
              : "var(--island-border)",
          boxShadow:
            utilityAlert?.kind === "timer_done"
              ? "0 0 28px rgba(245, 158, 11, 0.55), inset 0 0 14px rgba(251, 191, 36, 0.25)"
              : undefined,
          backgroundColor: isExpanded
            ? "var(--island-panel)"
            : "var(--island-pill)",
          opacity: isClosing || isMeetingSuppressed ? 0 : 1,
          pointerEvents: isMeetingSuppressed ? "none" : undefined,
          transform: shellTransform,
          // @ts-expect-error CSS custom property
          "--surface-alpha": 0.94 + ((config.surfaceOpacity ?? 80) / 100) * 0.06,
          transition: prefersReducedMotion
            ? "none"
            : isClosing
            ? "transform 230ms cubic-bezier(0.32, 0, 0.67, 0), opacity 210ms ease-in"
            : isExpanded
            ? "width 330ms cubic-bezier(0.22, 1.26, 0.36, 1), height 360ms cubic-bezier(0.18, 1.16, 0.32, 1) 20ms, border-radius 300ms ease, border-color 260ms ease, background-color 260ms ease, transform 140ms ease-out, opacity 220ms ease-out"
            : "height 280ms cubic-bezier(0.22, 1.15, 0.36, 1), width 300ms cubic-bezier(0.22, 1.15, 0.36, 1), border-radius 260ms ease, border-color 240ms ease, background-color 240ms ease, box-shadow 300ms ease, transform 260ms cubic-bezier(0.22, 1, 0.36, 1), opacity 220ms ease-out",
        }}
        className={`orbit-shell-3d transparency-${
          config.transparencyMode || "glass"
        } relative overflow-hidden select-none border ${
          isExpanded ? "cursor-default" : "cursor-pointer"
        }`}
      >
        {/* COMPACT / INTERMEDIATE 3D CAPSULE LAYER */}
        <CompactCapsule
          isExpanded={isExpanded}
          prefersReducedMotion={prefersReducedMotion}
          config={config}
          activeThemeMeta={activeThemeMeta}
          transientNotif={transientNotif}
          isNotifEntering={isNotifEntering}
          isNotifHovered={isNotifHovered}
          setIsNotifHovered={setIsNotifHovered}
          onViewNotification={handleViewNotification}
          onDismissNotification={handleDismissNotification}
          isMediaActive={isMediaActive}
          mediaSession={mediaSession}
          timeDisplay={timeDisplay}
          privacyLight={getCompactPrivacyLight()}
          timerRemainingSecs={timerRemainingSecs}
          timerTotalSecs={timerTotalSecs}
          onStopTimer={stopTimer}
          utilityAlert={utilityAlert}
          weatherInfo={weatherInfo}
          isTimerPaused={isTimerPaused}
          timerLabel={timerLabel}
          onTogglePauseTimer={togglePauseTimer}
          onResetTimer={resetTimer}
          onStartTimer={startTimer}
          onDismissUtilityAlert={dismissUtilityAlert}
          hardwareMetrics={hardwareMetrics}
          callDurationSecs={callDurationSecs}
          onMeasuredCompactWidth={setMeasuredCompactWidth}
          audioSpectrum={audioSpectrum}
          onOpenHome={handleOpenHome}
        />

        {/* EXPANDED 3D PANEL LAYER (356x168 canvas with 3D Drum Section Carousel) */}
        <div
          className="absolute top-0 left-0 w-[356px] h-[168px] px-3 pt-2.5 pb-3.5 pr-6 overflow-hidden select-none"
          style={{
            opacity: isExpanded ? 1 : 0,
            transform: isExpanded
              ? "translate3d(0, 0, 0) scale(1)"
              : "translate3d(0, 12px, -36px) rotateX(-16deg) scale(0.91)",
            pointerEvents: isExpanded ? "auto" : "none",
            transition: prefersReducedMotion
              ? "none"
              : isExpanded
              ? "opacity 210ms ease-out 65ms, transform 340ms cubic-bezier(0.2, 1.18, 0.34, 1) 50ms"
              : "opacity 110ms ease-in 0ms, transform 180ms ease-in 0ms",
          }}
        >
          {/* First-Launch Skippable Guided Tour (Sub-Phase 8.1) */}
          {showOnboarding && (
            <OnboardingOverlay
              activeThemeMeta={activeThemeMeta}
              onComplete={handleCompleteOnboarding}
            />
          )}

          {/* Floating mini privacy badge when scrolled to non-home sections */}
          {activeSection !== 0 && (privacyState?.cameraActive || privacyState?.microphoneActive) && (
            <div className="absolute right-3.5 top-2.5 z-40 flex items-center space-x-1.5 px-2 py-0.5 rounded-full bg-black/60 border border-white/10 shadow-sm backdrop-blur-md">
              {privacyState.cameraActive && (
                <span className="w-2 h-2 rounded-full bg-emerald-400 shadow-[0_0_6px_#34d399] animate-pulse" title="Camera Active" />
              )}
              {privacyState.microphoneActive && (
                <span className="w-2 h-2 rounded-full bg-amber-400 shadow-[0_0_6px_#fbbf24] animate-pulse" title="Microphone Active" />
              )}
              {callDurationSecs !== null && (
                <span className="text-[9px] font-mono font-bold text-white/90 ml-0.5">
                  {Math.floor(callDurationSecs / 60)}:{String(callDurationSecs % 60).padStart(2, "0")}
                </span>
              )}
            </div>
          )}

          {/* 3D Section Indicator Rail on the right edge (Matches reference screenshots) */}
          <div className="absolute right-2 top-1/2 -translate-y-1/2 flex flex-col items-center space-y-1.5 z-30">
            {dynamicSections.map((section, idx) => {
              const isActive = activeSection === idx;
              return (
                <button
                  key={section.id}
                  type="button"
                  title={section.label}
                  onClick={(e) => {
                    e.stopPropagation();
                    setActiveSection(idx);
                  }}
                  className={`w-[5px] h-[5px] rounded-full transition-all duration-250 outline-none focus:outline-none cursor-pointer ${
                    isActive
                      ? "scale-110"
                      : "bg-white/25 hover:bg-white/45 scale-95"
                  }`}
                  style={{
                    backgroundColor: isActive ? "#ffffff" : undefined,
                    boxShadow: isActive
                      ? `0 0 8px #ffffff, 0 0 12px ${activeThemeMeta.color}`
                      : undefined,
                  }}
                  aria-label={section.label}
                />
              );
            })}
          </div>

          {/* 3D Vertical Drum Stage for Sections */}
          <div
            className="relative w-full h-full"
            style={{
              perspective: "700px",
              transformStyle: "preserve-3d",
            }}
          >
            {dynamicSections.map((section, sectionIdx) => {
              const offset = sectionIdx - activeSection;
              const isCurrent = offset === 0;
              return (
                <div
                  key={section.id}
                  className="absolute inset-0 w-full h-full"
                  style={{
                    opacity: isCurrent ? 1 : 0,
                    pointerEvents: isCurrent ? "auto" : "none",
                    transform:
                      prefersReducedMotion || isCurrent
                        ? "none"
                        : `translate3d(0, ${offset * 95}%, ${
                            Math.abs(offset) * -45
                          }px) rotateX(${offset * -28}deg) scale(0.9)`,
                    transition: prefersReducedMotion
                      ? "none"
                      : "transform 380ms cubic-bezier(0.22, 1.15, 0.36, 1), opacity 260ms ease",
                  }}
                >
                  {section.id === "home" && (
                    <HomeSection
                      config={config}
                      activeThemeMeta={activeThemeMeta}
                      showHomeNotifications={showHomeNotifications}
                      setShowHomeNotifications={setShowHomeNotifications}
                      setSelectedNotifId={setSelectedNotifId}
                      displayNotif={displayNotif}
                      notificationSnapshot={notificationSnapshot}
                      onDismissNotification={handleDismissNotification}
                      onClearAllNotifications={handleClearAllNotifications}
                      onViewNotification={handleViewNotification}
                      formatRelativeTime={formatRelativeTime}
                      isMediaActive={isMediaActive}
                      mediaSession={mediaSession}
                      isMediaCommandPending={isMediaCommandPending}
                      onTogglePin={handleTogglePin}
                      onMediaCommand={handleMediaCommand}
                      isDragging={isDragging}
                      dragPos={dragPos}
                      pendingSeekPos={pendingSeekPos}
                      lastServerPos={lastServerPosRef.current}
                      lastServerTs={lastServerTsRef.current}
                      onSeekPointerDown={handleSeekPointerDown}
                      onSeekChange={handleSeekChange}
                      onSeekPointerUp={handleSeekPointerUp}
                      onSeekPointerCancel={handleSeekPointerCancel}
                      onSeekKeyUp={handleSeekKeyUp}
                      formatSeconds={formatSeconds}
                      timeDisplay={timeDisplay}
                      formattedDate={formatDate(currentTime)}
                      expandedPrivacySentence={expandedPrivacySentence}
                      privacyState={privacyState}
                      timerRemainingSecs={timerRemainingSecs}
                      timerTotalSecs={timerTotalSecs}
                      onStartTimer={startTimer}
                      onStopTimer={stopTimer}
                      isTimerPaused={isTimerPaused}
                      timerLabel={timerLabel}
                      onPauseTimer={pauseTimer}
                      onResumeTimer={resumeTimer}
                      onResetTimer={resetTimer}
                      onChangeTimerLabel={setTimerLabel}
                      shelfItems={shelfItems}
                      showFileShelf={showFileShelf}
                      setShowFileShelf={setShowFileShelf}
                      onRefreshShelf={() => ipc.getShelfItems().then(setShelfItems)}
                      callDurationSecs={callDurationSecs}
                      audioSpectrum={audioSpectrum}
                      todayNextEvent={todayNextEvent}
                      onOpenCalendarSection={() => {
                        const calIdx = dynamicSections.findIndex((s) => s.id === "calendar");
                        setActiveSection(calIdx >= 0 ? calIdx : 1);
                      }}
                    />
                  )}
                  {section.id === "multitask" && (
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
                  )}
                  {section.id === "calendar" && (
                    <CalendarSection
                      activeThemeMeta={activeThemeMeta}
                      calendarMonth={calendarMonth}
                      setCalendarMonth={setCalendarMonth}
                      selectedCalendarDate={selectedCalendarDate}
                      setSelectedCalendarDate={setSelectedCalendarDate}
                      reminders={reminders}
                      newReminderTitle={newReminderTitle}
                      setNewReminderTitle={setNewReminderTitle}
                      onAddReminder={handleAddReminder}
                      onToggleReminder={handleToggleReminder}
                      onDeleteReminder={handleDeleteReminder}
                      onImportCalendar={async () => {
                        try {
                          await ipc.pickAndImportCalendar("academic");
                          const updated = await ipc.getReminders();
                          setReminders(updated);
                        } catch (err) {
                          console.error("Failed to import calendar:", err);
                        }
                      }}
                      onRefreshReminders={loadReminders}
                    />
                  )}
                  {section.id === "hardware" && (
                    <HardwareSection
                      activeThemeMeta={activeThemeMeta}
                      hardwareMetrics={hardwareMetrics}
                      formatNetworkSpeed={formatNetworkSpeed}
                      onFreeMemory={handleFreeMemory}
                    />
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
};
