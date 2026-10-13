import React, { useEffect, useState, useRef } from "react";
import { Settings, ChevronDown, Clock, Palette, Check } from "lucide-react";
import { ipc } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";
import { AppConfig } from "../core/types/config";

const THEME_OPTIONS: Array<{ id: AppConfig["theme"]; label: string; color: string }> = [
  { id: "obsidian", label: "Obsidian Dark", color: "#22d3ee" },
  { id: "espresso", label: "Espresso Warm", color: "#f59e0b" },
  { id: "evergreen", label: "Evergreen", color: "#10b981" },
  { id: "nord", label: "Nord Frost", color: "#818cf8" },
  { id: "light", label: "Porcelain Light", color: "#0ea5e9" },
];

export const IslandApp: React.FC = () => {
  const { config, loadConfig, updateConfig } = useConfigStore();

  const [isExpanded, setIsExpanded] = useState(false);
  const [currentTime, setCurrentTime] = useState(new Date());
  const [contextMenuPos, setContextMenuPos] = useState<{ x: number; y: number } | null>(null);

  const collapseTimerRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // Live system clock ticker (1-second interval)
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // Sync window size with expanded state
  const handleSetExpanded = async (expanded: boolean) => {
    setIsExpanded(expanded);
    if (!expanded) {
      setContextMenuPos(null);
    }
    try {
      await ipc.setIslandState(expanded ? "expanded" : "compact");
    } catch (err) {
      console.error("Failed to set island state:", err);
    }
  };

  // Hover expansion triggers
  const handleMouseEnter = () => {
    if (collapseTimerRef.current) {
      clearTimeout(collapseTimerRef.current);
      collapseTimerRef.current = null;
    }
    if (!isExpanded) {
      handleSetExpanded(true);
    }
  };

  const handleMouseLeave = () => {
    // Only auto-collapse if context menu is not open
    if (!contextMenuPos) {
      collapseTimerRef.current = setTimeout(() => {
        handleSetExpanded(false);
      }, 350);
    }
  };

  // Right-click context menu
  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (isExpanded) {
      setContextMenuPos({ x: Math.min(e.clientX, 310), y: Math.min(e.clientY, 70) });
    }
  };

  const handleOpenSettings = async (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setContextMenuPos(null);
    try {
      await ipc.openSettingsWindow();
    } catch (err) {
      console.error("Failed to open settings:", err);
    }
  };

  const handleSelectTheme = async (theme: AppConfig["theme"]) => {
    setContextMenuPos(null);
    await updateConfig({ theme });
  };

  // Time formatting
  const formatTime = (date: Date) => {
    const is12h = config.timeFormat === "12h";
    let hours = date.getHours();
    const minutes = String(date.getMinutes()).padStart(2, "0");
    const seconds = String(date.getSeconds()).padStart(2, "0");

    if (is12h) {
      const ampm = hours >= 12 ? "PM" : "AM";
      hours = hours % 12 || 12;
      return { timeStr: `${hours}:${minutes}`, seconds, ampm };
    }
    return { timeStr: `${String(hours).padStart(2, "0")}:${minutes}`, seconds, ampm: "" };
  };

  const formatDate = (date: Date) => {
    return date.toLocaleDateString(undefined, {
      weekday: "short",
      month: "short",
      day: "numeric",
    });
  };

  const timeDisplay = formatTime(currentTime);

  return (
    <div
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onContextMenu={handleContextMenu}
      onClick={() => setContextMenuPos(null)}
      className="w-screen h-screen flex items-start justify-center overflow-hidden select-none p-1 relative"
    >
      {/* Primary Island Container */}
      <div
        className={`w-full h-full flex flex-col justify-between text-white transition-all duration-200 ease-out shadow-2xl relative ${
          isExpanded
            ? "glass-panel rounded-[28px] p-4 cursor-default border border-white/15"
            : "glass-pill rounded-full px-4 py-2 cursor-pointer hover:border-cyan-500/50"
        }`}
      >
        {/* COMPACT PILL VIEW (200x56 px: No Orbit wordmark, No Settings icon, Larger clock font) */}
        {!isExpanded && (
          <div className="w-full h-full flex items-center justify-center space-x-2">
            <div className="w-2.5 h-2.5 rounded-full bg-cyan-400 shadow-sm shadow-cyan-400/80 animate-pulse" />
            <div className="flex items-baseline space-x-1 font-mono">
              <span className="font-bold text-base tracking-tight text-white">
                {timeDisplay.timeStr}
              </span>
              {timeDisplay.ampm && (
                <span className="text-[11px] font-semibold text-cyan-300">
                  {timeDisplay.ampm}
                </span>
              )}
            </div>
          </div>
        )}

        {/* EXPANDED PANEL VIEW (480x220 px: Clock with Date, Right-click hint, Settings button) */}
        {isExpanded && (
          <div className="w-full h-full flex flex-col justify-between">
            {/* Header: Title & Actions */}
            <div className="flex items-center justify-between pb-2 border-b border-white/10">
              <div className="flex items-center space-x-2">
                <div className="w-2.5 h-2.5 rounded-full bg-cyan-400 shadow-sm shadow-cyan-400/80" />
                <span className="text-xs font-bold uppercase tracking-wider text-slate-200">
                  ORBIT Home
                </span>
                <span className="text-[10px] px-1.5 py-0.5 rounded bg-cyan-950/80 text-cyan-300 border border-cyan-800/40 font-mono">
                  {config.theme}
                </span>
              </div>

              <div className="flex items-center space-x-1.5">
                <button
                  onClick={handleOpenSettings}
                  title="Open Settings"
                  className="p-1.5 rounded-lg hover:bg-white/10 text-slate-400 hover:text-white transition-colors cursor-pointer"
                >
                  <Settings className="w-4 h-4" />
                </button>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleSetExpanded(false);
                  }}
                  title="Collapse to Pill"
                  className="p-1.5 rounded-lg hover:bg-white/10 text-slate-400 hover:text-white transition-colors cursor-pointer"
                >
                  <ChevronDown className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* Main Center Display: Clean Large Home Clock & Date */}
            <div className="flex-1 my-2 bg-slate-900/60 rounded-2xl p-4 border border-white/5 flex flex-col items-center justify-center">
              <div className="flex items-center space-x-2 text-slate-400 text-xs mb-1">
                <Clock className="w-3.5 h-3.5 text-cyan-400" />
                <span className="font-medium text-slate-300">{formatDate(currentTime)}</span>
              </div>

              <div className="text-4xl font-black tracking-tight text-white font-mono flex items-baseline space-x-1.5">
                <span>{timeDisplay.timeStr}</span>
                <span className="text-sm font-semibold text-cyan-400">:{timeDisplay.seconds}</span>
                {timeDisplay.ampm && (
                  <span className="text-sm font-bold text-slate-300 ml-1">
                    {timeDisplay.ampm}
                  </span>
                )}
              </div>

              <div className="text-[11px] text-slate-500 mt-2 font-sans">
                Activity slot · Home clock priority
              </div>
            </div>

            {/* Bottom Status / Hint Bar */}
            <div className="flex items-center justify-between pt-1 text-[11px] text-slate-400">
              <span className="text-slate-400 text-[11px]">
                Right-click for Themes & Menu
              </span>
              <span className="text-slate-500 font-mono text-[10px]">
                Move mouse away to collapse
              </span>
            </div>
          </div>
        )}
      </div>

      {/* Custom Context Menu on Right Click */}
      {isExpanded && contextMenuPos && (
        <div
          onClick={(e) => e.stopPropagation()}
          style={{ top: contextMenuPos.y, left: contextMenuPos.x }}
          className="absolute z-50 w-48 bg-slate-900/95 backdrop-blur-xl border border-white/15 rounded-xl shadow-2xl py-1.5 text-xs text-slate-200 font-sans"
        >
          <div className="px-3 py-1 font-semibold text-[10px] uppercase tracking-wider text-slate-400 border-b border-white/10 flex items-center space-x-1.5">
            <Palette className="w-3 h-3 text-cyan-400" />
            <span>Select Theme</span>
          </div>

          <div className="py-1">
            {THEME_OPTIONS.map((t) => (
              <button
                key={t.id}
                onClick={() => handleSelectTheme(t.id)}
                className="w-full px-3 py-1.5 flex items-center justify-between hover:bg-white/10 text-left transition-colors cursor-pointer"
              >
                <div className="flex items-center space-x-2">
                  <span
                    className="w-2.5 h-2.5 rounded-full border border-white/20"
                    style={{ backgroundColor: t.color }}
                  />
                  <span>{t.label}</span>
                </div>
                {config.theme === t.id && <Check className="w-3.5 h-3.5 text-cyan-400" />}
              </button>
            ))}
          </div>

          <div className="border-t border-white/10 mt-1 pt-1">
            <button
              onClick={handleOpenSettings}
              className="w-full px-3 py-1.5 flex items-center space-x-2 hover:bg-white/10 text-left transition-colors cursor-pointer"
            >
              <Settings className="w-3.5 h-3.5 text-slate-400" />
              <span>ORBIT Settings...</span>
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
