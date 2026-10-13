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
  const isTransitioningRef = useRef<boolean>(false);

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

  // Hover expansion triggers
  const handleMouseEnter = async () => {
    if (collapseTimerRef.current) {
      clearTimeout(collapseTimerRef.current);
      collapseTimerRef.current = null;
    }
    if (!isExpanded && !isTransitioningRef.current) {
      isTransitioningRef.current = true;
      try {
        // Expand native window and region first so webview has room to render
        await ipc.setIslandState("expanded");
      } catch (err) {
        console.error("Failed to set island state to expanded:", err);
      }
      setIsExpanded(true);
      isTransitioningRef.current = false;
    }
  };

  const handleMouseLeave = () => {
    // Only auto-collapse if context menu is not open
    if (!contextMenuPos && isExpanded) {
      if (collapseTimerRef.current) {
        clearTimeout(collapseTimerRef.current);
      }
      collapseTimerRef.current = setTimeout(() => {
        // 1. Animate visual shrink back to compact pill in webview
        setIsExpanded(false);
        // 2. Once the 250ms CSS shrink animation completes, resize native window back to 200x56
        setTimeout(async () => {
          try {
            await ipc.setIslandState("compact");
          } catch (err) {
            console.error("Failed to restore compact island state:", err);
          }
        }, 260);
      }, 350);
    }
  };

  const handleCollapseClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setContextMenuPos(null);
    if (collapseTimerRef.current) {
      clearTimeout(collapseTimerRef.current);
      collapseTimerRef.current = null;
    }
    setIsExpanded(false);
    setTimeout(async () => {
      try {
        await ipc.setIslandState("compact");
      } catch (err) {
        console.error("Failed to restore compact island state:", err);
      }
    }, 260);
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

  // Time & Date formatting
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

  const formatCompactDate = (date: Date) => {
    return date.toLocaleDateString(undefined, {
      weekday: "short",
      day: "numeric",
      month: "short",
    });
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
      className="w-full h-full flex items-start justify-center overflow-hidden select-none relative bg-transparent"
    >
      {/* Primary Island Container */}
      <div
        style={{
          width: isExpanded ? "480px" : "200px",
          height: isExpanded ? "220px" : "56px",
          borderRadius: "28px",
        }}
        className={`transition-all duration-250 ease-out shadow-2xl relative overflow-hidden ${
          isExpanded
            ? "glass-panel p-4 cursor-default border border-white/15"
            : "glass-pill px-3 cursor-pointer hover:border-cyan-500/50"
        }`}
      >
        {/* COMPACT PILL VIEW (200x56 px: Two balanced sections, subtle divider, no dot, larger clock font) */}
        <div
          className={`absolute inset-0 flex items-center justify-between px-3 transition-opacity duration-150 ${
            isExpanded ? "opacity-0 pointer-events-none" : "opacity-100"
          }`}
        >
          {/* Left: Current Time (+2-3px larger, AM/PM secondary) */}
          <div className="flex-1 flex items-center justify-center space-x-1 font-mono">
            <span className="font-bold text-[18px] tracking-tight text-white leading-none">
              {timeDisplay.timeStr}
            </span>
            {timeDisplay.ampm && (
              <span className="text-xs font-semibold text-cyan-300/90 leading-none ml-0.5">
                {timeDisplay.ampm}
              </span>
            )}
          </div>

          {/* Subtle Vertical Divider */}
          <div className="h-4 w-px bg-white/15 shrink-0" />

          {/* Right: Date / Day Indicator (until real weather data is integrated) */}
          <div className="flex-1 flex items-center justify-center font-sans">
            <span className="text-xs font-medium text-slate-300 tracking-tight leading-none">
              {formatCompactDate(currentTime)}
            </span>
          </div>
        </div>

        {/* EXPANDED PANEL VIEW (480x220 px: Home clock priority, clean layout, settings & collapse buttons) */}
        <div
          className={`w-full h-full flex flex-col justify-between transition-opacity duration-200 ${
            isExpanded ? "opacity-100 delay-75" : "opacity-0 pointer-events-none"
          }`}
        >
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
                onClick={handleCollapseClick}
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
