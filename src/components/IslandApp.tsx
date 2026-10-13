import React, { useEffect, useState, useRef } from "react";
import { ChevronDown, Clock, Palette, Check, Settings } from "lucide-react";
import { ipc } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";
import { AppConfig } from "../core/types/config";
import orbitLogo from "../assets/orbit-logo.png";

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
        // Expand native window and rounded region first so webview has room to render
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
        // 1. Trigger visual shrink back to compact pill in webview
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
      setContextMenuPos({ x: Math.min(e.clientX, 320), y: Math.min(e.clientY, 80) });
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

  // Time & Date formatting (seconds removed per specification)
  const formatTime = (date: Date) => {
    const is12h = config.timeFormat === "12h";
    let hours = date.getHours();
    const minutes = String(date.getMinutes()).padStart(2, "0");

    if (is12h) {
      const ampm = hours >= 12 ? "PM" : "AM";
      hours = hours % 12 || 12;
      return { timeStr: `${hours}:${minutes}`, ampm };
    }
    return { timeStr: `${String(hours).padStart(2, "0")}:${minutes}`, ampm: "" };
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
      className="w-full h-full flex items-start justify-center overflow-hidden select-none relative bg-transparent font-sans"
    >
      {/* Primary Island Container */}
      <div
        style={{
          width: isExpanded ? "500px" : "200px",
          height: isExpanded ? "230px" : "56px",
          borderRadius: "28px",
        }}
        className={`transition-all duration-250 ease-out shadow-2xl relative overflow-hidden ${
          isExpanded
            ? "glass-panel p-4 cursor-default border border-white/15"
            : "glass-pill px-3 cursor-pointer hover:border-cyan-500/50"
        }`}
      >
        {/* COMPACT PILL VIEW (200x56 px: Time and AM/PM only, centered, prominent font, secondary AM/PM) */}
        <div
          className={`absolute inset-0 flex items-center justify-center space-x-1.5 transition-opacity ${
            isExpanded
              ? "opacity-0 pointer-events-none duration-75"
              : "opacity-100 duration-150 delay-150"
          }`}
        >
          <span className="font-semibold text-[20px] tracking-tight text-white leading-none font-mono">
            {timeDisplay.timeStr}
          </span>
          {timeDisplay.ampm && (
            <span className="text-xs font-semibold text-cyan-300/80 leading-none">
              {timeDisplay.ampm}
            </span>
          )}
        </div>

        {/* EXPANDED PANEL VIEW (500x230 px: Relaxed breathing room, ~20% scaled-down interior, no seconds, logo header) */}
        <div
          className={`w-[500px] h-[230px] p-4 flex flex-col justify-between absolute inset-0 transition-opacity ${
            isExpanded
              ? "opacity-100 duration-200 delay-75"
              : "opacity-0 pointer-events-none duration-100"
          }`}
        >
          {/* Header: Preserved ORBIT Logo & Discreet Collapse Button */}
          <div className="flex items-center justify-between pb-2 border-b border-white/10">
            <img
              src={orbitLogo}
              alt="ORBIT"
              className="h-5 w-auto object-contain opacity-90 select-none pointer-events-none"
            />
            <button
              onClick={handleCollapseClick}
              title="Collapse to Pill"
              className="p-1 rounded-lg hover:bg-white/10 text-slate-400 hover:text-white transition-colors cursor-pointer"
            >
              <ChevronDown className="w-4 h-4" />
            </button>
          </div>

          {/* Main Center Display: Clean Home Clock (~20% smaller font, no seconds, date display) */}
          <div className="flex-1 my-2.5 bg-slate-900/50 rounded-2xl p-4 border border-white/5 flex flex-col items-center justify-center">
            <div className="flex items-center space-x-1.5 text-slate-400 text-xs mb-1">
              <Clock className="w-3.5 h-3.5 text-cyan-400" />
              <span className="font-medium text-slate-300">{formatDate(currentTime)}</span>
            </div>

            <div className="text-[30px] font-bold tracking-tight text-white font-mono flex items-baseline space-x-1.5 leading-none mt-1">
              <span>{timeDisplay.timeStr}</span>
              {timeDisplay.ampm && (
                <span className="text-sm font-semibold text-slate-300 ml-1">
                  {timeDisplay.ampm}
                </span>
              )}
            </div>
          </div>

          {/* Spacer / Bottom Breathing Area (Instructional text removed per design mandate) */}
          <div className="h-1" />
        </div>
      </div>

      {/* Custom Context Menu on Right Click (Themes & Settings) */}
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
