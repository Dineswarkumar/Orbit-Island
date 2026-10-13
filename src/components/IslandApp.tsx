import React, { useEffect, useState } from "react";
import { Settings, Cpu, ShieldCheck } from "lucide-react";
import { ipc } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";
import { MemoryReport } from "../core/types/telemetry";

export const IslandApp: React.FC = () => {
  const { config, loadConfig } = useConfigStore();
  const [telemetry, setTelemetry] = useState<MemoryReport | null>(null);
  const [showTelemetryDetails, setShowTelemetryDetails] = useState(false);

  useEffect(() => {
    loadConfig();

    // Fetch memory metrics at startup
    ipc.getMemoryMetrics().then(setTelemetry).catch(console.error);

    // Refresh telemetry periodically every 3 seconds for baseline monitoring
    const interval = setInterval(() => {
      ipc.getMemoryMetrics().then(setTelemetry).catch(console.error);
    }, 3000);

    return () => clearInterval(interval);
  }, [loadConfig]);

  const handleOpenSettings = async () => {
    try {
      await ipc.openSettingsWindow();
    } catch (err) {
      console.error("Failed to open settings:", err);
    }
  };

  return (
    <div className="w-full h-full flex flex-col items-center justify-start pt-2 pointer-events-none">
      {/* Primary Island Capsule */}
      <div
        className="pointer-events-auto flex items-center justify-between px-4 py-2 rounded-full glass-pill text-white transition-all duration-300 shadow-2xl hover:border-cyan-500/40"
        style={{ minWidth: "380px" }}
      >
        {/* Left: Branding & Pill Icon */}
        <div className="flex items-center space-x-2.5">
          <div className="w-3 h-3 rounded-full bg-gradient-to-r from-cyan-400 to-indigo-500 animate-pulse shadow-sm shadow-cyan-400/50" />
          <span className="font-semibold tracking-wider text-xs uppercase text-slate-200">
            ORBIT
          </span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-cyan-950/80 text-cyan-300 border border-cyan-800/40">
            Foundation
          </span>
        </div>

        {/* Center: Live Memory Badge */}
        <button
          onClick={() => setShowTelemetryDetails((prev) => !prev)}
          title="Click to toggle memory telemetry details"
          className="flex items-center space-x-1.5 px-2.5 py-1 rounded-full bg-slate-900/60 hover:bg-slate-800/80 border border-white/5 transition-colors cursor-pointer text-xs"
        >
          <Cpu className="w-3.5 h-3.5 text-cyan-400" />
          <span className="font-mono text-[11px] text-slate-300">
            {telemetry ? `${telemetry.totalPrivateMb.toFixed(1)} MB` : "Measuring..."}
          </span>
          {telemetry?.meetsNormalGoal && (
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" title="Meets <50MB Goal" />
          )}
        </button>

        {/* Right: Settings trigger */}
        <button
          onClick={handleOpenSettings}
          title="Open ORBIT Settings"
          className="p-1.5 rounded-full hover:bg-white/10 text-slate-400 hover:text-white transition-colors cursor-pointer"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>

      {/* Expanded Telemetry Breakdown (if toggled) */}
      {showTelemetryDetails && telemetry && (
        <div className="pointer-events-auto mt-2 p-3.5 rounded-2xl glass-panel text-slate-200 text-xs w-[420px] shadow-2xl border border-white/10 animate-in fade-in slide-in-from-top-2 duration-200">
          <div className="flex items-center justify-between pb-2 mb-2 border-b border-white/10 font-medium">
            <span className="flex items-center space-x-1.5 text-cyan-300">
              <ShieldCheck className="w-4 h-4" />
              <span>Foundation Memory Telemetry</span>
            </span>
            <span
              className={`px-2 py-0.5 rounded text-[10px] font-semibold ${
                telemetry.meetsNormalGoal
                  ? "bg-emerald-950 text-emerald-300 border border-emerald-800"
                  : telemetry.meetsMaximumLimit
                  ? "bg-amber-950 text-amber-300 border border-amber-800"
                  : "bg-rose-950 text-rose-300 border border-rose-800"
              }`}
            >
              {telemetry.meetsNormalGoal
                ? "<50 MB Goal Met"
                : telemetry.meetsMaximumLimit
                ? "Within 100 MB Limit"
                : "Exceeds Limit"}
            </span>
          </div>

          <div className="space-y-1.5 font-mono text-[11px]">
            <div className="flex justify-between text-slate-300">
              <span>Main Process (PID {telemetry.mainProcess.pid}):</span>
              <span>{telemetry.mainProcess.privateMb.toFixed(2)} MB Private</span>
            </div>
            {telemetry.childProcesses.map((child) => (
              <div key={child.pid} className="flex justify-between text-slate-400 pl-2">
                <span>↳ {child.name} ({child.pid}):</span>
                <span>{child.privateMb.toFixed(2)} MB</span>
              </div>
            ))}
            <div className="pt-2 mt-1 border-t border-white/10 flex justify-between font-bold text-white text-xs">
              <span>Total Attributable RAM:</span>
              <span className="text-cyan-400">{telemetry.totalPrivateMb.toFixed(2)} MB</span>
            </div>
          </div>
          <div className="mt-2 text-[10px] text-slate-400 leading-tight">
            Active Theme: <span className="text-slate-200 font-semibold">{config.theme}</span> |
            Position: <span className="text-slate-200 font-semibold">{config.position}</span>
          </div>
        </div>
      )}
    </div>
  );
};
