import React, { useEffect, useState } from "react";
import { listen, emit } from "@tauri-apps/api/event";
import {
  Settings as SettingsIcon,
  Save,
  RefreshCw,
  X,
  Check,
  Cpu,
  Bell,
  Shield,
  Sparkles,
  Sun,
  Moon,
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
} from "lucide-react";
import { useConfigStore } from "../core/stores/useConfigStore";
import { ipc } from "../core/ipc/client";
import { MemoryReport } from "../core/types/telemetry";
import { AppConfig } from "../core/types/config";
import { PrivacyState } from "../core/types/notifications";
import { THEMES, getThemeMeta } from "../core/theme";
import packageJson from "../../package.json";

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

export const SettingsApp: React.FC = () => {
  const { config, loadConfig, updateConfig, isLoading, isSaving } =
    useConfigStore();
  const [formData, setFormData] = useState<AppConfig>(config);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">(
    "idle"
  );
  const [uiMode, setUiMode] = useState<"dark" | "light">(() => {
    try {
      return (
        (localStorage.getItem("orbit_settings_ui_mode") as "dark" | "light") ||
        "dark"
      );
    } catch {
      return "dark";
    }
  });

  const [telemetry, setTelemetry] = useState<MemoryReport | null>(null);
  const [telemetryError, setTelemetryError] = useState<string | null>(null);
  const [privacyState, setPrivacyState] = useState<PrivacyState | null>(null);
  const [isTestingPrivacy, setIsTestingPrivacy] = useState(false);
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

  const saveStatusTimer = React.useRef<ReturnType<typeof setTimeout> | null>(
    null
  );

  const isLight = uiMode === "light";

  const toggleUiMode = () => {
    const next = isLight ? "dark" : "light";
    setUiMode(next);
    try {
      localStorage.setItem("orbit_settings_ui_mode", next);
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
        setPrivacyState(event.payload);
      }
    );

    const unlistenConfig = listen<AppConfig>("config-changed", (event) => {
      useConfigStore.setState({ config: event.payload });
      setFormData(event.payload);
    });

    return () => {
      unlistenConfig.then((unlisten) => unlisten());
      unlistenPrivacy.then((unlisten) => unlisten());
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
      launchAtStartup: false,
      showHardwareMonitor: true,
      transparencyMode: "glass",
      notificationsEnabled: true,
      privacyMode: false,
      notificationPreviewDurationSec: 4,
      showPrivacyIndicators: true,
      blockedApps: [],
      surfaceOpacity: 80,
      showWeather: false,
      weatherUnit: "C",
      adaptiveAlbumAccent: true,
      expandTrigger: "hover",
      pauseInFullscreen: true,
      shelfStorageMode: "reference",
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
    });
    setResetMessage("Settings reset to defaults. Click 'Save Preferences' to persist.");
    setTimeout(() => setResetMessage(null), 4000);
  };

  const handleClearLocalData = async () => {
    try {
      await ipc.clearLocalData();
      setResetMessage("Local reminders and shelf references cleared.");
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
      const defaultCfg: AppConfig = {
        theme: "obsidian",
        position: "top-center",
        density: "balanced",
        timeFormat: "12h",
        alwaysOnTop: true,
        launchAtStartup: false,
        showHardwareMonitor: true,
        transparencyMode: "glass",
        notificationsEnabled: true,
        privacyMode: false,
        notificationPreviewDurationSec: 4,
        showPrivacyIndicators: true,
        blockedApps: [],
        surfaceOpacity: 80,
        showWeather: false,
        weatherUnit: "C",
        adaptiveAlbumAccent: true,
        expandTrigger: "hover",
        pauseInFullscreen: true,
        shelfStorageMode: "reference",
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

  // Base theme classes
  const canvasBg = isLight ? "bg-slate-100 text-slate-900" : "bg-[#090d16] text-slate-100";
  const headerBg = isLight ? "bg-white/95 border-slate-200" : "bg-slate-950/95 border-slate-800";
  const cardBg = isLight
    ? "bg-white border-slate-200/90 shadow-sm"
    : "bg-slate-900/75 border-slate-800/90 shadow-md";
  const cardHeaderColor = isLight ? "text-slate-600" : "text-slate-400";
  const labelColor = isLight ? "text-slate-700" : "text-slate-300";
  const subtextColor = isLight ? "text-slate-500" : "text-slate-400";
  const inputBg = isLight
    ? "bg-slate-50 border-slate-300 text-slate-900 focus:border-cyan-600"
    : "bg-slate-900/90 border-slate-700/80 text-slate-100 focus:border-cyan-400";
  const optionCls = isLight ? "bg-white text-slate-900" : "bg-slate-900 text-slate-100";

  return (
    <div
      className={`h-full w-full flex flex-col font-sans select-none overflow-hidden transition-colors duration-200 ${canvasBg}`}
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
            <SettingsIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center space-x-2">
              <h1 className="text-base font-bold tracking-tight">
                ORBIT Settings
              </h1>
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
          {/* Light / Dark Mode Toggle */}
          <button
            type="button"
            onClick={toggleUiMode}
            className={`flex items-center space-x-1.5 px-3 py-1.5 rounded-lg border text-xs font-semibold transition-all cursor-pointer ${
              isLight
                ? "bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-200"
                : "bg-slate-900 border-slate-700 text-slate-200 hover:bg-slate-800"
            }`}
            title={isLight ? "Switch to Dark Mode" : "Switch to Light Mode"}
          >
            {isLight ? (
              <>
                <Moon className="w-3.5 h-3.5 text-indigo-600" />
                <span>Dark Mode</span>
              </>
            ) : (
              <>
                <Sun className="w-3.5 h-3.5 text-amber-400" />
                <span>Light Mode</span>
              </>
            )}
          </button>

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

      {/* Main Responsive Form (Constrained to max-w-5xl, 2-column Bento Grid on wide monitors) */}
      <form
        id="settings-form"
        aria-label="Settings sections"
        onSubmit={handleSave}
        className="flex-1 min-h-0 overflow-y-auto px-4 sm:px-6 py-5 focus:outline-none"
      >
        <div className="max-w-5xl mx-auto w-full grid grid-cols-1 lg:grid-cols-2 gap-5">
          {/* Bento Card 1: 3D Appearance, Material & Placement (Cyan Accent) */}
          <div className={`rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Sparkles className="w-4 h-4 text-cyan-500" />
                <span>3D Appearance & Placement</span>
              </h2>
              <span className="text-[10px] text-cyan-500 bg-cyan-500/10 border border-cyan-500/25 px-2 py-0.5 rounded font-mono font-semibold">
                3D Shell
              </span>
            </div>

            <div className="grid grid-cols-2 gap-3.5">
              <div>
                <label className={`block text-xs font-semibold mb-1 ${labelColor}`}>
                  3D Orbital Theme
                </label>
                <select
                  value={formData.theme}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      theme: e.target.value as AppConfig["theme"],
                    })
                  }
                  className={`w-full rounded-xl px-3 py-2 text-xs border focus:outline-none cursor-pointer ${inputBg}`}
                >
                  {THEMES.map((t) => (
                    <option key={t.id} value={t.id} className={optionCls}>
                      {t.label}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className={`block text-xs font-semibold mb-1 ${labelColor}`}>
                  3D Glass Material
                </label>
                <select
                  value={formData.transparencyMode}
                  onChange={(e) =>
                    setFormData({
                      ...formData,
                      transparencyMode: e.target
                        .value as AppConfig["transparencyMode"],
                    })
                  }
                  className={`w-full rounded-xl px-3 py-2 text-xs border focus:outline-none cursor-pointer ${inputBg}`}
                >
                  <option value="glass" className={optionCls}>Glossy 3D Lens (Specular)</option>
                  <option value="frosted" className={optionCls}>Frosted Acrylic (Satin)</option>
                  <option value="smoked" className={optionCls}>Smoked Obsidian (Deep)</option>
                  <option value="solid" className={optionCls}>Anodized Carbon (Opaque)</option>
                </select>
              </div>

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

              {/* Surface Opacity Presets */}
              <div className="col-span-2">
                <div className="flex items-center justify-between mb-1.5">
                  <label className={`block text-xs font-semibold ${labelColor}`}>
                    Surface Opacity Preset
                  </label>
                  <span className="text-[11px] font-mono text-cyan-500 font-bold">
                    {formData.surfaceOpacity ?? 80}%
                  </span>
                </div>
                <div className="grid grid-cols-4 gap-1.5 p-1 rounded-xl border bg-black/10">
                  {([20, 50, 80, 100] as const).map((preset) => {
                    const isSelected = (formData.surfaceOpacity ?? 80) === preset;
                    return (
                      <button
                        key={preset}
                        type="button"
                        onClick={() =>
                          setFormData({ ...formData, surfaceOpacity: preset })
                        }
                        className={`py-1 text-xs font-bold rounded-lg transition-all cursor-pointer ${
                          isSelected
                            ? "bg-cyan-500 text-slate-950 shadow-sm"
                            : isLight
                            ? "text-slate-600 hover:text-slate-900 hover:bg-slate-200"
                            : "text-slate-400 hover:text-slate-200 hover:bg-white/10"
                        }`}
                      >
                        {preset}%
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Live 3D Theme & Volumetric Preview Card */}
              <div className="col-span-2">
                {(() => {
                  const previewMeta = getThemeMeta(formData.theme);
                  const opacity = (formData.surfaceOpacity ?? 80) / 100;
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
                          {formData.transparencyMode.toUpperCase()} ·{" "}
                          {formData.surfaceOpacity ?? 80}%
                        </span>
                      </div>

                      <div
                        className="rounded-2xl px-5 py-3 border flex items-center justify-between relative overflow-hidden transition-all duration-300"
                        style={{
                          backgroundColor: `rgba(${previewMeta.surfaceBase}, ${opacity})`,
                          borderColor: previewMeta.borderGlow,
                          color: previewMeta.text,
                          boxShadow: `0 10px 24px -4px rgba(0,0,0,0.8), inset 0 1.5px 1px ${previewMeta.rimHighlight}`,
                        }}
                      >
                        <div
                          className="absolute inset-0 pointer-events-none"
                          style={{
                            background: `radial-gradient(ellipse 55% 70% at 30% 50%, ${previewMeta.coreGlow} 0%, transparent 75%)`,
                          }}
                        />
                        <div className="relative z-10 flex items-baseline space-x-2">
                          <span className="text-xl font-bold tracking-tight tabular-nums">
                            12:28
                          </span>
                          <span
                            className="text-xs font-bold uppercase tracking-wider"
                            style={{ color: previewMeta.accent }}
                          >
                            AM
                          </span>
                        </div>
                        <div className="relative z-10 flex items-center space-x-2">
                          <span
                            className="text-[11px] truncate max-w-[200px] font-medium"
                            style={{ color: previewMeta.muted }}
                          >
                            {previewMeta.description}
                          </span>
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

              {/* Adaptive Album Accent */}
              <div className="flex items-center justify-between">
                <div>
                  <span className={`text-xs font-semibold ${labelColor}`}>
                    Adaptive Album Art Accent
                  </span>
                  <p className={`text-[11px] ${subtextColor}`}>
                    Extract ambient color from artwork for waveforms & glowing borders
                  </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    checked={formData.adaptiveAlbumAccent ?? true}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        adaptiveAlbumAccent: e.target.checked,
                      })
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
          <div className={`rounded-2xl p-5 border space-y-4 ${cardBg}`}>
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
                  {privacyState?.cameraActive && privacyState?.microphoneActive ? (
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
          <div className={`rounded-2xl p-5 border space-y-4 ${cardBg}`}>
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <h2 className={`text-xs font-bold uppercase tracking-wider flex items-center space-x-1.5 ${cardHeaderColor}`}>
                <Cpu className="w-4 h-4 text-amber-500" />
                <span>Process Memory Telemetry</span>
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
                    Main Process (PID {telemetry.mainProcess.pid}):
                  </span>
                  <span className="font-bold">
                    {telemetry.mainProcess.privateWorkingSetMb?.toFixed(2) ?? "—"} MB Private WS
                  </span>
                </div>
                {telemetry.childProcesses.map((child) => (
                  <div
                    key={child.pid}
                    className="flex flex-wrap justify-between gap-x-3 gap-y-1 opacity-70 pl-3"
                  >
                    <span>↳ {child.name} (PID {child.pid}):</span>
                    <span>
                      {child.privateWorkingSetMb?.toFixed(2) ?? "—"} MB
                    </span>
                  </div>
                ))}
                <div className="pt-2 border-t border-white/10 flex flex-wrap justify-between gap-x-3 gap-y-1 font-bold">
                  <span>Total Process Tree:</span>
                  <span className="text-amber-400">
                    {telemetry.totalPrivateWorkingSetMb?.toFixed(2) ?? "—"} MB Private WS · {telemetry.totalWorkingSetMb.toFixed(2)} MB WS
                  </span>
                </div>
                <div className="flex justify-between text-[10px] opacity-75">
                  <span>RAM Goal Budget (&lt; 50 MB target):</span>
                  <span
                    className={
                      telemetry.meetsTarget ? "text-emerald-400 font-bold" : "text-amber-400"
                    }
                  >
                    {telemetry.meetsTarget ? "✓ Within 50 MB Target" : "Under Maximum"}
                  </span>
                </div>
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
          <div className={`col-span-1 lg:col-span-2 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
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
          <div className={`col-span-1 lg:col-span-2 rounded-2xl p-5 border space-y-4 ${cardBg}`}>
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
                  Reference Only links original paths; Managed Copy stores a safe copy (≤ 50 MB)
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
                  <option value="copy" className={optionCls}>Managed Copy (≤ 50 MB)</option>
                </select>
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
        </div>
      </form>

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
