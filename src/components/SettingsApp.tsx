import React, { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
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
} from "lucide-react";
import { useConfigStore } from "../core/stores/useConfigStore";
import { ipc } from "../core/ipc/client";
import { MemoryReport } from "../core/types/telemetry";
import { AppConfig } from "../core/types/config";
import { THEMES, getThemeMeta } from "../core/theme";
import packageJson from "../../package.json";

export const SettingsApp: React.FC = () => {
  const { config, loadConfig, updateConfig, isLoading, isSaving } =
    useConfigStore();
  const [formData, setFormData] = useState<AppConfig>(config);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">(
    "idle"
  );
  const [telemetry, setTelemetry] = useState<MemoryReport | null>(null);
  const [telemetryError, setTelemetryError] = useState<string | null>(null);
  const [notifAccessStatus, setNotifAccessStatus] =
    useState<string>("unspecified");
  const [isRequestingNotif, setIsRequestingNotif] = useState(false);
  const [clearNotifMessage, setClearNotifMessage] = useState<string | null>(
    null
  );
  const saveStatusTimer = React.useRef<ReturnType<typeof setTimeout> | null>(
    null
  );

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

    // Keep Settings window synchronized when theme or settings change via the Island's context menu
    const unlistenConfig = listen<AppConfig>("config-changed", (event) => {
      useConfigStore.setState({ config: event.payload });
      setFormData(event.payload);
    });

    return () => {
      unlistenConfig.then((unlisten) => unlisten());
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
      setClearNotifMessage("All notifications cleared.");
      setTimeout(() => setClearNotifMessage(null), 3000);
    } catch {
      setClearNotifMessage("Failed to clear notifications.");
      setTimeout(() => setClearNotifMessage(null), 3000);
    }
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

  const handleScrollKeys = (e: React.KeyboardEvent<HTMLFormElement>) => {
    if (e.target !== e.currentTarget) return;
    const viewport = e.currentTarget;
    const page = Math.max(120, viewport.clientHeight * 0.8);
    const offsets: Record<string, number> = {
      PageDown: page,
      PageUp: -page,
      ArrowDown: 48,
      ArrowUp: -48,
      Home: -viewport.scrollTop,
      End: viewport.scrollHeight - viewport.scrollTop,
    };
    if (e.key in offsets) {
      e.preventDefault();
      viewport.scrollBy({ top: offsets[e.key], behavior: "auto" });
    }
  };

  const handleClose = async () => {
    try {
      await ipc.closeSettingsWindow();
    } catch (err) {
      console.error(err);
    }
  };

  const [resetMessage, setResetMessage] = useState<string | null>(null);

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

  const handleRefreshMemory = async () => {
    try {
      const report = await ipc.getMemoryMetrics();
      setTelemetry(report);
      setTelemetryError(null);
    } catch (err) {
      setTelemetryError(String(err));
    }
  };

  return (
    <div className="h-full w-full flex flex-col bg-slate-950 text-slate-100 font-sans select-none overflow-hidden">
      {/* Pinned Header */}
      <div className="shrink-0 flex items-center justify-between px-6 py-4 border-b border-slate-800 bg-slate-950/95 backdrop-blur z-10">
        <div className="flex items-center space-x-3">
          <div className="w-8 h-8 rounded-xl bg-cyan-600/20 border border-cyan-500/30 flex items-center justify-center text-cyan-400 shadow-[0_0_12px_rgba(6,182,212,0.25)]">
            <SettingsIcon className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center space-x-2">
              <h1 className="text-base font-bold tracking-tight">
                ORBIT Settings
              </h1>
              <span className="text-[10px] font-mono font-semibold px-2 py-0.5 rounded-full bg-cyan-500/15 text-cyan-300 border border-cyan-500/30">
                v{packageJson.version}
              </span>
            </div>
            <p className="text-[11px] text-slate-400">
              3D Orbital Shell, Material Themes & System Diagnostics
            </p>
          </div>
        </div>
        <button
          onClick={handleClose}
          className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition-colors cursor-pointer"
          title="Close Settings Window"
        >
          <X className="w-5 h-5" />
        </button>
      </div>

      {/* Main Scrollable Form Body */}
      <form
        id="settings-form"
        aria-label="Settings sections"
        tabIndex={0}
        onSubmit={handleSave}
        onKeyDown={handleScrollKeys}
        className="flex-1 min-h-0 overflow-y-auto overscroll-y-contain px-6 py-5 space-y-5 focus:outline-none"
      >
        {/* Section 1: 3D Appearance, Material & Placement */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 flex items-center space-x-1.5">
              <Sparkles className="w-3.5 h-3.5 text-cyan-400" />
              <span>3D Appearance, Material & Placement</span>
            </h2>
            <span className="text-[10px] text-cyan-400 bg-cyan-950/50 border border-cyan-800/40 px-2 py-0.5 rounded font-mono">
              3D Shell Active
            </span>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">
                  3D Orbital Theme
                </label>
                <span className="text-[10px] text-cyan-400/70 font-mono">
                  Volumetric
                </span>
              </div>
              <select
                value={formData.theme}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    theme: e.target.value as AppConfig["theme"],
                  })
                }
                className="w-full bg-slate-800/60 border border-slate-700/60 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 cursor-pointer"
              >
                {THEMES.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.label}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">
                  3D Glass Material
                </label>
                <span className="text-[10px] text-cyan-400/70 font-mono">
                  Surface finish
                </span>
              </div>
              <select
                value={formData.transparencyMode}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    transparencyMode: e.target
                      .value as AppConfig["transparencyMode"],
                  })
                }
                className="w-full bg-slate-800/60 border border-slate-700/60 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 cursor-pointer"
              >
                <option value="glass">Glossy 3D Lens (Specular)</option>
                <option value="frosted">Frosted Acrylic 3D (Satin)</option>
                <option value="smoked">Smoked Obsidian 3D (Deep)</option>
                <option value="solid">Anodized Carbon 3D (Opaque)</option>
              </select>
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">
                  Screen Position
                </label>
                <span className="text-[10px] text-emerald-400 font-mono">
                  Desktop Anchor
                </span>
              </div>
              <select
                value={formData.position}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    position: e.target.value as AppConfig["position"],
                  })
                }
                className="w-full bg-slate-800/60 border border-slate-700/60 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 cursor-pointer"
              >
                <option value="top-center">Top Center (Default)</option>
                <option value="top-left">Top Left</option>
                <option value="top-right">Top Right</option>
              </select>
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">
                  Time Format
                </label>
                <span className="text-[10px] text-cyan-400/70 font-mono">
                  Clock
                </span>
              </div>
              <select
                value={formData.timeFormat}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    timeFormat: e.target.value as AppConfig["timeFormat"],
                  })
                }
                className="w-full bg-slate-800/60 border border-slate-700/60 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500 cursor-pointer"
              >
                <option value="12h">12-Hour (AM/PM)</option>
                <option value="24h">24-Hour</option>
              </select>
            </div>

            <div className="col-span-2">
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">
                  Surface Opacity
                </label>
                <span className="text-[10px] text-cyan-400/70 font-mono">
                  {formData.surfaceOpacity ?? 80}%
                </span>
              </div>
              <div className="grid grid-cols-4 gap-1.5 p-1 bg-slate-800/60 border border-slate-700/60 rounded-lg">
                {([20, 50, 80, 100] as const).map((preset) => {
                  const isSelected = (formData.surfaceOpacity ?? 80) === preset;
                  return (
                    <button
                      key={preset}
                      type="button"
                      onClick={() =>
                        setFormData({ ...formData, surfaceOpacity: preset })
                      }
                      className={`py-1 text-xs font-semibold rounded-md transition-all cursor-pointer ${
                        isSelected
                          ? "bg-cyan-500 text-slate-950 shadow-sm"
                          : "text-slate-400 hover:text-slate-200 hover:bg-slate-700/50"
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
                  <div className="rounded-xl p-3.5 border border-slate-800 bg-slate-950/80 space-y-2.5">
                    <div className="flex items-center justify-between text-[11px]">
                      <span className="font-semibold text-slate-200 flex items-center space-x-2">
                        <span
                          className="w-2.5 h-2.5 rounded-full inline-block"
                          style={{
                            backgroundColor: previewMeta.accent,
                            boxShadow: `0 0 8px ${previewMeta.accent}`,
                          }}
                        />
                        <span>{previewMeta.label}</span>
                      </span>
                      <span className="text-slate-400 font-mono text-[10px]">
                        {formData.transparencyMode.toUpperCase()} ·{" "}
                        {formData.surfaceOpacity ?? 80}%
                      </span>
                    </div>

                    {/* 3D Preview Stage */}
                    <div
                      className="rounded-2xl px-5 py-3 border flex items-center justify-between relative overflow-hidden transition-all duration-300"
                      style={{
                        backgroundColor: `rgba(${previewMeta.surfaceBase}, ${opacity})`,
                        borderColor: previewMeta.borderGlow,
                        color: previewMeta.text,
                        boxShadow: `0 10px 24px -4px rgba(0,0,0,0.8), inset 0 1.5px 1px ${previewMeta.rimHighlight}, inset 0 -2px 4px rgba(0,0,0,0.8)`,
                      }}
                    >
                      {/* Volumetric core preview aura */}
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
                          className="text-[11px] truncate max-w-[240px] font-medium"
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

          <div className="pt-2 flex items-center space-x-6 border-t border-slate-800/80">
            <label className="flex items-center space-x-2 text-xs cursor-pointer text-slate-200">
              <input
                type="checkbox"
                checked={formData.alwaysOnTop}
                onChange={(e) =>
                  setFormData({ ...formData, alwaysOnTop: e.target.checked })
                }
                className="rounded bg-slate-800 border-slate-700 text-cyan-500 focus:ring-0"
              />
              <span className="font-medium">Always on top</span>
              <span className="text-[10px] text-slate-500 font-mono ml-1">
                (Applied on save)
              </span>
            </label>
          </div>
        </div>

        {/* Notifications & Privacy Settings Card */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 flex items-center space-x-2">
              <Bell className="w-4 h-4 text-cyan-400" />
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
              <span className="text-[11px] font-mono capitalize text-slate-300">
                Windows Access: {notifAccessStatus}
              </span>
            </div>
          </div>

          {notifAccessStatus !== "allowed" && (
            <div className="bg-amber-950/40 border border-amber-800/60 rounded-lg p-3 text-xs text-amber-200/90 space-y-2">
              <p>
                ORBIT needs Windows Notification Listener access to display
                compact toasts and previews on the island. No notification
                content is transmitted or logged externally.
              </p>
              <button
                type="button"
                onClick={handleRequestNotifAccess}
                disabled={isRequestingNotif}
                className="px-3 py-1.5 bg-amber-600 hover:bg-amber-500 text-white rounded-md text-xs font-semibold transition-colors cursor-pointer disabled:opacity-50"
              >
                {isRequestingNotif
                  ? "Requesting Windows Permission..."
                  : "Request Access Permission"}
              </button>
            </div>
          )}

          <div className="space-y-3 pt-1">
            <label className="flex items-start space-x-2.5 text-xs cursor-pointer text-slate-200">
              <input
                type="checkbox"
                checked={formData.notificationsEnabled}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    notificationsEnabled: e.target.checked,
                  })
                }
                className="mt-0.5 rounded bg-slate-800 border-slate-700 text-cyan-500 focus:ring-0"
              />
              <div>
                <span className="font-medium">Enable Island Notifications</span>
                <p className="text-[11px] text-slate-400">
                  Surface compact 3D capsules for incoming Windows notifications.
                </p>
              </div>
            </label>

            <label className="flex items-start space-x-2.5 text-xs cursor-pointer text-slate-200">
              <input
                type="checkbox"
                checked={formData.privacyMode}
                onChange={(e) =>
                  setFormData({ ...formData, privacyMode: e.target.checked })
                }
                className="mt-0.5 rounded bg-slate-800 border-slate-700 text-cyan-500 focus:ring-0"
              />
              <div>
                <span className="font-medium flex items-center space-x-1.5">
                  <Shield className="w-3.5 h-3.5 text-cyan-400" />
                  <span>Privacy Mode (Hide notification message content)</span>
                </span>
                <p className="text-[11px] text-slate-400">
                  Shows only the app name and a generic label. Full message text
                  is never rendered.
                </p>
              </div>
            </label>

            <label className="flex items-start space-x-2.5 text-xs cursor-pointer text-slate-200">
              <input
                type="checkbox"
                checked={formData.showPrivacyIndicators}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    showPrivacyIndicators: e.target.checked,
                  })
                }
                className="mt-0.5 rounded bg-slate-800 border-slate-700 text-cyan-500 focus:ring-0"
              />
              <div>
                <span className="font-medium">
                  Camera & Microphone Activity Indicators
                </span>
                <p className="text-[11px] text-slate-400">
                  Display glowing indicator on pill: bright green (camera only),
                  yellow (microphone only), or bright cyan (both active).
                </p>
              </div>
            </label>

            <div className="pt-2 flex items-center justify-between border-t border-slate-800/80">
              <div>
                <span className="text-xs font-medium text-slate-200">
                  Compact Notification Duration
                </span>
                <p className="text-[11px] text-slate-400">
                  Seconds before compact notification auto-dismisses.
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
                  className="w-16 bg-slate-800/60 border border-slate-700/60 rounded-lg px-2.5 py-1 text-xs text-slate-200 text-center focus:outline-none focus:border-cyan-500"
                />
                <span className="text-xs text-slate-400">sec</span>
              </div>
            </div>

            <div className="pt-2 flex items-center justify-between border-t border-slate-800/80">
              <div>
                <span className="text-xs font-medium text-slate-200">
                  Clear Notification Queue
                </span>
                <p className="text-[11px] text-slate-400">
                  Dismiss all held notifications from ORBIT's queue.
                </p>
              </div>
              <div className="flex items-center space-x-3">
                {clearNotifMessage && (
                  <span className="text-xs text-cyan-400">
                    {clearNotifMessage}
                  </span>
                )}
                <button
                  type="button"
                  onClick={handleClearNotifications}
                  className="px-3 py-1 bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 rounded-lg text-xs font-medium transition-colors cursor-pointer"
                >
                  Clear Queue
                </button>
              </div>
            </div>
          </div>
        </div>

        {/* Real-time Memory Verification Card */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 flex items-center space-x-2">
              <Cpu className="w-4 h-4 text-cyan-400" />
              <span>Process Memory Telemetry</span>
            </h2>
            <button
              type="button"
              onClick={handleRefreshMemory}
              className="flex items-center space-x-1 text-[11px] text-cyan-400 hover:text-cyan-300 cursor-pointer"
            >
              <RefreshCw className="w-3 h-3" />
              <span>Refresh</span>
            </button>
          </div>

          {telemetry ? (
            <div className="space-y-2 text-xs font-mono bg-slate-950 p-3 rounded-lg border border-slate-800/80">
              <div className="flex flex-wrap justify-between gap-x-3 gap-y-1">
                <span className="text-slate-400">
                  Main Process (PID {telemetry.mainProcess.pid}):
                </span>
                <span className="text-slate-200">
                  {telemetry.mainProcess.privateWorkingSetMb?.toFixed(2) ??
                    "Unavailable"}{" "}
                  MB Private WS ·{" "}
                  {telemetry.mainProcess.privateMb.toFixed(2)} MB Private Bytes
                  · {telemetry.mainProcess.workingSetMb.toFixed(2)} MB WS
                </span>
              </div>
              {telemetry.childProcesses.map((child) => (
                <div
                  key={child.pid}
                  className="flex flex-wrap justify-between gap-x-3 gap-y-1 text-slate-400 pl-3"
                >
                  <span>
                    ↳ {child.name} (PID {child.pid}):
                  </span>
                  <span className="text-slate-300">
                    {child.privateWorkingSetMb?.toFixed(2) ?? "Unavailable"} MB
                    Private WS · {child.privateMb.toFixed(2)} MB Private Bytes ·{" "}
                    {child.workingSetMb.toFixed(2)} MB WS
                  </span>
                </div>
              ))}
              {telemetry.unmeasuredDescendantCount > 0 && (
                <div className="text-amber-400">
                  {telemetry.unmeasuredDescendantCount} child process(es) could
                  not be measured; totals are incomplete.
                </div>
              )}
              <div className="pt-2 border-t border-slate-800 flex flex-wrap justify-between gap-x-3 gap-y-1 font-bold">
                <span className="text-white">Total Process Tree:</span>
                <span className="text-slate-200">
                  {telemetry.totalPrivateWorkingSetMb?.toFixed(2) ??
                    "Unavailable"}{" "}
                  MB Private WS · {telemetry.totalPrivateMb.toFixed(2)} MB
                  Private Bytes · {telemetry.totalWorkingSetMb.toFixed(2)} MB WS
                </span>
              </div>
              <div className="flex justify-between text-[10px] text-slate-400">
                <span>RAM goal status (Private Working Set):</span>
                <span
                  className={
                    telemetry.meetsMaximum
                      ? "text-emerald-400"
                      : "text-rose-400"
                  }
                >
                  {telemetry.totalPrivateWorkingSetMb === null
                    ? "Unavailable"
                    : telemetry.meetsMaximum
                    ? telemetry.meetsAspiration
                      ? "Below 30 MB aspiration"
                      : telemetry.meetsTarget
                      ? "Within 50 MB target"
                      : "Below 100 MB maximum"
                    : "Above 100 MB maximum"}
                </span>
              </div>
            </div>
          ) : (
            <div className="text-xs text-slate-500 py-2">
              {telemetryError
                ? `Memory measurement failed: ${telemetryError}`
                : "Measuring memory..."}
            </div>
          )}
        </div>

        {/* Startup & Reset (Phase 3) */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Startup &amp; Reset
            </h2>
            {resetMessage && (
              <span className="text-[11px] text-cyan-400 font-medium">
                {resetMessage}
              </span>
            )}
          </div>

          {/* Launch at Startup */}
          <div className="flex items-center justify-between">
            <div>
              <span className="text-xs font-medium text-slate-200">
                Launch ORBIT at Windows Sign-In
              </span>
              <p className="text-[11px] text-slate-500">
                Automatically run ORBIT in the background when your PC starts
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
              <div className="w-9 h-5 bg-slate-800 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-cyan-600"></div>
            </label>
          </div>

          {/* Reset actions */}
          <div className="flex flex-wrap items-center gap-3 pt-2 border-t border-slate-800/80">
            <button
              type="button"
              onClick={handleResetDefaults}
              className="px-3 py-1.5 rounded-lg text-xs font-medium bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white transition-colors cursor-pointer"
            >
              Reset Settings to Defaults
            </button>
            <button
              type="button"
              onClick={handleClearLocalData}
              className="px-3 py-1.5 rounded-lg text-xs font-medium bg-rose-950/40 hover:bg-rose-900/50 text-rose-300 hover:text-rose-200 border border-rose-800/30 transition-colors cursor-pointer"
            >
              Clear Local Reminders &amp; Shelf
            </button>
          </div>
        </div>

        {/* About ORBIT */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              About ORBIT
            </h2>
            <span className="text-[10px] text-cyan-400 font-mono">
              v{packageJson.version}
            </span>
          </div>
          <div className="text-xs text-slate-400 space-y-1">
            <p>Windows 11 3D Dynamic Island companion overlay.</p>
            <p className="text-[11px] text-slate-500 font-mono">
              Configuration file: %APPDATA%\com.orbit.island\config.json
            </p>
          </div>
        </div>
      </form>

      {/* Pinned Footer Actions */}
      <div className="shrink-0 flex flex-wrap items-center justify-between gap-x-3 gap-y-2 px-4 sm:px-6 py-3 border-t border-slate-800 bg-slate-950/95 backdrop-blur z-10">
        <div className="min-w-0 flex-1 text-[11px] text-slate-500 truncate">
          Saved to:{" "}
          <span className="font-mono text-slate-400">
            %APPDATA%\com.orbit.island\config.json
          </span>
        </div>

        <div className="flex shrink-0 items-center space-x-3">
          {saveStatus === "success" && (
            <span className="flex items-center space-x-1 text-xs text-emerald-400">
              <Check className="w-4 h-4" />
              <span>Saved successfully!</span>
            </span>
          )}
          {saveStatus === "error" && (
            <span className="flex items-center space-x-1 text-xs text-rose-400">
              <X className="w-4 h-4" />
              <span>Save failed!</span>
            </span>
          )}
          <button
            type="submit"
            form="settings-form"
            disabled={isSaving || isLoading}
            className="flex items-center space-x-2 bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-semibold px-4 py-2 rounded-lg transition-colors cursor-pointer disabled:opacity-50"
          >
            <Save className="w-4 h-4" />
            <span>{isSaving ? "Saving..." : "Save Preferences"}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
