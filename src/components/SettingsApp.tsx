import React, { useEffect, useState } from "react";
import { Settings as SettingsIcon, Save, RefreshCw, X, Check, Cpu } from "lucide-react";
import { useConfigStore } from "../core/stores/useConfigStore";
import { ipc } from "../core/ipc/client";
import { MemoryReport } from "../core/types/telemetry";
import { AppConfig } from "../core/types/config";

export const SettingsApp: React.FC = () => {
  const { config, loadConfig, updateConfig, isLoading, isSaving } = useConfigStore();
  const [formData, setFormData] = useState<AppConfig>(config);
  const [saveSuccess, setSaveSuccess] = useState(false);
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
    await updateConfig(formData);
    setSaveSuccess(true);
    setTimeout(() => setSaveSuccess(false), 2500);
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
    <div className="min-h-screen bg-slate-950 text-slate-100 p-6 flex flex-col font-sans select-none">
      {/* Header */}
      <div className="flex items-center justify-between pb-4 border-b border-slate-800">
        <div className="flex items-center space-x-3">
          <div className="w-8 h-8 rounded-xl bg-cyan-600/20 border border-cyan-500/30 flex items-center justify-center text-cyan-400">
            <SettingsIcon className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-lg font-bold tracking-tight">ORBIT Settings</h1>
            <p className="text-xs text-slate-400">Foundation Preferences & System Telemetry</p>
          </div>
        </div>
        <button
          onClick={handleClose}
          className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition-colors"
          title="Close Settings Window"
        >
          <X className="w-5 h-5" />
        </button>
      </div>

      {/* Main Content Form */}
      <form onSubmit={handleSave} className="flex-1 mt-6 space-y-6">
        {/* Appearance Section */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-4">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400">
            Appearance & Placement
          </h2>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-xs font-medium text-slate-300 mb-1">Theme</label>
              <select
                value={formData.theme}
                onChange={(e) =>
                  setFormData({ ...formData, theme: e.target.value as AppConfig["theme"] })
                }
                className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500"
              >
                <option value="obsidian">Obsidian (Default Dark)</option>
                <option value="espresso">Espresso</option>
                <option value="evergreen">Evergreen</option>
                <option value="nord">Nord</option>
                <option value="light">Porcelain (Light)</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-medium text-slate-300 mb-1">Position</label>
              <select
                value={formData.position}
                onChange={(e) =>
                  setFormData({ ...formData, position: e.target.value as AppConfig["position"] })
                }
                className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500"
              >
                <option value="top-center">Top Center (Default)</option>
                <option value="top-left">Top Left</option>
                <option value="top-right">Top Right</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-medium text-slate-300 mb-1">Density</label>
              <select
                value={formData.density}
                onChange={(e) =>
                  setFormData({ ...formData, density: e.target.value as AppConfig["density"] })
                }
                className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500"
              >
                <option value="compact">Compact</option>
                <option value="balanced">Balanced (Recommended)</option>
                <option value="detailed">Detailed</option>
              </select>
            </div>

            <div>
              <label className="block text-xs font-medium text-slate-300 mb-1">Time Format</label>
              <select
                value={formData.timeFormat}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    timeFormat: e.target.value as AppConfig["timeFormat"],
                  })
                }
                className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-200 focus:outline-none focus:border-cyan-500"
              >
                <option value="12h">12-Hour (AM/PM)</option>
                <option value="24h">24-Hour</option>
              </select>
            </div>
          </div>

          <div className="pt-2 flex items-center space-x-6">
            <label className="flex items-center space-x-2 text-xs cursor-pointer text-slate-300">
              <input
                type="checkbox"
                checked={formData.alwaysOnTop}
                onChange={(e) => setFormData({ ...formData, alwaysOnTop: e.target.checked })}
                className="rounded bg-slate-800 border-slate-700 text-cyan-500 focus:ring-0"
              />
              <span>Always on top</span>
            </label>

            <label className="flex items-center space-x-2 text-xs cursor-pointer text-slate-300">
              <input
                type="checkbox"
                checked={formData.launchAtStartup}
                onChange={(e) => setFormData({ ...formData, launchAtStartup: e.target.checked })}
                className="rounded bg-slate-800 border-slate-700 text-cyan-500 focus:ring-0"
              />
              <span>Launch at Windows sign-in</span>
            </label>
          </div>
        </div>

        {/* Real-time Memory Verification Card */}
        <div className="bg-slate-900/60 rounded-xl p-4 border border-slate-800 space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="text-xs font-semibold uppercase tracking-wider text-slate-400 flex items-center space-x-2">
              <Cpu className="w-4 h-4 text-cyan-400" />
              <span>Foundation Memory Telemetry (Target: &lt;50 MB, Max: 100 MB)</span>
            </h2>
            <button
              type="button"
              onClick={handleRefreshMemory}
              className="flex items-center space-x-1 text-[11px] text-cyan-400 hover:text-cyan-300"
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
                  {telemetry.mainProcess.privateMb.toFixed(2)} MB Private (Working Set:{" "}
                  {telemetry.mainProcess.workingSetMb.toFixed(2)} MB)
                </span>
              </div>
              {telemetry.childProcesses.map((child) => (
                <div key={child.pid} className="flex justify-between text-slate-400 pl-3">
                  <span>↳ {child.name} (PID {child.pid}):</span>
                  <span className="text-slate-300">{child.privateMb.toFixed(2)} MB Private</span>
                </div>
              ))}
              <div className="pt-2 border-t border-slate-800 flex justify-between font-bold">
                <span className="text-white">Total Attributable Memory:</span>
                <span
                  className={
                    telemetry.meetsNormalGoal
                      ? "text-emerald-400"
                      : telemetry.meetsMaximumLimit
                      ? "text-amber-400"
                      : "text-rose-400"
                  }
                >
                  {telemetry.totalPrivateMb.toFixed(2)} MB Private Usage
                </span>
              </div>
              <div className="text-[10px] text-slate-500 pt-1">
                Method: {telemetry.measurementMethod}
              </div>
            </div>
          ) : (
            <div className="text-xs text-slate-500 py-2">Measuring memory...</div>
          )}
        </div>

        {/* Footer Actions */}
        <div className="flex items-center justify-between pt-2">
          <div className="text-xs text-slate-500">
            Saved to: <span className="font-mono text-slate-400">%APPDATA%\ORBIT\config.json</span>
          </div>

          <div className="flex items-center space-x-3">
            {saveSuccess && (
              <span className="flex items-center space-x-1 text-xs text-emerald-400">
                <Check className="w-4 h-4" />
                <span>Saved successfully!</span>
              </span>
            )}
            <button
              type="submit"
              disabled={isSaving || isLoading}
              className="flex items-center space-x-2 bg-cyan-600 hover:bg-cyan-500 text-white text-xs font-semibold px-4 py-2 rounded-lg transition-colors cursor-pointer disabled:opacity-50"
            >
              <Save className="w-4 h-4" />
              <span>{isSaving ? "Saving..." : "Save Preferences"}</span>
            </button>
          </div>
        </div>
      </form>
    </div>
  );
};
