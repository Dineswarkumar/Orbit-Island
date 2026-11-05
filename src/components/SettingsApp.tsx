import React, { useEffect, useState } from "react";
import { listen, emit } from "@tauri-apps/api/event";
import orbitIcon from "../../src-tauri/icons/128x128.png";
import {
  Save,
  RefreshCw,
  X,
  Check,
  Cpu,
  Bell,
  Shield,
  Sparkles,
  Type,
  HelpCircle,
  RotateCcw,
  FolderOpen,
  Keyboard,
  EyeOff,
  Video,
  Activity,
  Image as ImageIcon,
  Calendar,
  Upload,
  FileText,
  Trash2,
  GraduationCap,
  Briefcase,
  Palette,
  Music,
  ExternalLink,
  ShieldCheck,
} from "lucide-react";
import { useConfigStore } from "../core/stores/useConfigStore";
import { ipc } from "../core/ipc/client";
import { MemoryReport } from "../core/types/telemetry";
import { AppConfig, normalizeGlassPreset } from "../core/types/config";
import { HardwareMetrics } from "../core/types/hardware";
import { PrivacyState } from "../core/types/notifications";
import { THEMES, getThemeMeta } from "../core/theme";
import packageJson from "../../package.json";
import { GlassLayers } from "./island/GlassLayers";

const FONT_PRESETS: {
  id: AppConfig["fontFamily"];
  name: string;
  desc: string;
  preview: string;
  className: string;
}[] = [
  {
    id: "inter",
    name: "Modern UI",
    desc: "Clean geometric sans (Default)",
    preview: "12:30 PM · Focus Timer",
    className: "orbit-font-inter",
  },
  {
    id: "bold",
    name: "SF Pro Bold",
    desc: "High-impact bold typography",
    preview: "12:30 PM · Focus Timer",
    className: "orbit-font-bold",
  },
  {
    id: "claude",
    name: "Claude Editorial",
    desc: "Warm Anthropic book serif",
    preview: "12:30 PM · Focus Timer",
    className: "orbit-font-claude",
  },
  {
    id: "mono",
    name: "Developer Mono",
    desc: "Cascadia / JetBrains code precision",
    preview: "12:30 PM · Focus Timer",
    className: "orbit-font-mono",
  },
  {
    id: "rounded",
    name: "Soft Rounded",
    desc: "Friendly geometric curves",
    preview: "12:30 PM · Focus Timer",
    className: "orbit-font-rounded",
  },
  {
    id: "cyber",
    name: "Cyber HUD",
    desc: "Futuristic wide tech display",
    preview: "12:30 PM · Focus Timer",
    className: "orbit-font-cyber",
  },
];

const GLASS_STYLES: {
  id: "normal" | "realistic" | "blurred" | "opaque";
  mode: "glass" | "smoked" | "frosted" | "solid";
  opacity: 20 | 50 | 80 | 100;
  name: string;
  description: string;
  sample: string;
}[] = [
  {
    id: "normal",
    mode: "glass",
    opacity: 20,
    name: "Normal",
    description: "Clear surface · very light blur",
    sample: "from-cyan-100 via-white to-slate-200",
  },
  {
    id: "realistic",
    mode: "smoked",
    opacity: 50,
    name: "Realistic",
    description: "Soft depth · natural tint",
    sample: "from-sky-200 via-indigo-100 to-slate-300",
  },
  {
    id: "blurred",
    mode: "frosted",
    opacity: 80,
    name: "Blurred",
    description: "Frosted surface · stronger blur",
    sample: "from-slate-300 via-slate-100 to-cyan-100",
  },
  {
    id: "opaque",
    mode: "solid",
    opacity: 100,
    name: "Opaque",
    description: "No live capture or blur · lowest performance cost",
    sample: "from-slate-700 via-slate-800 to-slate-950",
  },
];

export const SettingsApp: React.FC = () => {
  const { config, loadConfig, updateConfig, isLoading, isSaving } =
    useConfigStore();
  const [formData, setFormData] = useState<AppConfig>(config);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">(
    "idle"
  );
  const [settingsAppearance, setSettingsAppearance] = useState<"system" | "light" | "dark" | "orbit">(() => {
    try {
      const saved = localStorage.getItem("orbit_settings_appearance");
      if (saved === "system" || saved === "light" || saved === "dark" || saved === "orbit") return saved;
      const legacy = localStorage.getItem("orbit_settings_ui_mode");
      return legacy === "light" ? "light" : "system";
    } catch {
      return "system";
    }
  });
  const [systemIsLight, setSystemIsLight] = useState(() =>
    typeof window !== "undefined" && window.matchMedia?.("(prefers-color-scheme: light)").matches === true
  );

  const [telemetry, setTelemetry] = useState<MemoryReport | null>(null);
  const [hardwareStatus, setHardwareStatus] = useState<HardwareMetrics | null>(null);
  const [telemetryError, setTelemetryError] = useState<string | null>(null);
  const [privacyState, setPrivacyState] = useState<PrivacyState | null>(null);
  const [isTestingPrivacy, setIsTestingPrivacy] = useState(false);
  const privacyTestUntilRef = React.useRef(0);
  const [notifAccessStatus, setNotifAccessStatus] =
    useState<string>("unspecified");
  const [isRequestingNotif, setIsRequestingNotif] = useState(false);
  const [clearNotifMessage, setClearNotifMessage] = useState<string | null>(
    null
  );
  const [resetMessage, setResetMessage] = useState<string | null>(null);
  const [logPathMessage, setLogPathMessage] = useState<string | null>(null);
  const [calendarImportMsg, setCalendarImportMsg] = useState<string | null>(null);
  const [isImportingCalendar, setIsImportingCalendar] = useState(false);
  const [showPrivacyTerms, setShowPrivacyTerms] = useState(false);
  const [destinationName, setDestinationName] = useState("");
  const [destinationUrl, setDestinationUrl] = useState("");

  const saveStatusTimer = React.useRef<ReturnType<typeof setTimeout> | null>(
    null
  );

  useEffect(() => {
    if (!window.matchMedia) return;
    const query = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = (event: MediaQueryListEvent) => setSystemIsLight(event.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  const selectSettingsAppearance = (appearance: "system" | "light" | "dark" | "orbit") => {
    setSettingsAppearance(appearance);
    try {
      localStorage.setItem("orbit_settings_appearance", appearance);
    } catch {}
  };

  useEffect(
    () => () => {
      if (saveStatusTimer.current) clearTimeout(saveStatusTimer.current);
    },
    []
  );

  useEffect(() => {
    loadConfig();
    ipc
      .getHardwareMetrics()
      .then(setHardwareStatus)
      .catch((err) => console.error("Failed to load hardware status in settings:", err));
    ipc
      .getMemoryMetrics()
      .then((report) => {
        setTelemetry(report);
        setTelemetryError(null);
      })
      .catch((err) => setTelemetryError(String(err)));

    ipc
      .getNotificationAccessStatus()
      .then((status) => setNotifAccessStatus(status))
      .catch((err) =>
        console.error("Failed to get notification access status:", err)
      );

    ipc
      .getPrivacyState()
      .then((priv) => setPrivacyState(priv))
      .catch((err) => console.error("Failed to load privacy state in settings:", err));

    const unlistenPrivacy = listen<PrivacyState>(
      "privacy-state-changed",
      (event) => {
        if (event.payload.cameraApp?.includes("(Test)") || event.payload.microphoneApp?.includes("(Test)")) {
          privacyTestUntilRef.current = Date.now() + 5000;
        } else if (Date.now() < privacyTestUntilRef.current) {
          return;
        }
        setPrivacyState(event.payload);
      }
    );

    const unlistenPrivacyTestFinished = listen<PrivacyState>(
      "privacy-test-finished",
      (event) => {
        privacyTestUntilRef.current = 0;
        setPrivacyState(event.payload);
      }
    );

    const unlistenConfig = listen<AppConfig>("config-changed", (event) => {
      const normalizedConfig = normalizeGlassPreset(event.payload);
      useConfigStore.setState({ config: normalizedConfig });
      setFormData(normalizedConfig);
    });

    return () => {
      unlistenConfig.then((unlisten) => unlisten());
      unlistenPrivacy.then((unlisten) => unlisten());
      unlistenPrivacyTestFinished.then((unlisten) => unlisten());
    };
  }, [loadConfig]);

  useEffect(() => {
    setFormData(config);
  }, [config]);

  const handleRequestNotifAccess = async () => {
    setIsRequestingNotif(true);
    try {
      const res = await ipc.requestNotificationAccess();
      setNotifAccessStatus(res);
    } catch (err) {
      console.error("Failed to request notification access:", err);
    } finally {
      setIsRequestingNotif(false);
    }
  };

  const handleClearNotifications = async () => {
    try {
      await ipc.clearAllNotifications();
      setClearNotifMessage("All notifications cleared from ORBIT.");
      setTimeout(() => setClearNotifMessage(null), 3000);
    } catch {
      setClearNotifMessage("Failed to clear notifications.");
      setTimeout(() => setClearNotifMessage(null), 3000);
    }
  };

  const handleTestPrivacy = async () => {
    if (isTestingPrivacy) return;
    setIsTestingPrivacy(true);
    try {
      await ipc.triggerPrivacyTest();
    } catch (err) {
      console.error("Failed to trigger privacy test:", err);
    } finally {
      setTimeout(() => {
        setIsTestingPrivacy(false);
      }, 4500);
    }
  };

  const handleCustomImageUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (event) => {
      const img = new Image();
      img.onload = () => {
        const canvas = document.createElement("canvas");
        canvas.width = 64;
        canvas.height = 64;
        const ctx = canvas.getContext("2d");
        if (ctx) {
          const minSide = Math.min(img.width, img.height);
          const sx = (img.width - minSide) / 2;
          const sy = (img.height - minSide) / 2;
          ctx.drawImage(img, sx, sy, minSide, minSide, 0, 0, 64, 64);
          const dataUrl = canvas.toDataURL("image/png");
          setFormData((prev) => ({
            ...prev,
            customRightImageBase64: dataUrl,
          }));
        }
      };
      if (typeof event.target?.result === "string") {
        img.src = event.target.result;
      }
    };
    reader.readAsDataURL(file);
  };

  const selectTheme = (themeId: AppConfig["theme"]) => {
    setFormData((prev) => {
      const next = { ...prev, theme: themeId };
      updateConfig(next).catch(() => {});
      return next;
    });
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (saveStatusTimer.current) clearTimeout(saveStatusTimer.current);
    const success = await updateConfig(formData);
    if (success) {
      setSaveStatus("success");
      saveStatusTimer.current = setTimeout(() => setSaveStatus("idle"), 2500);
    } else {
      setSaveStatus("error");
      saveStatusTimer.current = setTimeout(() => setSaveStatus("idle"), 3500);
    }
  };

  const handleClose = async () => {
    try {
      await ipc.closeSettingsWindow();
    } catch (err) {
      console.error(err);
    }
  };

  const handleResetDefaults = () => {
    setFormData({
      theme: "obsidian",
      position: "top-center",
      density: "balanced",
      timeFormat: "12h",
      alwaysOnTop: true,
      showSystemTrayIcon: true,
      launchAtStartup: false,
      showHardwareMonitor: true,
      transparencyMode: "glass",
      notificationsEnabled: true,
      privacyMode: false,
      notificationPreviewDurationSec: 4,
      showPrivacyIndicators: true,
      blockedApps: [],
      surfaceOpacity: 20,
      showWeather: false,
      weatherUnit: "C",
      adaptiveAlbumAccent: true,
      expandTrigger: "hover",
      pauseInFullscreen: true,
      shelfStorageMode: "reference",
      dragDropEnabled: true,
      fontFamily: "inter",
      onboardingCompleted: true,
      compactRightSlot: "weather",
      customRightText: "ORBIT",
      customRightTextStyle: "neon_glow",
      customRightImageBase64: "",
      hideFromScreenCapture: false,
      shortcutsEnabled: true,
      autoHideInMeetings: true,
      networkActivityMode: "speed_only",
      onlineWeatherEnabled: false,
      mediaBackgroundMode: "cover",
      mediaVisualizerMode: "spectrum",
      mediaShowCoverArt: true,
      mediaCompactMode: false,
      pinnedDestinations: [],
    });
    setResetMessage("Settings reset to defaults. Click 'Save Preferences' to persist.");
    setTimeout(() => setResetMessage(null), 4000);
  };

  const handleClearLocalData = async () => {
    try {
      await ipc.clearLocalData();
      localStorage.removeItem("orbit_shelf_snippets");
      sessionStorage.removeItem("orbit_shelf_temporary_snippets");
      setResetMessage("Local reminders, Shelf entries, and saved snippets cleared.");
      setTimeout(() => setResetMessage(null), 4000);
    } catch (err) {
      setResetMessage(`Failed to clear local data: ${err}`);
      setTimeout(() => setResetMessage(null), 4000);
    }
  };

  const handleImportCalendar = async (sourceTag: "academic" | "office" | "ics") => {
    setIsImportingCalendar(true);
    try {
      const count = await ipc.pickAndImportCalendar(sourceTag);
      if (count > 0) {
        setCalendarImportMsg(`✓ Successfully imported ${count} schedule event${count === 1 ? "" : "s"}!`);
      } else {
        setCalendarImportMsg("No events imported (dialog cancelled or empty file).");
      }
      setTimeout(() => setCalendarImportMsg(null), 5000);
    } catch (err) {
      setCalendarImportMsg(`Import failed: ${err}`);
      setTimeout(() => setCalendarImportMsg(null), 5000);
    } finally {
      setIsImportingCalendar(false);
    }
  };

  const handleClearImportedCalendar = async () => {
    try {
      const count = await ipc.clearImportedCalendar(null);
      setCalendarImportMsg(`✓ Cleared ${count} imported schedule event${count === 1 ? "" : "s"} (manual tasks preserved).`);
      setTimeout(() => setCalendarImportMsg(null), 5000);
    } catch (err) {
      setCalendarImportMsg(`Clear failed: ${err}`);
      setTimeout(() => setCalendarImportMsg(null), 5000);
    }
  };

  const copyCsvTemplate = () => {
    const sample = "Title,Date,Time,Category\nData Structures Final Exam,2026-11-15,09:30,Exam\nOperating Systems Lab,2026-11-18,14:00,Class\nSprint Review & Demo,2026-11-20,11:00,Meeting\nProject Milestone Submission,2026-11-22,23:59,Deadline\nThanksgiving Break,2026-11-26,09:00,Holiday\n";
    navigator.clipboard.writeText(sample);
    setCalendarImportMsg("✓ Copied sample schedule CSV template to clipboard!");
    setTimeout(() => setCalendarImportMsg(null), 4000);
  };

  const handleFullReset = async () => {
    try {
      await ipc.clearLocalData();
      localStorage.removeItem("orbit_onboarding_seen");
      localStorage.removeItem("orbit_timer_state");
      localStorage.removeItem("orbit_weather_cache");
      localStorage.removeItem("orbit_shelf_snippets");
      sessionStorage.removeItem("orbit_shelf_temporary_snippets");
      const defaultCfg: AppConfig = {
        theme: "obsidian",
        position: "top-center",
        density: "balanced",
        timeFormat: "12h",
        alwaysOnTop: true,
        showSystemTrayIcon: true,
        launchAtStartup: false,
        showHardwareMonitor: true,
        transparencyMode: "glass",
        notificationsEnabled: true,
        privacyMode: false,
        notificationPreviewDurationSec: 4,
        showPrivacyIndicators: true,
        blockedApps: [],
        surfaceOpacity: 20,
        showWeather: false,
        weatherUnit: "C",
        adaptiveAlbumAccent: true,
        expandTrigger: "hover",
        pauseInFullscreen: true,
        shelfStorageMode: "reference",
        dragDropEnabled: true,
        fontFamily: "inter",
        onboardingCompleted: false,
        compactRightSlot: "weather",
        customRightText: "ORBIT",
        customRightTextStyle: "neon_glow",
        customRightImageBase64: "",
        hideFromScreenCapture: false,
        shortcutsEnabled: true,
        autoHideInMeetings: true,
        networkActivityMode: "speed_only",
        onlineWeatherEnabled: false,
        mediaBackgroundMode: "cover",
        mediaVisualizerMode: "spectrum",
        mediaShowCoverArt: true,
        mediaCompactMode: false,
        pinnedDestinations: [],
      };
      setFormData(defaultCfg);
      await updateConfig(defaultCfg);
      await emit("replay-onboarding", {});
      setResetMessage("Full reset complete! Data cleared & onboarding tour started.");
      setTimeout(() => setResetMessage(null), 5000);
    } catch (err) {
      setResetMessage(`Reset failed: ${err}`);
      setTimeout(() => setResetMessage(null), 4000);
    }
  };

  const handleReplayTour = async () => {
    try {
      localStorage.removeItem("orbit_onboarding_seen");
      setFormData((prev) => ({ ...prev, onboardingCompleted: false }));
      await updateConfig({ onboardingCompleted: false });
      await emit("replay-onboarding", {});
      setResetMessage("Guided tour started! Look at the ORBIT Island on your screen.");
      setTimeout(() => setResetMessage(null), 5000);
    } catch (err) {
      console.error(err);
    }
  };

  const handleRevealLogs = async () => {
    try {
      const path = await ipc.revealDiagnosticLog();
      setLogPathMessage(`Log file revealed: ${path}`);
      setTimeout(() => setLogPathMessage(null), 6000);
    } catch (err) {
      setLogPathMessage(`Failed to reveal log: ${err}`);
      setTimeout(() => setLogPathMessage(null), 4000);
    }
  };

  const handleRefreshMemory = async () => {
    try {
      const report = await ipc.getMemoryMetrics();
      setTelemetry(report);
      setTelemetryError(null);
    } catch (err) {
      setTelemetryError(String(err));
    }
  };

  // Settings can follow Windows, use a neutral light/dark palette, or mirror ORBIT's theme.
  const orbitThemeId = config.theme === "system" ? (systemIsLight ? "light" : "black") : config.theme;
  const isOrbitTheme = settingsAppearance === "orbit";
  const isLight = settingsAppearance === "light" ||
    (settingsAppearance === "system" && systemIsLight) ||
    (isOrbitTheme && orbitThemeId === "light");
  const settingsThemeMeta = getThemeMeta(
    isOrbitTheme ? orbitThemeId : isLight ? "light" : "black"
  );
  const settingsThemeStyle = {
    backgroundColor: isOrbitTheme ? `rgb(${settingsThemeMeta.surfaceBase})` : isLight ? "#f1f5f9" : "#090d16",
    color: isOrbitTheme ? settingsThemeMeta.text : isLight ? "#0f172a" : "#f1f5f9",
    "--settings-theme-canvas": `rgb(${settingsThemeMeta.surfaceBase})`,
    "--settings-theme-header-bg": isOrbitTheme ? `rgba(${settingsThemeMeta.surfaceBase}, 0.98)` : undefined,
    "--settings-theme-card-bg": settingsThemeMeta.surfaceElevated,
    "--settings-theme-input-bg": `rgba(${settingsThemeMeta.surfaceBase}, 0.88)`,
    "--settings-theme-border": settingsThemeMeta.surfaceCardBorder,
    "--settings-theme-text": settingsThemeMeta.text,
    "--settings-theme-muted": settingsThemeMeta.muted,
    "--settings-theme-accent": settingsThemeMeta.accent,
  } as React.CSSProperties;
  const canvasBg = isOrbitTheme ? "settings-orbit-canvas" : isLight ? "bg-slate-100 text-slate-900" : "bg-[#090d16] text-slate-100";
  const headerBg = isOrbitTheme ? "settings-orbit-header" : isLight ? "bg-white/95 border-slate-200" : "bg-slate-950/95 border-slate-800";
  const cardBg = isOrbitTheme
    ? "settings-orbit-card"
    : isLight
    ? "bg-white border-slate-200/90 shadow-sm"
    : "bg-slate-900/75 border-slate-800/90 shadow-md";
  const cardHeaderColor = isOrbitTheme ? "settings-orbit-muted" : isLight ? "text-slate-600" : "text-slate-400";
  const labelColor = isOrbitTheme ? "settings-orbit-text" : isLight ? "text-slate-700" : "text-slate-300";
  const subtextColor = isOrbitTheme ? "settings-orbit-muted" : isLight ? "text-slate-500" : "text-slate-400";
  const glassFallbackReasons = [
    hardwareStatus?.batterySaverActive ? "Battery Saver is on" : null,
    hardwareStatus?.osTransparencyEnabled === false ? "Windows transparency effects are off" : null,
  ].filter((reason): reason is string => reason !== null);
  const inputBg = isOrbitTheme
    ? "settings-orbit-input"
    : isLight
    ? "bg-slate-50 border-slate-300 text-slate-900 focus:border-cyan-600"
    : "bg-slate-900/90 border-slate-700/80 text-slate-100 focus:border-cyan-400";
  const optionCls = isOrbitTheme ? "settings-orbit-input" : isLight ? "bg-white text-slate-900" : "bg-slate-900 text-slate-100";

  return (
    <div
      style={settingsThemeStyle}
      className={`h-full w-full flex flex-col font-sans select-none overflow-hidden transition-colors duration-200 ${canvasBg} ${isOrbitTheme ? `settings-orbit-theme theme-${orbitThemeId}` : ""}`}
    >
      {/* Pinned Header */}
      <div
        className={`shrink-0 flex items-center justify-between px-6 py-4 border-b backdrop-blur z-20 transition-colors duration-200 ${headerBg}`}
      >
        <div className="flex items-center space-x-3">
          <div
            className={`w-9 h-9 rounded-xl flex items-center justify-center shadow-sm ${
              isLight
                ? "bg-cyan-50 border border-cyan-200 text-cyan-600"
                : "bg-cyan-600/20 border border-cyan-500/30 text-cyan-400 shadow-[0_0_12px_rgba(6,182,212,0.25)]"
            }`}
          >
            <img src={orbitIcon} alt="" className="w-8 h-8 rounded-lg object-cover" />
          </div>
          <div>
            <div className="flex items-center space-x-2">
              <h1 className="text-base font-bold tracking-tight">ORBIT Settings</h1>
              <span
                className={`text-[10.5px] font-mono font-bold px-2 py-0.5 rounded-full border ${
                  isLight
                    ? "bg-cyan-50 text-cyan-700 border-cyan-200"
                    : "bg-cyan-500/15 text-cyan-300 border-cyan-500/30"
                }`}
              >
                v{packageJson.version}
              </span>
            </div>
            <p className={`text-[11px] ${subtextColor}`}>
              3D Orbital Shell, Typography Presets & System Diagnostics
            </p>
          </div>
        </div>

        <div className="flex items-center space-x-2">
          <label className="flex items-center space-x-2">
            <span className={`text-[10px] font-semibold ${subtextColor}`}>Appearance</span>
            <select
              aria-label="Settings appearance"
              value={settingsAppearance}
              onChange={(event) => selectSettingsAppearance(event.target.value as typeof settingsAppearance)}
              className={`px-2.5 py-1.5 rounded-lg border text-xs font-semibold focus:outline-none focus-visible:ring-2 focus-visible:ring-cyan-500 ${
                isOrbitTheme ? "settings-orbit-input" : isLight ? "bg-white border-slate-300 text-slate-800" : "bg-slate-900 border-slate-700 text-slate-100"
              }`}
            >
              <option value="system">System</option>
              <option value="light">Light</option>
              <option value="dark">Dark</option>
              <option value="orbit">ORBIT theme</option>
            </select>
          </label>

          {/* Close Window Button */}
          <button
            type="button"
            onClick={handleClose}
            className={`p-1.5 rounded-lg transition-colors cursor-pointer ${
              isLight
                ? "text-slate-400 hover:text-slate-900 hover:bg-slate-200"
                : "text-slate-400 hover:text-white hover:bg-slate-800"
            }`}
            title="Close Settings Window"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Responsive settings form */}
      <form
        id="settings-form"
        aria-label="Settings sections"
        onSubmit={handleSave}
        className="flex-1 min-h-0 overflow-y-auto px-4 sm:px-6 py-5 focus:outline-none"
      >
        <div className="max-w-7xl mx-auto w-full">
          <nav aria-label="Settings sections" className={`sticky top-0 z-10 mb-4 flex flex-wrap gap-2 rounded-2xl border p-2 backdrop-blur-xl ${cardBg}`}>
            {[
              { id: "appearance", label: "Appearance", Icon: Palette },
              { id: "media", label: "Media", Icon: Music },
              { id: "privacy", label: "Privacy & notifications", Icon: ShieldCheck },
              { id: "performance", label: "Performance", Icon: Activity },
              { id: "about", label: "About & privacy", Icon: HelpCircle },
            ].map(({ id, label, Icon }) => (
              <button key={id} type="button" aria-label={label} title={label} onClick={() => document.getElementById(`settings-${id}`)?.scrollIntoView({ behavior: "smooth", block: "start" })} className={`rounded-xl p-2.5 transition-colors ${isLight ? "text-slate-600 hover:bg-cyan-50 hover:text-cyan-800" : "text-slate-300 hover:bg-white/10 hover:text-white"}`}>
                <Icon className="h-4 w-4" aria-hidden="true" />
              </button>
            ))}
          </nav>
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-2 items-start gap-4 xl:gap-5">
          {/* Bento Card 0: 3D Curated Theme Showcase Gallery (Full Width) */}
          <div id="settings-appearance" className={`scroll-mt-20 rounded-2xl p-5 border space-y-4 md:col-span-2 xl:col-span-2 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Palette className="w-4 h-4 text-cyan-500" />
                <span>Curated 3D Theme Showcase</span>
              </h2>
              <span className="text-[10px] text-cyan-500 bg-cyan-500/10 border border-cyan-500/25 px-2 py-0.5 rounded font-mono font-semibold">
                {THEMES.length} Signature Styles
              </span>
            </div>

            <p className={`text-xs ${subtextColor}`}>
              Crafted with authentic desktop glassmorphism, calibrated surface depth, specular rim lighting, and fluid Apple HIG physics. Changes apply instantly to the floating Island overlay.
            </p>

            {/* Responsive Grid of Curated Theme Cards */}
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3 pt-1">
              {THEMES.map((t) => {
                const isSelected = formData.theme === t.id;
                return (
                  <button
                    type="button"
                    key={t.id}
                    onClick={() => selectTheme(t.id)}
                    aria-pressed={isSelected}
                    className={`relative p-3 rounded-xl border cursor-pointer transition-all flex flex-col justify-between select-none text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-500 focus-visible:ring-offset-2 ${
                      isSelected
                        ? "ring-2 ring-cyan-500 bg-cyan-500/10 border-cyan-500/40 shadow-lg scale-[1.01]"
                        : isLight
                        ? "bg-slate-50 hover:bg-slate-100 border-slate-200"
                        : "bg-black/30 hover:bg-white/5 border-white/10"
                    }`}
                  >
                    {/* Top Row: Mini Island Capsule Preview */}
                    <div
                      className="w-full h-8 rounded-full border px-2.5 flex items-center justify-between mb-2.5 overflow-hidden transition-all"
                      style={{
                        backgroundColor: `rgba(${t.surfaceBase}, 0.96)`,
                        borderColor: t.borderGlow,
                        boxShadow: `inset 0 1px 1px ${t.rimHighlight}, 0 2px 8px -1px rgba(0,0,0,0.4)`,
                      }}
                    >
                      <div className="flex items-center space-x-1 min-w-0">
                        <span
                          className="w-2 h-2 rounded-full shrink-0"
                          style={{ backgroundColor: t.accent }}
                        />
                        <span
                          className="text-[9.5px] font-bold truncate tabular-nums"
                          style={{ color: t.text }}
                        >
                          9:41
                        </span>
                      </div>
                      <div className="flex items-center space-x-1 shrink-0">
                        <span
                          className="w-1.5 h-1.5 rounded-full"
                          style={{ backgroundColor: t.accentSecondary }}
                        />
                        <span
                          className="text-[8.5px] font-semibold px-1.5 py-0.2 rounded-full"
                          style={{
                            backgroundColor: t.surfaceElevated,
                            color: t.accent,
                          }}
                        >
                          Island
                        </span>
                      </div>
                    </div>

                    {/* Theme Meta Info */}
                    <div className="min-w-0">
                      <div className="flex items-center justify-between">
                        <span className={`text-xs font-bold truncate ${labelColor}`}>
                          {t.label}
                        </span>
                        {isSelected && (
                          <span className="w-2 h-2 rounded-full bg-cyan-400 shadow-[0_0_6px_#06b6d4] animate-pulse shrink-0 ml-1" />
                        )}
                      </div>
                      <p className="text-[10px] text-slate-400 line-clamp-2 mt-0.5 leading-snug">
                        {t.tagline}
                      </p>
                    </div>

                    {/* Bottom Swatches Row */}
                    <div className="flex items-center justify-between pt-2 mt-2 border-t border-white/5">
                      <div className="flex items-center space-x-1.5">
                        {t.swatches.map((color, sIdx) => (
                          <span
                            key={sIdx}
                            className="w-3.5 h-3.5 rounded-full border border-white/20 shadow-sm"
                            style={{ backgroundColor: color }}
                            title={`Palette tone ${sIdx + 1}: ${color}`}
                          />
                        ))}
                      </div>
                      <span
                        className={`text-[9px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded ${
                          isSelected
                            ? "bg-cyan-500/20 text-cyan-300 font-extrabold"
                            : isLight
                            ? "bg-slate-200 text-slate-600"
                            : "bg-white/10 text-white/60"
                        }`}
                      >
                        {isSelected ? "Active" : "Select"}
                      </span>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Bento Card 1: Glass Style & Placement */}
          <div id="settings-placement" className={`scroll-mt-20 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Sparkles className="w-4 h-4 text-cyan-500" />
                <span>3D Glass & Placement</span>
              </h2>
              <span className="text-[10px] text-cyan-500 bg-cyan-500/10 border border-cyan-500/25 px-2 py-0.5 rounded font-mono font-semibold">
                Island style
              </span>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
              <div>
                <label className={`block text-xs font-semibold mb-1 ${labelColor}`}>
                  Screen Position
                </label>
                <select
                  value={formData.position}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      position: e.target.value as AppConfig["position"],
                    })
                  }
                  className={`w-full rounded-xl px-3 py-2 text-xs border focus:outline-none cursor-pointer ${inputBg}`}
                >
                  <option value="top-center" className={optionCls}>Top Center (Default)</option>
                  <option value="top-left" className={optionCls}>Top Left</option>
                  <option value="top-right" className={optionCls}>Top Right</option>
                </select>
              </div>

              <div>
                <label className={`block text-xs font-semibold mb-1 ${labelColor}`}>
                  Time Format
                </label>
                <select
                  value={formData.timeFormat}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      timeFormat: e.target.value as AppConfig["timeFormat"],
                    })
                  }
                  className={`w-full rounded-xl px-3 py-2 text-xs border focus:outline-none cursor-pointer ${inputBg}`}
                >
                  <option value="12h" className={optionCls}>12-Hour (AM/PM)</option>
                  <option value="24h" className={optionCls}>24-Hour</option>
                </select>
              </div>

              <div className="sm:col-span-2 space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <label className={`block text-xs font-semibold ${labelColor}`}>
                    Glass style
                  </label>
                  <span className={`text-[10px] ${subtextColor}`}>
                    Choose how much of the desktop shows through
                  </span>
                </div>
                <div role="radiogroup" aria-label="Glass style" className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-2">
                  {GLASS_STYLES.map((style) => {
                    const isSelected = formData.transparencyMode === style.mode
                      && (formData.surfaceOpacity ?? 20) === style.opacity;
                    return (
                      <button
                        key={style.id}
                        type="button"
                        role="radio"
                        aria-checked={isSelected}
                        onClick={() => setFormData((prev) => ({
                          ...prev,
                          transparencyMode: style.mode,
                          surfaceOpacity: style.opacity,
                        }))}
                        className={`rounded-xl border p-2 text-left transition-all cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-cyan-500 ${
                          isSelected
                            ? "border-cyan-400 bg-cyan-500/10 shadow-[0_0_0_1px_rgba(34,211,238,0.22)]"
                            : isLight
                            ? "border-slate-200 bg-white/70 hover:border-cyan-300 hover:bg-cyan-50/60"
                            : "border-slate-700 bg-slate-950/40 hover:border-cyan-700 hover:bg-cyan-950/20"
                        }`}
                      >
                        <span className={`relative mb-2 block h-9 overflow-hidden rounded-lg border ${isLight ? "border-slate-200" : "border-white/10"}`}>
                          <span className={`absolute inset-0 bg-gradient-to-br ${style.sample}`} />
                          <span className={`absolute inset-x-3 top-2 h-5 rounded-full border shadow-sm ${style.mode === "solid" ? "border-white/30 bg-slate-900" : `backdrop-blur-[2px] ${isLight ? "border-white/90 bg-white/45" : "border-white/35 bg-slate-900/45"}`}`} />
                          <span className="absolute left-1.5 top-1.5 h-1.5 w-1.5 rounded-full bg-cyan-400 shadow-[0_0_6px_rgba(34,211,238,0.8)]" />
                        </span>
                        <span className={`block text-xs font-bold ${labelColor}`}>{style.name}</span>
                        <span className={`mt-0.5 block text-[10px] leading-snug ${subtextColor}`}>{style.description}</span>
                      </button>
                    );
                  })}
                </div>
              </div>

              <p className={`sm:col-span-2 rounded-xl border px-3 py-2 text-[11px] leading-relaxed ${isLight ? "border-cyan-200 bg-cyan-50 text-slate-700" : "border-cyan-900/70 bg-cyan-950/30 text-slate-300"}`}>
                Performance: Opaque is best for low-end PCs. Normal uses the least live glass processing; Realistic and Blurred use more desktop rendering, with Blurred the heaviest.
              </p>

              {glassFallbackReasons.length > 0 && (
                <p
                  role="status"
                  className={`sm:col-span-2 rounded-lg border px-3 py-2 text-[11px] leading-relaxed ${
                    isLight
                      ? "border-amber-300 bg-amber-50 text-amber-900"
                      : "border-amber-500/25 bg-amber-500/10 text-amber-200"
                  }`}
                >
                  Glass is temporarily disabled on the island because {glassFallbackReasons.join(" and ")}. Turn those off to use the selected material.
                </p>
              )}

              {/* Live 3D Theme & Volumetric Preview Card */}
              <div className="sm:col-span-2">
                {(() => {
                  const previewMeta = getThemeMeta(formData.theme);
                  const opacity = (formData.surfaceOpacity ?? 20) / 100;
                  const modeClass = `transparency-${formData.transparencyMode}`;
                  const themeClass = `theme-${previewMeta.id}`;
                  return (
                    <div
                      className={`rounded-2xl p-3.5 border space-y-2.5 ${
                        isLight ? "bg-slate-50 border-slate-200" : "bg-slate-950/80 border-slate-800"
                      }`}
                    >
                      <div className="flex items-center justify-between text-[11px]">
                        <span className="font-bold flex items-center space-x-2">
                          <span
                            className="w-2.5 h-2.5 rounded-full inline-block"
                            style={{
                              backgroundColor: previewMeta.accent,
                              boxShadow: `0 0 8px ${previewMeta.accent}`,
                            }}
                          />
                          <span>{previewMeta.label}</span>
                        </span>
                        <span className="font-mono text-[10px] opacity-60">
                          {GLASS_STYLES.find((style) => style.mode === formData.transparencyMode)?.name ?? "Glass"}
                        </span>
                      </div>

                      {/* Split Light & Dark Desktop Simulation Backdrop */}
                      <div className="relative rounded-2xl overflow-hidden p-4 border border-white/10 shadow-inner flex items-center justify-center min-h-[116px]">
                        {/* Light Half (Day Desktop) */}
                        <div className="absolute inset-y-0 left-0 w-1/2 bg-gradient-to-br from-sky-100 via-white to-slate-200 flex items-start justify-start p-2">
                          <span className="text-[9px] font-bold uppercase tracking-wider text-slate-600 bg-white/80 backdrop-blur-xs px-1.5 py-0.5 rounded shadow-xs">
                            Light Desktop
                          </span>
                        </div>
                        {/* Dark Half (Night Desktop) */}
                        <div className="absolute inset-y-0 right-0 w-1/2 bg-gradient-to-br from-slate-950 via-slate-900 to-indigo-950 flex items-start justify-end p-2 border-l border-white/20">
                          <span className="text-[9px] font-bold uppercase tracking-wider text-slate-300 bg-slate-900/80 backdrop-blur-xs px-1.5 py-0.5 rounded shadow-xs">
                            Dark Desktop
                          </span>
                        </div>

                        {/* Centered Dynamic Island Glass Capsule bridging both backdrops */}
                        <div className="orbit-stage-3d relative z-10 w-full max-w-[340px]">
                          <div
                            className={`orbit-shell-3d ${modeClass} ${themeClass} ${
                              formData.transparencyMode === "solid"
                                ? ""
                                : previewMeta.id === "light"
                                ? "glass-tone-light"
                                : "glass-tone-dark"
                            } rounded-[24px] px-4 py-2.5 border flex items-center justify-between transition-all duration-300 relative overflow-hidden`}
                            style={
                              {
                                "--surface-alpha": opacity,
                                "--theme-accent": previewMeta.accent,
                                "--theme-accent-secondary": previewMeta.accentSecondary,
                              } as React.CSSProperties
                            }
                          >
                            {/* Full liquid-glass stack; the preview window is opaque, so it can blur natively */}
                            {formData.transparencyMode !== "solid" && (
                              <GlassLayers
                                mode={formData.transparencyMode as "glass" | "frosted" | "smoked"}
                                tone={previewMeta.id === "light" ? "light" : "dark"}
                                width={340}
                                height={46}
                                backdropUrl={null}
                                offsetX={0}
                                offsetY={0}
                                nativeBackdrop
                              />
                            )}

                            <div className="relative z-10 flex items-baseline space-x-2">
                              <span className="orbit-clock-3d text-lg font-bold tracking-tight tabular-nums">
                                10:08
                              </span>
                              <span
                                className="text-xs font-bold uppercase tracking-wider"
                                style={{ color: previewMeta.accent }}
                              >
                                AM
                              </span>
                            </div>

                            <div className="relative z-10 flex items-center space-x-2">
                              <div className="orbit-pill-badge-3d px-2 py-0.5 rounded-full text-[10px] font-semibold flex items-center space-x-1">
                                <span
                                  className="w-1.5 h-1.5 rounded-full"
                                  style={{ backgroundColor: previewMeta.accent }}
                                />
                                <span>Preview</span>
                              </div>
                              <span className="text-[11px] font-medium truncate max-w-[110px] opacity-80">
                                {`${formData.surfaceOpacity ?? 20}% surface`}
                              </span>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>
                  );
                })()}
              </div>
            </div>

            {/* Quick Feature Toggles */}
            <div className="pt-2 flex flex-col space-y-3 border-t border-white/10">
              {/* Weather Toggle */}
              <div className="flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Show Weather in Compact Pill
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Live temperature and condition icon alongside clock
                  </p>
                </div>
                <div className="flex items-center space-x-2.5">
                  <div className="flex items-center space-x-1 p-0.5 rounded-lg border bg-black/10">
                    <button
                      type="button"
                      onClick={() => setFormData({ ...formData, weatherUnit: "C" })}
                      className={`px-2 py-0.5 text-xs font-bold rounded-md transition-all cursor-pointer ${
                        formData.weatherUnit === "C"
                          ? "bg-cyan-500 text-slate-950 shadow-sm"
                          : "opacity-60 hover:opacity-100"
                      }`}
                    >
                      °C
                    </button>
                    <button
                      type="button"
                      onClick={() => setFormData({ ...formData, weatherUnit: "F" })}
                      className={`px-2 py-0.5 text-xs font-bold rounded-md transition-all cursor-pointer ${
                        formData.weatherUnit === "F"
                          ? "bg-cyan-500 text-slate-950 shadow-sm"
                          : "opacity-60 hover:opacity-100"
                      }`}
                    >
                      °F
                    </button>
                  </div>
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={formData.showWeather}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          showWeather: e.target.checked,
                        })
                      }
                      className="sr-only peer"
                    />
                    <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-600"></div>
                  </label>
                </div>
              </div>

              <label className={`flex items-start gap-3 rounded-xl border p-3 text-xs ${isLight ? "border-amber-200 bg-amber-50/70" : "border-amber-500/20 bg-amber-500/5"}`}>
                <input
                  type="checkbox"
                  checked={formData.onlineWeatherEnabled}
                  onChange={(e) => setFormData({ ...formData, onlineWeatherEnabled: e.target.checked })}
                  className="mt-0.5 accent-cyan-500"
                />
                <span>
                  <span className={`block font-semibold ${labelColor}`}>Allow online weather (optional)</span>
                  <span className={`mt-0.5 block text-[11px] leading-relaxed ${subtextColor}`}>
                    Weather is requested from wttr.in. That service receives the request and can see your network IP. Turn this off to keep ORBIT’s features offline.
                  </span>
                </span>
              </label>

              {/* Always on top */}
              <div className="flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Always on Top
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Keep island above standard desktop apps
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.alwaysOnTop}
                    onChange={(e) =>
                      setFormData({ ...formData, alwaysOnTop: e.target.checked })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-600"></div>
                </label>
              </div>

              {/* System tray icon */}
              <div className="flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Show System Tray Icon
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Keep ORBIT in the Windows notification area. You can turn it back on here.
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.showSystemTrayIcon}
                    onChange={(e) =>
                      setFormData({ ...formData, showSystemTrayIcon: e.target.checked })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-600"></div>
                </label>
              </div>

              {/* Expand on Click Only */}
              <div className="flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Expand on Click Only
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Expand only on mouse click instead of hover
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.expandTrigger === "click"}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        expandTrigger: e.target.checked ? "click" : "hover",
                      })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-600"></div>
                </label>
              </div>

              {/* Auto-Hide in Fullscreen Video */}
              <div className="flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Auto-Hide in Fullscreen Video
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Hide island during YouTube / VLC / Netflix fullscreen video
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.pauseInFullscreen ?? true}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        pauseInFullscreen: e.target.checked,
                      })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-600"></div>
                </label>
              </div>

              {/* Compact Right Slot Customizer (Sub-Phase 9.1) */}
              <div className="pt-3 border-t border-white/10 space-y-3">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Compact Right Slot Content
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Icon, custom badge, avatar, or weather displayed on the right edge of the compact capsule
                  </p>
                </div>

                <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                  {[
                    { id: "weather", label: "Weather", desc: "Live Temp / Sky" },
                    { id: "orbit_logo", label: "3D ORBIT", desc: "Orbital Core" },
                    { id: "windows_logo", label: "Windows", desc: "Fluent 4-Tile" },
                    { id: "apple_logo", label: "Apple", desc: "Silver Silhouette" },
                    { id: "linux_logo", label: "Linux", desc: "Tux Penguin" },
                    { id: "custom_text", label: "Glowing Text", desc: "1-5 Letter Badge" },
                    { id: "custom_image", label: "Avatar Image", desc: "Circular Photo" },
                    { id: "smart", label: "Smart Auto", desc: "Weather or Logo" },
                  ].map((opt) => (
                    <button
                      key={opt.id}
                      type="button"
                      onClick={() =>
                        setFormData({
                          ...formData,
                          compactRightSlot: opt.id as AppConfig["compactRightSlot"],
                        })
                      }
                      className={`p-2 rounded-xl text-left border transition-all cursor-pointer flex flex-col justify-between ${
                        formData.compactRightSlot === opt.id
                          ? isLight
                            ? "bg-cyan-50 border-cyan-500 shadow-sm"
                            : "bg-cyan-500/15 border-cyan-500/50 shadow-[0_0_12px_rgba(6,182,212,0.18)]"
                          : isLight
                          ? "bg-slate-50 border-slate-200 hover:bg-slate-100"
                          : "bg-slate-800/60 border-slate-700/60 hover:bg-slate-800"
                      }`}
                    >
                      <span
                        className={`text-[11.5px] font-bold ${
                          formData.compactRightSlot === opt.id
                            ? isLight
                              ? "text-cyan-700"
                              : "text-cyan-300"
                            : labelColor
                        }`}
                      >
                        {opt.label}
                      </span>
                      <span className={`text-[9.5px] mt-0.5 ${subtextColor}`}>
                        {opt.desc}
                      </span>
                    </button>
                  ))}
                </div>

                {/* Sub-controls when custom_text is active */}
                {formData.compactRightSlot === "custom_text" && (
                  <div
                    className={`p-3 rounded-xl border space-y-3 ${
                      isLight
                        ? "bg-slate-50 border-slate-200"
                        : "bg-slate-900/80 border-slate-800"
                    }`}
                  >
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <span className={`text-xs font-semibold ${labelColor}`}>
                          Badge Text (Max 5 Chars)
                        </span>
                        <p className={`text-[10px] ${subtextColor}`}>
                          Uppercase tag (e.g. ORBIT, PRO, DEV, AI, VIP)
                        </p>
                      </div>
                      <input
                        type="text"
                        maxLength={5}
                        value={formData.customRightText || ""}
                        onChange={(e) =>
                          setFormData({
                            ...formData,
                            customRightText: e.target.value
                              .slice(0, 5)
                              .toUpperCase(),
                          })
                        }
                        placeholder="ORBIT"
                        className={`w-24 rounded-lg px-2.5 py-1 text-xs font-mono font-bold tracking-wider text-center border uppercase focus:outline-none ${inputBg}`}
                      />
                    </div>

                    <div className="space-y-1.5">
                      <span className={`text-[11px] font-semibold ${labelColor}`}>
                        Glowing 3D Badge Style
                      </span>
                      <div className="grid grid-cols-2 gap-2">
                        {[
                          {
                            id: "neon_glow",
                            label: "Neon Glow",
                            cls: "orbit-right-badge-neon_glow",
                          },
                          {
                            id: "cyber_badge",
                            label: "Cyber Badge",
                            cls: "orbit-right-badge-cyber_badge",
                          },
                          {
                            id: "aurora_pill",
                            label: "Aurora Pill",
                            cls: "orbit-right-badge-aurora_pill",
                          },
                          {
                            id: "liquid_gold",
                            label: "Liquid Gold",
                            cls: "orbit-right-badge-liquid_gold",
                          },
                        ].map((st) => (
                          <button
                            key={st.id}
                            type="button"
                            onClick={() =>
                              setFormData({
                                ...formData,
                                customRightTextStyle:
                                  st.id as AppConfig["customRightTextStyle"],
                              })
                            }
                            className={`p-2 rounded-lg border text-left flex items-center justify-between cursor-pointer transition-all ${
                              formData.customRightTextStyle === st.id
                                ? isLight
                                  ? "bg-cyan-50 border-cyan-500"
                                  : "bg-cyan-500/15 border-cyan-500/50"
                                : isLight
                                ? "bg-white border-slate-200 hover:bg-slate-100"
                                : "bg-slate-800/60 border-slate-700/60 hover:bg-slate-800"
                            }`}
                          >
                            <span
                              className={`text-[10.5px] font-semibold ${labelColor}`}
                            >
                              {st.label}
                            </span>
                            <span
                              className={`${st.cls} text-[9px] font-mono font-bold px-1.5 py-0.5 rounded tracking-wider`}
                            >
                              {(formData.customRightText || "ORBIT")
                                .slice(0, 5)
                                .toUpperCase()}
                            </span>
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                )}

                {/* Sub-controls when custom_image is active */}
                {formData.compactRightSlot === "custom_image" && (
                  <div
                    className={`p-3 rounded-xl border flex items-center justify-between gap-3 ${
                      isLight
                        ? "bg-slate-50 border-slate-200"
                        : "bg-slate-900/80 border-slate-800"
                    }`}
                  >
                    <div className="flex items-center space-x-3">
                      {formData.customRightImageBase64 ? (
                        <div className="relative w-9 h-9 rounded-full overflow-hidden border-2 border-white/20 shadow-md shrink-0">
                          <img
                            src={formData.customRightImageBase64}
                            alt="Custom compact avatar"
                            className="w-full h-full object-cover"
                          />
                        </div>
                      ) : (
                        <div
                          className={`w-9 h-9 rounded-full flex items-center justify-center border border-dashed shrink-0 ${
                            isLight
                              ? "border-slate-300 text-slate-400"
                              : "border-slate-700 text-slate-500"
                          }`}
                        >
                          <ImageIcon className="w-4 h-4" />
                        </div>
                      )}
                      <div>
                        <span className={`text-xs font-semibold ${labelColor}`}>
                          Avatar Photo
                        </span>
                        <p className={`text-[10px] ${subtextColor}`}>
                          Auto-scaled to 64x64 circular avatar
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2">
                      <label
                        className={`px-2.5 py-1 rounded-lg text-xs font-semibold border cursor-pointer transition-all ${
                          isLight
                            ? "bg-white hover:bg-slate-100 border-slate-300 text-slate-700"
                            : "bg-slate-800 hover:bg-slate-700 border-slate-700 text-slate-200"
                        }`}
                      >
                        <span>Browse...</span>
                        <input
                          type="file"
                          accept="image/*"
                          onChange={handleCustomImageUpload}
                          className="sr-only"
                        />
                      </label>
                      {formData.customRightImageBase64 && (
                        <button
                          type="button"
                          onClick={() =>
                            setFormData({
                              ...formData,
                              customRightImageBase64: "",
                            })
                          }
                          className="px-2 py-1 rounded-lg text-xs font-semibold text-rose-400 hover:text-rose-300 hover:bg-rose-500/10 transition-colors cursor-pointer"
                        >
                          Remove
                        </button>
                      )}
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Dedicated media player controls */}
          <div id="settings-media" className={`scroll-mt-20 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center gap-1.5 ${cardHeaderColor}`}>
                <Music className="w-4 h-4 text-blue-400" />
                <span>Media Player</span>
              </h2>
              <span className="text-[10px] text-blue-400 bg-blue-500/10 border border-blue-500/25 px-2 py-0.5 rounded font-mono font-semibold">Personalize</span>
            </div>
            <p className={`text-xs leading-relaxed ${subtextColor}`}>
              Choose how the player uses artwork, theme color, and live system-audio visualization. ORBIT reads playback details from Windows; it does not record microphone audio.
            </p>

            <div>
              <span className={`block text-xs font-semibold mb-2 ${labelColor}`}>Player background</span>
              <div className="grid grid-cols-3 gap-2">
                {([
                  ["cover", "Cover art", "Soft artwork backdrop"],
                  ["theme", "Theme", "Use the active theme"],
                  ["solid", "Solid", "No artwork backdrop"],
                ] as const).map(([mode, title, description]) => (
                  <button
                    key={mode}
                    type="button"
                    aria-pressed={formData.mediaBackgroundMode === mode}
                    onClick={() => setFormData({ ...formData, mediaBackgroundMode: mode })}
                    className={`rounded-xl border p-2 text-left transition-colors cursor-pointer ${
                      formData.mediaBackgroundMode === mode
                        ? "border-blue-400 bg-blue-500/10"
                        : isLight ? "border-slate-200 bg-white/70 hover:border-blue-300" : "border-slate-700 bg-slate-950/40 hover:border-blue-700"
                    }`}
                  >
                    <span className={`block text-[11px] font-bold ${labelColor}`}>{title}</span>
                    <span className={`mt-0.5 block text-[10px] leading-snug ${subtextColor}`}>{description}</span>
                  </button>
                ))}
              </div>
            </div>

            <div>
              <span className={`block text-xs font-semibold mb-2 ${labelColor}`}>Audio visualization</span>
              <div className="grid grid-cols-3 gap-2">
                {([
                  ["spectrum", "Spectrum", "20 frequency bands"],
                  ["beats", "Bass pulse", "Follow low frequencies"],
                  ["off", "Off", "Hide visualizer"],
                ] as const).map(([mode, title, description]) => (
                  <button
                    key={mode}
                    type="button"
                    aria-pressed={formData.mediaVisualizerMode === mode}
                    onClick={() => setFormData({ ...formData, mediaVisualizerMode: mode })}
                    className={`rounded-xl border p-2 text-left transition-colors cursor-pointer ${
                      formData.mediaVisualizerMode === mode
                        ? "border-blue-400 bg-blue-500/10"
                        : isLight ? "border-slate-200 bg-white/70 hover:border-blue-300" : "border-slate-700 bg-slate-950/40 hover:border-blue-700"
                    }`}
                  >
                    <span className={`block text-[11px] font-bold ${labelColor}`}>{title}</span>
                    <span className={`mt-0.5 block text-[10px] leading-snug ${subtextColor}`}>{description}</span>
                  </button>
                ))}
              </div>
            </div>

            <label className="flex items-start gap-3 text-xs cursor-pointer">
              <input type="checkbox" checked={formData.mediaShowCoverArt} onChange={(e) => setFormData({ ...formData, mediaShowCoverArt: e.target.checked })} className="mt-0.5 accent-blue-500" />
              <span><span className={`block font-semibold ${labelColor}`}>Show cover art</span><span className={`block text-[11px] ${subtextColor}`}>Show artwork in the player and compact capsule.</span></span>
            </label>
            <label className="flex items-start gap-3 text-xs cursor-pointer">
              <input type="checkbox" checked={formData.mediaCompactMode} onChange={(e) => setFormData({ ...formData, mediaCompactMode: e.target.checked })} className="mt-0.5 accent-blue-500" />
              <span><span className={`block font-semibold ${labelColor}`}>Compact expanded player</span><span className={`block text-[11px] ${subtextColor}`}>Use a tighter layout when the island is expanded.</span></span>
            </label>
            <label className="flex items-start gap-3 text-xs cursor-pointer">
              <input type="checkbox" checked={formData.adaptiveAlbumAccent} onChange={(e) => setFormData({ ...formData, adaptiveAlbumAccent: e.target.checked })} className="mt-0.5 accent-blue-500" />
              <span><span className={`block font-semibold ${labelColor}`}>Match accent to artwork</span><span className={`block text-[11px] ${subtextColor}`}>Use cover-art color for the player glow and controls.</span></span>
            </label>
          </div>

          {/* Bento Card 2: Typography & Font Engine (Sub-Phase 7.4b - Violet Accent) */}
          <div className={`rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Type className="w-4 h-4 text-violet-500" />
                <span>Typography & Font Engine</span>
              </h2>
              <span className="text-[10px] text-violet-500 bg-violet-500/10 border border-violet-500/25 px-2 py-0.5 rounded font-mono font-semibold">
                6 Presets
              </span>
            </div>

            <p className={`text-xs leading-relaxed ${subtextColor}`}>
              Select a typography style applied across the Dynamic Island overlay and compact capsules.
            </p>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
              {FONT_PRESETS.map((font) => {
                const isSelected = (formData.fontFamily || "inter") === font.id;
                return (
                  <button
                    key={font.id}
                    type="button"
                    onClick={() => setFormData({ ...formData, fontFamily: font.id })}
                    className={`text-left p-3 rounded-xl border transition-all cursor-pointer relative ${
                      isSelected
                        ? "border-violet-500 ring-2 ring-violet-500/30 bg-violet-500/10"
                        : isLight
                        ? "border-slate-200 bg-slate-50 hover:bg-slate-100"
                        : "border-slate-800 bg-slate-950/60 hover:bg-slate-800/50"
                    }`}
                  >
                    <div className="flex items-center justify-between">
                      <span className="text-xs font-bold text-slate-100 truncate block">
                        {font.name}
                      </span>
                      {isSelected && (
                        <span className="w-2 h-2 rounded-full bg-violet-400 shrink-0" />
                      )}
                    </div>
                    <span className="text-[10px] text-slate-400 block truncate mt-0.5">
                      {font.desc}
                    </span>
                    <div
                      className={`text-xs mt-2 px-2 py-1 rounded-md bg-black/20 text-slate-200 truncate ${font.className}`}
                    >
                      {font.preview}
                    </div>
                  </button>
                );
              })}
            </div>

            {/* Density Selector */}
            <div className="pt-2 border-t border-white/10 flex items-center justify-between">
              <div>
                <span className={`text-xs font-semibold ${labelColor}`}>
                  Island Layout Density
                </span>
                <p className={`text-[11px] ${subtextColor}`}>
                  Adjust internal spacing and capsule padding
                </p>
              </div>
              <select
                value={formData.density}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    density: e.target.value as AppConfig["density"],
                  })
                }
                className={`rounded-xl px-3 py-1.5 text-xs border focus:outline-none cursor-pointer ${inputBg}`}
              >
                <option value="compact" className={optionCls}>Compact</option>
                <option value="balanced" className={optionCls}>Balanced (Default)</option>
                <option value="detailed" className={optionCls}>Detailed</option>
              </select>
            </div>
          </div>

          {/* Bento Card 3: Notifications & Privacy (Emerald Accent) */}
          <div id="settings-privacy" className={`scroll-mt-20 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Bell className="w-4 h-4 text-emerald-500" />
                <span>Notifications & Privacy</span>
              </h2>
              <div className="flex items-center space-x-1.5">
                <span
                  className={`w-2 h-2 rounded-full ${
                    notifAccessStatus === "allowed"
                      ? "bg-emerald-400"
                      : notifAccessStatus === "denied"
                      ? "bg-rose-400"
                      : "bg-amber-400"
                  }`}
                />
                <span className="text-[10.5px] font-mono capitalize opacity-80">
                  {notifAccessStatus}
                </span>
              </div>
            </div>

            {notifAccessStatus !== "allowed" && (
              <div className="bg-amber-500/10 border border-amber-500/30 rounded-xl p-3 text-xs text-amber-200 space-y-2">
                <p>
                  ORBIT requires Windows Notification Listener access to display 2-step compact alerts and brand icons.
                </p>
                <button
                  type="button"
                  onClick={handleRequestNotifAccess}
                  disabled={isRequestingNotif}
                  className="px-3 py-1.5 bg-amber-600 hover:bg-amber-500 text-white rounded-lg text-xs font-bold transition-colors cursor-pointer disabled:opacity-50"
                >
                  {isRequestingNotif ? "Requesting Access..." : "Grant Windows Permission"}
                </button>
              </div>
            )}

            <div className="space-y-3">
              <label className="flex items-start space-x-3 text-xs cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.notificationsEnabled}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      notificationsEnabled: e.target.checked,
                    })
                  }
                  className="mt-0.5 rounded bg-slate-800 border-slate-700 text-emerald-500 focus:ring-0"
                />
                <div>
                  <span className={`font-semibold ${labelColor}`}>
                    Enable Island Notifications
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Surface 2-step interactive alerts with brand icons
                  </p>
                </div>
              </label>

              <label className="flex items-start space-x-3 text-xs cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.privacyMode}
                  onChange={(e) =>
                    setFormData({ ...formData, privacyMode: e.target.checked })
                  }
                  className="mt-0.5 rounded bg-slate-800 border-slate-700 text-emerald-500 focus:ring-0"
                />
                <div>
                  <span className={`font-semibold flex items-center space-x-1.5 ${labelColor}`}>
                    <Shield className="w-3.5 h-3.5 text-emerald-500" />
                    <span>Privacy Mode (Hide message content)</span>
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Hides sender text, showing only app title and brand logo
                  </p>
                </div>
              </label>

              <label className="flex items-start space-x-3 text-xs cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.showPrivacyIndicators}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      showPrivacyIndicators: e.target.checked,
                    })
                  }
                  className="mt-0.5 rounded bg-slate-800 border-slate-700 text-emerald-500 focus:ring-0"
                />
                <div>
                  <span className={`font-semibold ${labelColor}`}>
                    Camera & Microphone Privacy Light
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Active hardware security indicator: Green (Camera), Yellow (Microphone), Cyan (Both Active)
                  </p>
                </div>
              </label>

              {/* Interactive Privacy Light Test & Real-time Sensor Telemetry */}
              <div className="ml-7 pt-1 pb-1 flex flex-col space-y-2">
                <div className="flex items-center space-x-2">
                  <button
                    type="button"
                    onClick={handleTestPrivacy}
                    disabled={isTestingPrivacy || !formData.showPrivacyIndicators}
                    className={`px-3 py-1.5 rounded-lg text-xs font-semibold flex items-center space-x-1.5 border transition-all ${
                      isTestingPrivacy
                        ? "bg-cyan-500/20 text-cyan-300 border-cyan-500/40 animate-pulse"
                        : !formData.showPrivacyIndicators
                        ? "opacity-40 cursor-not-allowed bg-slate-800 text-slate-400 border-slate-700"
                        : "bg-slate-800 hover:bg-slate-700 text-slate-200 border-slate-700 cursor-pointer"
                    }`}
                  >
                    <Video className="w-3.5 h-3.5 text-cyan-400" />
                    <span>{isTestingPrivacy ? "Testing on Island (4s)..." : "Test Privacy Light (4s)"}</span>
                  </button>
                  {isTestingPrivacy && (
                    <span className="text-[11px] font-mono font-medium text-cyan-400 animate-pulse">
                      🟢 Cam ➔ 🟡 Mic ➔ 🩵 Dual
                    </span>
                  )}
                </div>

                {/* Real-time Hardware Privacy Telemetry Status */}
                <div className={`text-[11px] flex items-center space-x-1.5 font-medium ${subtextColor}`}>
                  <span>Live Sensor Status:</span>
                  {privacyState === null ? (
                    <span className="text-slate-500 italic">Checking camera and microphone status…</span>
                  ) : privacyState.cameraActive && privacyState.microphoneActive ? (
                    <span className="inline-flex items-center space-x-1 text-cyan-400 font-semibold">
                      <span className="w-2 h-2 rounded-full bg-cyan-400 animate-pulse" />
                      <span>Camera & Microphone Active ({privacyState.cameraApp || "In Use"})</span>
                    </span>
                  ) : privacyState?.cameraActive ? (
                    <span className="inline-flex items-center space-x-1 text-emerald-400 font-semibold">
                      <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                      <span>Camera Active ({privacyState.cameraApp || "In Use"})</span>
                    </span>
                  ) : privacyState?.microphoneActive ? (
                    <span className="inline-flex items-center space-x-1 text-amber-400 font-semibold">
                      <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
                      <span>Microphone Active ({privacyState.microphoneApp || "In Use"})</span>
                    </span>
                  ) : (
                    <span className="text-slate-500 italic">
                      Idle (No apps using camera or microphone)
                    </span>
                  )}
                </div>
              </div>

              {/* Preview Duration Slider */}
              <div className="pt-2 border-t border-white/10 flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Compact Preview Duration
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Seconds before auto-reverting when unhovered
                  </p>
                </div>
                <div className="flex items-center space-x-2">
                  <input
                    type="number"
                    min={2}
                    max={30}
                    value={formData.notificationPreviewDurationSec}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        notificationPreviewDurationSec: Math.max(
                          2,
                          Math.min(30, parseInt(e.target.value) || 4)
                        ),
                      })
                    }
                    className={`w-14 rounded-lg px-2 py-1 text-xs text-center border focus:outline-none ${inputBg}`}
                  />
                  <span className={`text-xs font-mono ${subtextColor}`}>sec</span>
                </div>
              </div>

              {/* Clear Queue Action */}
              <div className="pt-2 border-t border-white/10 flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Clear Notification Queue
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Dismiss all currently queued notifications
                  </p>
                </div>
                <div className="flex items-center space-x-2">
                  {clearNotifMessage && (
                    <span className="text-xs text-emerald-400 font-medium">
                      {clearNotifMessage}
                    </span>
                  )}
                  <button
                    type="button"
                    onClick={handleClearNotifications}
                    className={`px-3 py-1.5 rounded-lg text-xs font-semibold border transition-all cursor-pointer ${
                      isLight
                        ? "bg-slate-100 hover:bg-slate-200 border-slate-300 text-slate-800"
                        : "bg-slate-800 hover:bg-slate-700 border-slate-700 text-slate-200"
                    }`}
                  >
                    Clear Queue
                  </button>
                </div>
              </div>

              {/* Screen Recording Stealth (Sub-Phase 9.3) */}
              <div className="pt-2 border-t border-white/10 flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold flex items-center space-x-1.5 ${labelColor}`}>
                    <EyeOff className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Hide in Screen Recording & Share (Alt+S)</span>
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Exclude ORBIT from OBS, Discord, Teams, Zoom & Meet video captures
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.hideFromScreenCapture ?? false}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        hideFromScreenCapture: e.target.checked,
                      })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-emerald-600"></div>
                </label>
              </div>

              {/* Meeting Auto-Hide (Sub-Phase 9.3) */}
              <div className="pt-2 border-t border-white/10 flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold flex items-center space-x-1.5 ${labelColor}`}>
                    <Video className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Auto-Hide During Video Calls & Meetings</span>
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Auto-suppress island when both mic & cam are active in calls; restores on alerts & timers
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.autoHideInMeetings ?? true}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        autoHideInMeetings: e.target.checked,
                      })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-emerald-600"></div>
                </label>
              </div>

              {/* Compact Network Transfer Mode (Sub-Phase 9.2) */}
              <div className="pt-2 border-t border-white/10 space-y-2">
                <div>
                  <span className={`text-xs font-semibold flex items-center space-x-1.5 ${labelColor}`}>
                    <Activity className="w-3.5 h-3.5 text-emerald-400" />
                    <span>Compact Network Transfer Display</span>
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Choose when transfer speeds and download completion badges flip in the compact pill
                  </p>
                </div>
                <div className="grid grid-cols-3 gap-2">
                  {[
                    { id: "off", label: "Hidden", desc: "Full Privacy" },
                    { id: "speed_only", label: "Speeds Only", desc: "Live KB/s & MB/s" },
                    { id: "full", label: "Speeds + Alerts", desc: "Download Complete" },
                  ].map((mode) => (
                    <button
                      key={mode.id}
                      type="button"
                      onClick={() =>
                        setFormData({
                          ...formData,
                          networkActivityMode: mode.id as AppConfig["networkActivityMode"],
                        })
                      }
                      className={`p-2 rounded-xl text-left border transition-all cursor-pointer flex flex-col justify-between ${
                        formData.networkActivityMode === mode.id
                          ? isLight
                            ? "bg-emerald-50 border-emerald-500 shadow-sm"
                            : "bg-emerald-500/15 border-emerald-500/50 shadow-[0_0_12px_rgba(16,185,129,0.18)]"
                          : isLight
                          ? "bg-slate-50 border-slate-200 hover:bg-slate-100"
                          : "bg-slate-800/60 border-slate-700/60 hover:bg-slate-800"
                      }`}
                    >
                      <span
                        className={`text-[11.5px] font-bold ${
                          formData.networkActivityMode === mode.id
                            ? isLight
                              ? "text-emerald-700"
                              : "text-emerald-300"
                            : labelColor
                        }`}
                      >
                        {mode.label}
                      </span>
                      <span className={`text-[9.5px] mt-0.5 ${subtextColor}`}>
                        {mode.desc}
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* Bento Card: Global Keyboard Shortcuts (Sub-Phase 9.3 & 9.4 - Indigo Accent) */}
          <div className={`rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Keyboard className="w-4 h-4 text-indigo-400" />
                <span>Global Keyboard Shortcuts</span>
              </h2>
              <span className={`text-[10px] font-mono font-semibold px-2 py-0.5 rounded ${
                formData.shortcutsEnabled
                  ? "text-indigo-400 bg-indigo-500/10 border border-indigo-500/25"
                  : "text-slate-400 bg-slate-800/40 border border-slate-700/40"
              }`}>
                {formData.shortcutsEnabled ? "Active" : "Disabled (Gaming Mode)"}
              </span>
            </div>

            <div className="space-y-3">
              {/* Master Gaming Mode Toggle */}
              <div className="flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Enable Global Alt Shortcuts
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Disable this while gaming so Alt combinations never interfere with controls
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.shortcutsEnabled ?? true}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        shortcutsEnabled: e.target.checked,
                      })
                    }
                    className="sr-only peer"
                  />
                  <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-600"></div>
                </label>
              </div>

              {/* Hotkeys Grid */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-1">
                {[
                  { key: "Alt + O", label: "Toggle Expand / Compact", desc: "Instant expand or collapse" },
                  { key: "Alt + H", label: "Hide / Show Island", desc: "Retract into Windows tray" },
                  { key: "Alt + S", label: "Screen-Share Stealth", desc: "Toggle capture invisibility" },
                  { key: "Alt + M", label: "Play / Pause Media", desc: "Global playback control" },
                ].map((hk) => (
                  <div
                    key={hk.key}
                    className={`p-2.5 rounded-xl border flex items-center justify-between transition-all ${
                      formData.shortcutsEnabled
                        ? isLight
                          ? "bg-slate-50 border-slate-200"
                          : "bg-slate-800/60 border-slate-700/60"
                        : "opacity-45 bg-slate-800/30 border-slate-800"
                    }`}
                  >
                    <div>
                      <div className={`text-[11.5px] font-semibold ${labelColor}`}>
                        {hk.label}
                      </div>
                      <div className={`text-[10px] ${subtextColor}`}>
                        {hk.desc}
                      </div>
                    </div>
                    <kbd className={`px-2 py-1 rounded text-[10.5px] font-mono font-bold tracking-wider border shadow-sm shrink-0 ${
                      isLight
                        ? "bg-white border-slate-300 text-slate-800"
                        : "bg-slate-900 border-slate-700 text-indigo-300"
                    }`}>
                      {hk.key}
                    </kbd>
                  </div>
                ))}
              </div>

              {!formData.shortcutsEnabled && (
                <p className="text-[10.5px] text-amber-400/90 bg-amber-500/10 border border-amber-500/20 rounded-lg p-2 leading-relaxed">
                  Hotkeys are unregistered at the Windows API level. All Alt key combinations pass through transparently to full-screen games and pro editing software.
                </p>
              )}
            </div>
          </div>

          {/* Bento Card 4: Process Memory Telemetry (Amber Accent) */}
          <div id="settings-performance" className={`scroll-mt-20 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Cpu className="w-4 h-4 text-amber-500" />
                <span>Memory used by ORBIT</span>
              </h2>
              <button
                type="button"
                onClick={handleRefreshMemory}
                className="flex items-center space-x-1 text-[11px] text-amber-400 hover:text-amber-300 font-semibold cursor-pointer"
              >
                <RefreshCw className="w-3 h-3" />
                <span>Refresh</span>
              </button>
            </div>

            {telemetry ? (
              <div
                className={`space-y-2 text-xs font-mono p-3 rounded-xl border ${
                  isLight ? "bg-slate-50 border-slate-200" : "bg-slate-950/80 border-slate-800"
                }`}
              >
                <div className="flex flex-wrap justify-between gap-x-3 gap-y-1">
                  <span className="opacity-70">
                    ORBIT app (PID {telemetry.mainProcess.pid}):
                  </span>
                  <span className="font-bold">
                    {telemetry.mainProcess.privateWorkingSetMb?.toFixed(2) ?? "—"} MB RAM
                  </span>
                </div>
                {telemetry.webviewProcesses.map((child) => (
                  <div
                    key={child.pid}
                    className="flex flex-wrap justify-between gap-x-3 gap-y-1 opacity-70 pl-3"
                  >
                    <span>↳ {child.name} (PID {child.pid}):</span>
                    <span>
                      {child.privateWorkingSetMb?.toFixed(2) ?? "—"} MB RAM
                    </span>
                  </div>
                ))}
                <div className="pt-2 border-t border-white/10 flex flex-wrap justify-between gap-x-3 gap-y-1 font-bold">
                  <span>Combined app + WebView2 RAM:</span>
                  <span className="text-amber-400">{telemetry.combinedPrivateWorkingSetMb?.toFixed(2) ?? "Incomplete"} MB</span>
                </div>
                {telemetry.unmeasuredWebviewCount > 0 && <div className="text-[10px] text-amber-400">{telemetry.unmeasuredWebviewCount} WebView2 process(es) could not be measured.</div>}
                <div className="text-[10px] opacity-70">Measured {new Date(Number(telemetry.measuredAtUnixMs)).toLocaleTimeString()}</div>
                <div className="text-[10px] leading-relaxed opacity-70">RAM shown is Private Working Set (resident app memory). It is a live measurement, not a performance target.</div>
              </div>
            ) : (
              <div className="text-xs opacity-60 py-2">
                {telemetryError ? `Telemetry error: ${telemetryError}` : "Measuring process memory..."}
              </div>
            )}

            {/* Launch at Windows Sign-In */}
            <div className="pt-2 border-t border-white/10 flex items-center justify-between">
              <div>
                <span className={`text-xs font-semibold ${labelColor}`}>
                  Launch at Windows Sign-In
                </span>
                <p className={`text-[11px] ${subtextColor}`}>
                  Start ORBIT in background on system boot
                </p>
              </div>
              <label className="relative inline-flex items-center cursor-pointer">
                <input
                  type="checkbox"
                  checked={formData.launchAtStartup}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      launchAtStartup: e.target.checked,
                    })
                  }
                  className="sr-only peer"
                />
                <div className="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-600"></div>
              </label>
            </div>
          </div>

          {/* Bento Card: Academic & Office Schedule Sync (Cyan/Sky Accent) */}
          <div className={`col-span-1 md:col-span-2 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Calendar className="w-4 h-4 text-sky-400" />
                <span>Academic & Office Schedule Sync (.ICS / .CSV / .JSON)</span>
              </h2>
              {calendarImportMsg && (
                <span className="text-xs text-sky-400 font-semibold animate-pulse">
                  {calendarImportMsg}
                </span>
              )}
            </div>

            <p className={`text-xs ${subtextColor}`}>
              Import university exam timetables, lecture courses, assignment due dates, or office sprint schedules directly into ORBIT. Events are automatically categorized and alerted in the Dynamic Island before they start.
            </p>

            {/* 3 Quick Import Buttons */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-1">
              <button
                type="button"
                disabled={isImportingCalendar}
                onClick={() => handleImportCalendar("academic")}
                className="p-3 rounded-xl border border-sky-500/25 bg-sky-500/10 hover:bg-sky-500/20 active:scale-[0.98] transition-all flex flex-col items-center justify-center text-center cursor-pointer disabled:opacity-50"
              >
                <GraduationCap className="w-5 h-5 text-sky-400 mb-1.5" />
                <span className={`text-xs font-bold ${labelColor}`}>Academic Schedule</span>
                <span className="text-[10px] text-white/50 mt-0.5">Exams, Classes, Viva (.ics / .csv)</span>
              </button>

              <button
                type="button"
                disabled={isImportingCalendar}
                onClick={() => handleImportCalendar("office")}
                className="p-3 rounded-xl border border-amber-500/25 bg-amber-500/10 hover:bg-amber-500/20 active:scale-[0.98] transition-all flex flex-col items-center justify-center text-center cursor-pointer disabled:opacity-50"
              >
                <Briefcase className="w-5 h-5 text-amber-400 mb-1.5" />
                <span className={`text-xs font-bold ${labelColor}`}>Office & Work Calendar</span>
                <span className="text-[10px] text-white/50 mt-0.5">Sprints, 1:1s, Deadlines (.ics / .csv)</span>
              </button>

              <button
                type="button"
                disabled={isImportingCalendar}
                onClick={() => handleImportCalendar("ics")}
                className="p-3 rounded-xl border border-emerald-500/25 bg-emerald-500/10 hover:bg-emerald-500/20 active:scale-[0.98] transition-all flex flex-col items-center justify-center text-center cursor-pointer disabled:opacity-50"
              >
                <Upload className="w-5 h-5 text-emerald-400 mb-1.5" />
                <span className={`text-xs font-bold ${labelColor}`}>Google / Outlook iCal</span>
                <span className="text-[10px] text-white/50 mt-0.5">Standard iCalendar (.ics / .ical)</span>
              </button>
            </div>

            {/* Actions & Format Guidance */}
            <div className="pt-2 border-t border-white/10 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2.5">
              <div className="flex items-center space-x-2">
                <button
                  type="button"
                  onClick={copyCsvTemplate}
                  className="px-2.5 py-1 rounded-lg text-xs font-medium border border-white/20 hover:bg-white/10 flex items-center space-x-1.5 transition-colors cursor-pointer"
                >
                  <FileText className="w-3.5 h-3.5 text-white/70" />
                  <span className={labelColor}>Copy CSV Template</span>
                </button>

                <button
                  type="button"
                  onClick={handleClearImportedCalendar}
                  className="px-2.5 py-1 rounded-lg text-xs font-medium border border-rose-500/30 text-rose-400 hover:bg-rose-500/10 flex items-center space-x-1.5 transition-colors cursor-pointer"
                  title="Removes imported calendar events while preserving manual tasks"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  <span>Clear Imported Schedules</span>
                </button>
              </div>

              <div className="text-[10.5px] text-white/45 sm:text-right">
                <span>Columns: Title, Date (YYYY-MM-DD), Time (HH:MM), Category</span>
              </div>
            </div>
          </div>

          {/* Bento Card 5: System, Onboarding & 3-Tier Reset (Sub-Phase 8.2 - Rose/Indigo Accent) */}
          <div className={`col-span-1 md:col-span-2 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <HelpCircle className="w-4 h-4 text-rose-500" />
                <span>Onboarding, Storage & Diagnostic Resets</span>
              </h2>
              {resetMessage && (
                <span className="text-xs text-cyan-400 font-semibold animate-pulse">
                  {resetMessage}
                </span>
              )}
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {/* File Shelf Storage Mode */}
              <div className="space-y-1">
                <span className={`text-xs font-semibold ${labelColor}`}>
                  File Shelf Storage Mode
                </span>
                <p className={`text-[11px] ${subtextColor}`}>
                  Reference Only saves a file location. Managed Copy stores a separate copy and uses available disk space.
                </p>
                <select
                  value={formData.shelfStorageMode ?? "reference"}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      shelfStorageMode: e.target.value as AppConfig["shelfStorageMode"],
                    })
                  }
                  className={`w-full rounded-xl px-3 py-2 text-xs border focus:outline-none cursor-pointer mt-1 ${inputBg}`}
                >
                  <option value="reference" className={optionCls}>Reference Only (Default)</option>
                  <option value="copy" className={optionCls}>Managed Copy (disk space available)</option>
                </select>
              </div>

              <div className="space-y-1">
                <span className={`text-xs font-semibold ${labelColor}`}>Drag-to-expand drop tray</span>
                <p className={`text-[11px] ${subtextColor}`}>
                  Drag files onto ORBIT to reveal Shelf, Open, Open with, File Explorer, and Print targets.
                </p>
                <label className="flex items-center gap-2.5 pt-1 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.dragDropEnabled}
                    onChange={(event) => setFormData({ ...formData, dragDropEnabled: event.target.checked })}
                    className="w-4 h-4 accent-cyan-500"
                  />
                  <span className={`text-xs ${labelColor}`}>{formData.dragDropEnabled ? "Enabled" : "Disabled"}</span>
                </label>
              </div>

              {/* Diagnostic Log Reveal */}
              <div className="space-y-1">
                <span className={`text-xs font-semibold ${labelColor}`}>
                  Diagnostic Logging & Export
                </span>
                <p className={`text-[11px] ${subtextColor}`}>
                  Inspect rotating local diagnostic log (%APPDATA%\com.orbit.island\logs\orbit.log)
                </p>
                <div className="flex items-center space-x-2 pt-1">
                  <button
                    type="button"
                    onClick={handleRevealLogs}
                    className="flex items-center space-x-1.5 px-3 py-1.5 rounded-xl text-xs font-bold bg-cyan-600/20 text-cyan-300 border border-cyan-500/30 hover:bg-cyan-600/35 transition-colors cursor-pointer"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    <span>Reveal Diagnostic Log</span>
                  </button>
                  {logPathMessage && (
                    <span className="text-[10.5px] text-cyan-400 font-mono truncate max-w-xs">
                      {logPathMessage}
                    </span>
                  )}
                </div>
              </div>
            </div>

            {/* Three-Tier Reset & Replay Guided Tour Actions */}
            <div className="pt-3 border-t border-white/10 flex flex-wrap items-center justify-between gap-2.5">
              <div className="flex flex-wrap items-center gap-2">
                {/* Replay Tour */}
                <button
                  type="button"
                  onClick={handleReplayTour}
                  className="flex items-center space-x-1.5 px-3 py-1.5 rounded-xl text-xs font-bold bg-indigo-600/20 text-indigo-300 border border-indigo-500/30 hover:bg-indigo-600/35 transition-colors cursor-pointer"
                >
                  <HelpCircle className="w-3.5 h-3.5" />
                  <span>Replay Guided Tour</span>
                </button>

                {/* Tier 1: Reset Settings */}
                <button
                  type="button"
                  onClick={handleResetDefaults}
                  className={`px-3 py-1.5 rounded-xl text-xs font-semibold border transition-colors cursor-pointer ${
                    isLight
                      ? "bg-slate-100 hover:bg-slate-200 border-slate-300 text-slate-800"
                      : "bg-slate-800 hover:bg-slate-700 border-slate-700 text-slate-200"
                  }`}
                >
                  Reset Settings to Defaults
                </button>

                {/* Tier 2: Clear Reminders & Shelf */}
                <button
                  type="button"
                  onClick={handleClearLocalData}
                  className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-amber-500/15 hover:bg-amber-500/25 text-amber-300 border border-amber-500/30 transition-colors cursor-pointer"
                >
                  Clear Reminders &amp; Shelf
                </button>
              </div>

              {/* Tier 3: Full Reset to Onboarding */}
              <button
                type="button"
                onClick={handleFullReset}
                className="flex items-center space-x-1.5 px-3 py-1.5 rounded-xl text-xs font-bold bg-rose-600/20 text-rose-300 border border-rose-500/35 hover:bg-rose-600/35 transition-colors cursor-pointer"
                title="Clears all data, resets config, and restarts the first-launch guided onboarding tour"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>Full Reset to Onboarding</span>
              </button>
            </div>
          </div>
          <section id="settings-about" className={`scroll-mt-20 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center gap-2 border-b border-white/10 pb-2">
              <ShieldCheck className="h-4 w-4 text-cyan-500" />
              <h2 className={`text-xs font-bold uppercase tracking-wider ${cardHeaderColor}`}>About, terms & privacy</h2>
            </div>
            <p className={`text-xs ${subtextColor}`}>ORBIT {packageJson.version}. Your settings, saved Shelf items and media details stay on this device. No account or cloud sync is used.</p>
            <button type="button" onClick={() => setShowPrivacyTerms(true)} className={`w-full rounded-xl border px-3 py-2.5 text-left text-xs font-semibold ${isLight ? "border-slate-200 hover:bg-cyan-50" : "border-white/10 hover:bg-white/5"}`}>
              Read Terms & Privacy
              <span className={`block mt-1 text-[10px] font-normal ${subtextColor}`}>Local storage and optional network behavior.</span>
            </button>
            <button type="button" onClick={() => ipc.openExternalUrl("https://github.com/Dineswarkumar/Orbit-Island").catch((error) => setResetMessage(`Could not open GitHub: ${error}`))} className="w-full flex items-center justify-between rounded-xl border border-cyan-500/25 bg-cyan-500/5 px-3 py-2.5 text-xs font-semibold text-cyan-500 hover:bg-cyan-500/10">
              <span>ORBIT on GitHub</span><ExternalLink className="h-3.5 w-3.5" />
            </button>
            <button type="button" onClick={() => ipc.openExternalUrl("https://github.com/Dineswarkumar/Orbit-Island/releases").catch((error) => setResetMessage(`Could not open Releases: ${error}`))} className={`w-full flex items-center justify-between rounded-xl border px-3 py-2.5 text-xs font-semibold ${isLight ? "border-slate-200 hover:bg-slate-50" : "border-white/10 hover:bg-white/5"}`}>
              <span>Latest releases & updates</span><ExternalLink className="h-3.5 w-3.5" />
            </button>
            <div className="border-t border-white/10 pt-3 space-y-2">
              <div className="flex items-center justify-between"><h3 className={`text-xs font-bold ${labelColor}`}>Pinned drop destinations</h3><span className={`text-[10px] ${subtextColor}`}>{formData.pinnedDestinations.length}/6</span></div>
              <p className={`text-[10px] leading-relaxed ${subtextColor}`}>Pinned web apps appear in the expanded drop tray. ORBIT saves dropped files to Shelf, opens the site, then you attach them there. Websites do not accept automatic file handoff.</p>
              <div className="flex gap-2">
                <input aria-label="Destination name" value={destinationName} onChange={(event) => setDestinationName(event.target.value)} placeholder="Name" maxLength={32} className={`min-w-0 w-1/3 rounded-lg border px-2 py-1.5 text-[10px] ${inputBg}`} />
                <input aria-label="Destination HTTPS URL" value={destinationUrl} onChange={(event) => setDestinationUrl(event.target.value)} placeholder="https://…" className={`min-w-0 flex-1 rounded-lg border px-2 py-1.5 text-[10px] ${inputBg}`} />
                <button type="button" disabled={formData.pinnedDestinations.length >= 6} onClick={() => {
                  const name = destinationName.trim();
                  const url = destinationUrl.trim();
                  try {
                    const parsed = new URL(url);
                    if (!name || parsed.protocol !== "https:") throw new Error();
                    setFormData({ ...formData, pinnedDestinations: [...formData.pinnedDestinations, { id: crypto.randomUUID(), name, url: parsed.href }] });
                    setDestinationName(""); setDestinationUrl("");
                  } catch { setSaveStatus("error"); }
                }} className="shrink-0 rounded-lg bg-cyan-600 px-2.5 py-1.5 text-[10px] font-semibold text-white disabled:opacity-40">Pin</button>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {[ ["ChatGPT", "https://chatgpt.com"], ["Gemini", "https://gemini.google.com"], ["Claude", "https://claude.ai"], ["WhatsApp", "https://web.whatsapp.com"], ["Discord", "https://discord.com/app"], ["Instagram", "https://www.instagram.com"] ].map(([name, url]) => (
                  <button key={name} type="button" disabled={formData.pinnedDestinations.length >= 6 || formData.pinnedDestinations.some((pin) => pin.name === name)} onClick={() => setFormData({ ...formData, pinnedDestinations: [...formData.pinnedDestinations, { id: crypto.randomUUID(), name, url }] })} className={`rounded-full border px-2 py-1 text-[9px] disabled:opacity-35 ${isLight ? "border-slate-200 hover:bg-slate-50" : "border-white/10 hover:bg-white/5"}`}>+ {name}</button>
                ))}
              </div>
              {formData.pinnedDestinations.length > 0 && <div className="space-y-1">{formData.pinnedDestinations.map((pin) => <div key={pin.id} className={`flex items-center justify-between gap-2 rounded-lg px-2 py-1 ${isLight ? "bg-slate-50" : "bg-white/5"}`}><span className={`min-w-0 truncate text-[10px] ${labelColor}`}>{pin.name}<span className={`ml-2 ${subtextColor}`}>{pin.url}</span></span><button type="button" aria-label={`Remove ${pin.name}`} onClick={() => setFormData({ ...formData, pinnedDestinations: formData.pinnedDestinations.filter((item) => item.id !== pin.id) })} className="shrink-0 rounded p-1 text-rose-400 hover:bg-rose-500/10">Remove</button></div>)}</div>}
            </div>
          </section>
          </div>
        </div>
      </form>

      {showPrivacyTerms && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/70 p-4 backdrop-blur-sm" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setShowPrivacyTerms(false); }}>
          <section role="dialog" aria-modal="true" aria-labelledby="orbit-privacy-title" className={`max-h-[85vh] w-full max-w-2xl overflow-y-auto rounded-3xl border p-6 shadow-2xl ${isLight ? "border-slate-200 bg-white text-slate-900" : "border-slate-700 bg-slate-900 text-slate-100"}`}>
            <div className="flex items-start justify-between gap-4">
              <div><h2 id="orbit-privacy-title" className="text-lg font-bold">ORBIT Terms & Privacy</h2><p className={`mt-1 text-xs ${subtextColor}`}>Version {packageJson.version} · stored locally on this PC</p></div>
              <button type="button" aria-label="Close Terms & Privacy" onClick={() => setShowPrivacyTerms(false)} className="rounded-lg p-2 hover:bg-white/10"><X className="h-4 w-4" /></button>
            </div>
            <div className="mt-5 space-y-4 text-sm leading-relaxed">
              <div><h3 className="font-semibold">Terms of use</h3><p className={`mt-1 ${subtextColor}`}>ORBIT is a local desktop utility provided as-is. Use system controls only when you understand their effect. RAM Boost asks Windows to trim working sets for this session; it cannot guarantee a performance gain or recover memory that applications still need.</p></div>
              <div><h3 className="font-semibold">Local data</h3><p className={`mt-1 ${subtextColor}`}>Settings, reminders, file references or managed copies, and saved snippets are stored on this PC. ORBIT does not upload them or sync them to a cloud service. Shelf references store paths; managed copies use local disk space. Snippets are saved only when you choose to save them; ORBIT does not monitor the clipboard in the background.</p></div>
              <div><h3 className="font-semibold">Network and permissions</h3><p className={`mt-1 ${subtextColor}`}>Online weather is optional. When enabled, ORBIT requests weather from wttr.in. Opening GitHub links connects your browser to GitHub. Media and notification details are read from Windows locally. Microphone or camera activity indicators report Windows privacy status; ORBIT does not record audio or video.</p></div>
              <div><h3 className="font-semibold">Your choices</h3><p className={`mt-1 ${subtextColor}`}>You can remove saved items, clear local data, disable optional features, or uninstall ORBIT. Keep sensitive information such as passwords out of manually saved snippets.</p></div>
            </div>
          </section>
        </div>
      )}

      {/* Pinned Footer Actions */}
      <div
        className={`shrink-0 flex flex-wrap items-center justify-between gap-x-3 gap-y-2 px-6 py-3.5 border-t backdrop-blur z-20 transition-colors duration-200 ${headerBg}`}
      >
        <div className={`min-w-0 flex-1 text-[11px] truncate ${subtextColor}`}>
          Config file:{" "}
          <span className="font-mono opacity-80">
            %APPDATA%\com.orbit.island\config.json
          </span>
        </div>

        <div className="flex shrink-0 items-center space-x-3">
          {saveStatus === "success" && (
            <span className="flex items-center space-x-1 text-xs font-bold text-emerald-500">
              <Check className="w-4 h-4" />
              <span>Saved preferences!</span>
            </span>
          )}
          {saveStatus === "error" && (
            <span className="flex items-center space-x-1 text-xs font-bold text-rose-500">
              <X className="w-4 h-4" />
              <span>Failed to save!</span>
            </span>
          )}
          <button
            type="submit"
            form="settings-form"
            disabled={isSaving || isLoading}
            className="flex items-center space-x-2 bg-cyan-600 hover:bg-cyan-500 active:scale-95 text-white text-xs font-bold px-4 py-2 rounded-xl transition-all cursor-pointer shadow-sm disabled:opacity-50"
          >
            <Save className="w-4 h-4" />
            <span>{isSaving ? "Saving..." : "Save Preferences"}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
