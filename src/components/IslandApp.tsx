import React, { useEffect } from "react";
import { Settings } from "lucide-react";
import { ipc } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";

export const IslandApp: React.FC = () => {
  const { loadConfig } = useConfigStore();

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleOpenSettings = async () => {
    try {
      await ipc.openSettingsWindow();
    } catch (err) {
      console.error("Failed to open settings:", err);
    }
  };

  return (
    <div className="w-screen h-screen flex items-center justify-center p-1 overflow-hidden select-none">
      {/* Primary Island Capsule */}
      <div
        className="w-full h-full flex items-center justify-between px-4 py-2 rounded-full glass-pill text-white transition-all duration-300 shadow-2xl hover:border-cyan-500/40"
      >
        {/* Left: Branding & Status */}
        <div className="flex items-center space-x-2.5">
          <div className="w-2.5 h-2.5 rounded-full bg-gradient-to-r from-cyan-400 to-indigo-500 shadow-sm shadow-cyan-400/50" />
          <span className="font-semibold tracking-wider text-xs uppercase text-slate-200">
            ORBIT
          </span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-cyan-950/80 text-cyan-300 border border-cyan-800/40 font-mono">
            Foundation
          </span>
        </div>

        {/* Right: Settings trigger */}
        <button
          onClick={handleOpenSettings}
          title="Open ORBIT Settings"
          className="p-1 rounded-full hover:bg-white/10 text-slate-400 hover:text-white transition-colors cursor-pointer"
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
};
