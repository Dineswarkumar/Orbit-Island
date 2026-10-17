import React, { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { ipc } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";
import { AppConfig } from "../core/types/config";
import { getThemeMeta } from "../core/theme";
import { MediaSessionSnapshot } from "../core/types/media";
import {
  NotificationItem,
  NotificationSnapshot,
  PrivacyState,
} from "../core/types/notifications";
import { ReminderItem } from "../core/types/reminders";
import { HardwareMetrics } from "../core/types/hardware";
import { ShelfItem } from "../core/types/shelf";
import { CompactCapsule, SystemUtilityAlert } from "./island/CompactCapsule";
import { HomeSection } from "./island/HomeSection";
import { CalendarSection } from "./island/CalendarSection";
import { HardwareSection } from "./island/HardwareSection";

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

const SECTIONS = [
  { id: "home", label: "Home" },
  { id: "calendar", label: "Calendar" },
  { id: "hardware", label: "Hardware Monitor" },
] as const;

export const IslandApp: React.FC = () => {
  const { config, loadConfig } = useConfigStore();

  const islandRef = useRef<HTMLDivElement>(null);
  const [isExpanded, setIsExpanded] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
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

  // Hardware monitor state
  const [hardwareMetrics, setHardwareMetrics] =
    useState<HardwareMetrics | null>(null);

  // 2-second compact utility alert state
  const [utilityAlert, setUtilityAlert] = useState<SystemUtilityAlert | null>(null);
  const utilityTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const prevUtilityRef = useRef<{
    isCharging: boolean;
    wifiConnected: boolean;
    bluetoothEnabled: boolean;
    hotspotActive: boolean;
  } | null>(null);

  const triggerUtilityAlert = (alert: SystemUtilityAlert) => {
    if (isFullscreenSuppressedRef.current) return;
    setUtilityAlert(alert);
    if (!isExpandedRef.current && !transientNotifRef.current) {
      ipc.setIslandState("compact_alert").catch(() => {});
    }
    if (utilityTimerRef.current) {
      clearTimeout(utilityTimerRef.current);
    }
    utilityTimerRef.current = setTimeout(() => {
      setUtilityAlert(null);
      if (!isExpandedRef.current && !transientNotifRef.current) {
        ipc.setIslandState("compact").catch(() => {});
      }
    }, 2000);
  };
  const triggerUtilityAlertRef = useRef(triggerUtilityAlert);
  triggerUtilityAlertRef.current = triggerUtilityAlert;

  // Timer state (persisted in localStorage per PRD Section 5.3)
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

  const startTimer = (durationSecs: number) => {
    const deadline = Date.now() + durationSecs * 1000;
    setTimerDeadlineMs(deadline);
    setTimerTotalSecs(durationSecs);
    localStorage.setItem(
      "orbit_timer_state",
      JSON.stringify({ deadlineMs: deadline, totalSecs: durationSecs })
    );
  };

  const stopTimer = () => {
    setTimerDeadlineMs(null);
    setTimerTotalSecs(0);
    localStorage.removeItem("orbit_timer_state");
  };

  const timerRemainingSecs = timerDeadlineMs
    ? Math.max(0, Math.ceil((timerDeadlineMs - currentTime.getTime()) / 1000))
    : 0;

  // File Shelf state
  const [shelfItems, setShelfItems] = useState<ShelfItem[]>([]);
  const [showFileShelf, setShowFileShelf] = useState(false);

  // Live refs to avoid stale closures in long-lived Tauri event listeners
  const configRef = useRef<AppConfig>(config);
  configRef.current = config;
  const isExpandedRef = useRef<boolean>(isExpanded);
  isExpandedRef.current = isExpanded;
  const transientNotifRef = useRef<NotificationItem | null>(transientNotif);
  transientNotifRef.current = transientNotif;

  // Transition tracking and coordination refs
  const transitionIdRef = useRef<number>(0);
  const pointerInsideRef = useRef(false);
  const nativeStateRef = useRef<"compact" | "notification" | "expanded">(
    "compact"
  );
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
      setTransientNotif(next);
      setIsNotifHovered(false);
      setIsNotifEntering(true);
      if (notifEntranceTimerRef.current)
        clearTimeout(notifEntranceTimerRef.current);
      notifEntranceTimerRef.current = setTimeout(() => {
        setIsNotifEntering(false);
      }, 35);

      if (!pointerInsideRef.current && !isExpandedRef.current) {
        if (nativeStateRef.current !== "notification") {
          nativeStateRef.current = "notification";
          ipc.setIslandState("notification").catch((e) => console.error(e));
        }
      }
      const dur = (configRef.current.notificationPreviewDurationSec || 4) * 1000;
      previewTimerRef.current = setTimeout(() => {
        playNextNotificationRef.current();
      }, dur);
    } else {
      setTransientNotif(null);
      setIsNotifHovered(false);
      setIsNotifEntering(false);
      if (!pointerInsideRef.current && !isExpandedRef.current) {
        if (nativeStateRef.current !== "compact") {
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
        !previewTimerRef.current &&
        !transientNotifRef.current
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
      pointerInsideRef.current = false;
      reset3DTilt();
      triggerCollapse(++transitionIdRef.current);
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

    const privacyInterval = setInterval(() => {
      ipc
        .getPrivacyState()
        .then((priv) => {
          setPrivacyState(priv);
        })
        .catch((e) => console.error(e));
    }, 2500);

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

    return () => {
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
      clearInterval(privacyInterval);
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

  // Live system clock ticker (1-second interval) & Timer Deadline Checker
  useEffect(() => {
    const timer = setInterval(() => {
      const now = new Date();
      setCurrentTime(now);

      // Check if active countdown timer reached deadline
      if (timerDeadlineMs) {
        if (now.getTime() >= timerDeadlineMs) {
          stopTimer();
          if (!isFullscreenSuppressedRef.current) {
            const finishedNotif: NotificationItem = {
              id: Date.now(),
              appName: "Focus Timer",
              appId: "orbit.timer",
              title: "Timer Complete",
              body: "Your countdown timer has finished.",
              timestampMs: Date.now(),
              isCritical: true,
              count: 1,
              actions: [],
            };
            notifQueueRef.current.push(finishedNotif);
            if (!previewTimerRef.current && !transientNotifRef.current) {
              playNextNotificationRef.current();
            }
          }
        }
      }
    }, 1000);
    return () => clearInterval(timer);
  }, [timerDeadlineMs]);

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
      transientNotifRef.current || notifQueueRef.current.length > 0
        ? "notification"
        : "compact";
    if (nativeStateRef.current !== targetState) {
      nativeStateRef.current = targetState;
      try {
        await ipc.setIslandState(targetState);
        if (pointerInsideRef.current || contextMenuOpenRef.current) {
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
    if (watchdogTimerRef.current) {
      clearTimeout(watchdogTimerRef.current);
      watchdogTimerRef.current = null;
    }
    if (autoExpandTimerRef.current) {
      clearTimeout(autoExpandTimerRef.current);
      autoExpandTimerRef.current = null;
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
              : "compact";
        }
      });
    }
  };

  const handleMouseLeave = () => {
    if (contextMenuOpenRef.current) {
      return;
    }

    pointerInsideRef.current = false;
    reset3DTilt();

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
    if (!isExpanded && !pointerInsideRef.current) {
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
        if (prev < SECTIONS.length - 1) {
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

  const handleAddReminder = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!newReminderTitle.trim()) return;
    try {
      const dueMs = selectedCalendarDate.getTime();
      await ipc.addReminder(
        newReminderTitle.trim(),
        dueMs,
        newReminderCategory,
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

  const handleDismissNotification = async (id: number) => {
    try {
      await ipc.dismissNotification(id);
      if (transientNotif?.id === id) {
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
    if (previewTimerRef.current) {
      clearTimeout(previewTimerRef.current);
      previewTimerRef.current = null;
    }
    if (notif) {
      setSelectedNotifId(notif.id);
    }
    setActiveSection(0);
    setShowHomeNotifications(true);
    setIsExpanded(true);
    if (nativeStateRef.current !== "expanded") {
      nativeStateRef.current = "expanded";
      ipc.setIslandState("expanded").catch((err) => console.error(err));
    }
  };

  const timeDisplay = formatTime(currentTime);
  const activeThemeMeta = getThemeMeta(config.theme);

  const isMediaActive = Boolean(
    mediaSession &&
      (mediaSession.playback_status === "playing" ||
        (mediaSession.playback_status === "paused" &&
          mediaSession.track.title.trim().length > 0))
  );

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

  // Compact privacy status light (strict mapping: Camera=green, Mic=yellow, Both=cyan, Neither=hidden)
  const getCompactPrivacyLight = () => {
    if (isExpanded) return null;
    if (!config.showPrivacyIndicators || !privacyState) return null;
    const { cameraActive, microphoneActive } = privacyState;
    if (!cameraActive && !microphoneActive) return null;

    let color = "#22c55e";
    let title = `Camera in use: ${privacyState.cameraApp || "Active"}`;

    if (cameraActive && microphoneActive) {
      color = "#06b6d4";
      title = "Camera and Microphone active";
    } else if (microphoneActive) {
      color = "#eab308";
      title = `Microphone in use: ${privacyState.microphoneApp || "Active"}`;
    }

    return (
      <span
        className="w-[8px] h-[8px] rounded-full shrink-0 inline-block"
        style={{
          backgroundColor: color,
          boxShadow: `0 0 8px ${color}, inset 0 1px 1px rgba(255,255,255,0.6)`,
        }}
        title={title}
      />
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
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onContextMenu={handleContextMenu}
      onWheel={handleWheel}
      className={`island-container orbit-stage-3d w-full h-full flex items-start justify-center pt-[3px] overflow-hidden select-none relative bg-transparent font-sans theme-${config.theme}`}
    >
      {/* Primary 3D Sculpted Island Shell */}
      <div
        ref={islandRef}
        onTransitionEnd={handleTransitionEnd}
        style={{
          width: isExpanded ? "356px" : transientNotif ? "316px" : "140px",
          height: isExpanded ? "168px" : transientNotif ? "56px" : "40px",
          borderRadius: isExpanded ? "26px" : transientNotif ? "28px" : "20px",
          borderColor:
            config.theme === "light"
              ? "rgba(148, 163, 184, 0.42)"
              : "rgba(255, 255, 255, 0.13)",
          backgroundColor: isExpanded
            ? "var(--island-panel)"
            : "var(--island-pill)",
          opacity: isClosing ? 0 : 1,
          transform: shellTransform,
          // @ts-expect-error CSS custom property
          "--surface-alpha": 0.94 + ((config.surfaceOpacity ?? 80) / 100) * 0.06,
          transition: prefersReducedMotion
            ? "none"
            : isClosing
            ? "transform 230ms cubic-bezier(0.32, 0, 0.67, 0), opacity 210ms ease-in"
            : isExpanded
            ? "width 330ms cubic-bezier(0.22, 1.26, 0.36, 1), height 360ms cubic-bezier(0.18, 1.16, 0.32, 1) 20ms, border-radius 300ms ease, border-color 260ms ease, background-color 260ms ease, transform 140ms ease-out, opacity 220ms ease-out"
            : "height 240ms cubic-bezier(0.32, 0.04, 0.24, 1), width 300ms cubic-bezier(0.22, 1.12, 0.36, 1) 18ms, border-radius 260ms ease, border-color 240ms ease, background-color 240ms ease, transform 260ms cubic-bezier(0.22, 1, 0.36, 1), opacity 220ms ease-out",
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
          {/* 3D Section Indicator Rail on the right edge (Matches reference screenshots) */}
          <div className="absolute right-2 top-1/2 -translate-y-1/2 flex flex-col items-center space-y-1.5 z-30">
            {SECTIONS.map((section, idx) => {
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
            {[0, 1, 2].map((sectionIdx) => {
              const offset = sectionIdx - activeSection;
              const isCurrent = offset === 0;
              return (
                <div
                  key={sectionIdx}
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
                  {sectionIdx === 0 && (
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
                      shelfItems={shelfItems}
                      showFileShelf={showFileShelf}
                      setShowFileShelf={setShowFileShelf}
                      onRefreshShelf={() => ipc.getShelfItems().then(setShelfItems)}
                    />
                  )}
                  {sectionIdx === 1 && (
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
                    />
                  )}
                  {sectionIdx === 2 && (
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
