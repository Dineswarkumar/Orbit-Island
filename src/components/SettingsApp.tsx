import React, { useEffect, useState } from "react";
import { Settings as SettingsIcon, Save, RefreshCw, X, Check, Cpu } from "lucide-react";
import { useConfigStore } from "../core/stores/useConfigStore";
import { ipc } from "../core/ipc/client";
import { MemoryReport } from "../core/types/telemetry";
import { AppConfig } from "../core/types/config";

export const SettingsApp: React.FC = () => {
  const { config, loadConfig, updateConfig, isLoading, isSaving } = useConfigStore();
  const [formData, setFormData] = useState<AppConfig>(config);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">("idle");
  const [telemetry, setTelemetry] = useState<MemoryReport | null>(null);

  useEffect(() => {
    loadConfig();
    ipc.getMemoryMetrics().then(setTelemetry).catch(console.error);
  }, [loadConfig]);

  useEffect(() => {
    setFormData(config);
  }, [config]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const success = await updateConfig(formData);
    if (success) {
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2500);
    } else {
      setSaveStatus("error");
      setTimeout(() => setSaveStatus("idle"), 3500);
    }
  };

  const handleClose = async () => {
    try {
      await ipc.closeSettingsWindow();
    } catch (err) {
      console.error(err);
    }
  };

  const handleRefreshMemory = async () => {
    try {
      const report = await ipc.getMemoryMetrics();
      setTelemetry(report);
    } catch (err) {
      console.error(err);
    }
  };

  return (
    <div className="h-full w-full flex flex-col bg-slate-950 text-slate-100 font-sans select-none overflow-hidden">
      {/* Pinned Header */}
      <div className="shrink-0 flex items-center justify-between px-6 py-4 border-b border-slate-800 bg-slate-950/95 backdrop-blur z-10">
        <div className="flex items-center space-x-3">
          <div className="w-8 h-8 rounded-xl bg-cyan-600/20 border border-cyan-500/30 flex items-center justify-center text-cyan-400">
            <SettingsIcon className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-base font-bold tracking-tight">ORBIT Settings</h1>
            <p className="text-[11px] text-slate-400">Core Shell Preferences & System Diagnostics</p>
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

      {/* Main Scrollable Form Body (Scrolls via wheel, touchpad, and keyboard) */}
      <form
        id="settings-form"
        tabIndex={0}
        onSubmit={handleSave}
        className="flex-1 min-h-0 overflow-y-auto px-6 py-5 space-y-5 focus:outline-none"
      >
        {/* Section 1: Appearance & Placement */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Appearance & Placement
            </h2>
            <span className="text-[10px] text-cyan-400 bg-cyan-950/50 border border-cyan-800/40 px-2 py-0.5 rounded font-mono">
              Core Shell Scope
            </span>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">Theme</label>
                <span className="text-[10px] text-cyan-400/70 font-mono">Real-time</span>
              </div>
              <select
                value={formData.theme}
                onChange={(e) =>
                  setFormData({ ...formData, theme: e.target.value as AppConfig["theme"] })
                }
                className="w-full bg-slate-800/60 border border-slate-700/60 rounded-lg px-3 py-2 text-xs text-slate-300 focus:outline-none focus:border-cyan-500"
              >
                <option value="obsidian">Obsidian (Default Dark)</option>
                <option value="dark">Dark</option>
                <option value="dynamic">Dynamic</option>
                <option value="light">Porcelain (Light)</option>
                <option value="black">Black (OLED)</option>
                <option value="espresso">Espresso (Warm)</option>
                <option value="evergreen">Evergreen (Forest)</option>
                <option value="nord">Nord (Frost)</option>
              </select>
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">Position</label>
                <span className="text-[10px] text-emerald-400 font-mono">Active (Top Center)</span>
              </div>
              <select
                value={formData.position}
                onChange={(e) =>
                  setFormData({ ...formData, position: e.target.value as AppConfig["position"] })
                }
                className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500"
              >
                <option value="top-center">Top Center (Active)</option>
                <option value="top-left" disabled>Top Left (Milestone 4)</option>
                <option value="top-right" disabled>Top Right (Milestone 4)</option>
              </select>
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">Density</label>
                <span className="text-[10px] text-cyan-400/70 font-mono">Capsule</span>
              </div>
              <select
                value={formData.density}
                onChange={(e) =>
                  setFormData({ ...formData, density: e.target.value as AppConfig["density"] })
                }
                className="w-full bg-slate-800/60 border border-slate-700/60 rounded-lg px-3 py-2 text-xs text-slate-300 focus:outline-none focus:border-cyan-500"
              >
                <option value="compact">Compact (200x56 px)</option>
                <option value="balanced">Balanced (Recommended)</option>
                <option value="detailed">Detailed</option>
              </select>
            </div>

            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="block text-xs font-medium text-slate-300">Time Format</label>
                <span className="text-[10px] text-cyan-400/70 font-mono">Clock</span>
              </div>
              <select
                value={formData.timeFormat}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    timeFormat: e.target.value as AppConfig["timeFormat"],
                  })
                }
                className="w-full bg-slate-800/60 border border-slate-700/60 rounded-lg px-3 py-2 text-xs text-slate-300 focus:outline-none focus:border-cyan-500"
              >
                <option value="12h">12-Hour (AM/PM)</option>
                <option value="24h">24-Hour</option>
              </select>
            </div>
          </div>

          <div className="pt-2 flex items-center space-x-6 border-t border-slate-800/80">
            <label className="flex items-center space-x-2 text-xs cursor-pointer text-slate-200">
              <input
                type="checkbox"
                checked={formData.alwaysOnTop}
                onChange={(e) => setFormData({ ...formData, alwaysOnTop: e.target.checked })}
                className="rounded bg-slate-800 border-slate-700 text-cyan-500 focus:ring-0"
              />
              <span className="font-medium">Always on top</span>
              <span className="text-[10px] text-emerald-400 font-mono ml-1">(Live)</span>
            </label>

            <label className="flex items-center space-x-2 text-xs text-slate-400">
              <input
                type="checkbox"
                checked={formData.launchAtStartup}
                onChange={(e) => setFormData({ ...formData, launchAtStartup: e.target.checked })}
                className="rounded bg-slate-800 border-slate-700 text-cyan-500 focus:ring-0"
              />
              <span>Launch at Windows sign-in</span>
              <span className="text-[10px] text-slate-500 font-mono ml-1">(Milestone 4)</span>
            </label>
          </div>
        </div>

        {/* Section 2: Interaction & Animation Behavior */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Interaction & Animation
            </h2>
            <span className="text-[10px] text-slate-500 bg-slate-800 px-2 py-0.5 rounded">
              Core Shell
            </span>
          </div>

          <div className="space-y-3 text-xs text-slate-300">
            <div className="flex items-center justify-between p-2.5 rounded-lg bg-slate-800/40 border border-slate-800">
              <div>
                <div className="font-medium text-slate-200">Hover Expansion</div>
                <div className="text-[11px] text-slate-400">Expands compact capsule on pointer hover</div>
              </div>
              <span className="text-xs font-mono text-emerald-400 bg-emerald-950/60 px-2 py-0.5 rounded border border-emerald-800/40">
                Enabled
              </span>
            </div>

            <div className="flex items-center justify-between p-2.5 rounded-lg bg-slate-800/40 border border-slate-800">
              <div>
                <div className="font-medium text-slate-200">Auto-Collapse Delay</div>
                <div className="text-[11px] text-slate-400">Debounced return to compact pill on mouse leave</div>
              </div>
              <span className="text-xs font-mono text-cyan-400 bg-cyan-950/60 px-2 py-0.5 rounded border border-cyan-800/40">
                300 ms
              </span>
            </div>

            <div className="flex items-center justify-between p-2.5 rounded-lg bg-slate-800/40 border border-slate-800">
              <div>
                <div className="font-medium text-slate-200">Transition Curve</div>
                <div className="text-[11px] text-slate-400">Fluid stretch synchronized with native window region</div>
              </div>
              <span className="text-xs font-mono text-slate-300 bg-slate-800 px-2 py-0.5 rounded">
                250 ms ease-out
              </span>
            </div>
          </div>
        </div>

        {/* Section 3: Privacy & System Indicators */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              Privacy & System Indicators
            </h2>
            <span className="text-[10px] text-slate-500 bg-slate-800 px-2 py-0.5 rounded">
              PRD Section 5.4
            </span>
          </div>

          <div className="space-y-3 text-xs text-slate-300">
            <div className="flex items-center justify-between p-2.5 rounded-lg bg-slate-800/40 border border-slate-800">
              <div>
                <div className="font-medium text-slate-200">Local-First Guarantee</div>
                <div className="text-[11px] text-slate-400">Zero cloud analytics, tracking, or network telemetry</div>
              </div>
              <span className="text-xs font-mono text-emerald-400 bg-emerald-950/60 px-2 py-0.5 rounded border border-emerald-800/40">
                Active
              </span>
            </div>

            <div className="flex items-center justify-between p-2.5 rounded-lg bg-slate-800/40 border border-slate-800">
              <div>
                <div className="font-medium text-slate-200">Hardware & Sensor Privacy</div>
                <div className="text-[11px] text-slate-400">Only surface camera/microphone when OS exposes status</div>
              </div>
              <span className="text-xs font-mono text-slate-400 bg-slate-800 px-2 py-0.5 rounded">
                Milestone 3
              </span>
            </div>
          </div>
        </div>

        {/* Section 4: Real-time Memory Verification Card */}
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
              <div className="flex justify-between">
                <span className="text-slate-400">Main Process (PID {telemetry.mainProcess.pid}):</span>
                <span className="text-slate-200">
                  {telemetry.mainProcess.privateMb.toFixed(2)} MB Private Bytes · {telemetry.mainProcess.workingSetMb.toFixed(2)} MB WS
                </span>
              </div>
              {telemetry.childProcesses.map((child) => (
                <div key={child.pid} className="flex justify-between text-slate-400 pl-3">
                  <span>↳ {child.name} (PID {child.pid}):</span>
                  <span className="text-slate-300">
                    {child.privateMb.toFixed(2)} MB Private Bytes · {child.workingSetMb.toFixed(2)} MB WS
                  </span>
                </div>
              ))}
              <div className="pt-2 border-t border-slate-800 flex justify-between font-bold">
                <span className="text-white">Total Process Tree:</span>
                <span className="text-slate-200">
                  {telemetry.totalPrivateMb.toFixed(2)} MB Private (Total WS: {telemetry.totalWorkingSetMb.toFixed(2)} MB)
                </span>
              </div>
              <div className="text-[10px] text-slate-500 pt-1 border-t border-slate-900 leading-relaxed">
                Metric: Win32 K32GetProcessMemoryInfo PrivateUsage & WorkingSetSize. Targets: &lt;30 MB aspiration, &lt;50 MB target.
              </div>
            </div>
          ) : (
            <div className="text-xs text-slate-500 py-2">Measuring memory...</div>
          )}
        </div>

        {/* Section 5: About ORBIT */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
              About ORBIT
            </h2>
            <span className="text-[10px] text-cyan-400 font-mono">v0.1.0 Core Shell</span>
          </div>
          <div className="text-xs text-slate-400 space-y-1">
            <p>Windows 11 dynamic glanceable companion overlay.</p>
            <p className="text-[11px] text-slate-500 font-mono">
              Configuration file: %APPDATA%\com.orbit.island\config.json
            </p>
          </div>
        </div>
      </form>

      {/* Pinned Footer Actions (Always reachable across all window sizes) */}
      <div className="shrink-0 flex items-center justify-between px-6 py-3 border-t border-slate-800 bg-slate-950/95 backdrop-blur z-10">
        <div className="text-[11px] text-slate-500 truncate max-w-[280px]">
          Saved to: <span className="font-mono text-slate-400">%APPDATA%\com.orbit.island\config.json</span>
        </div>

        <div className="flex items-center space-x-3">
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
