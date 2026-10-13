import React, { useEffect, useState, useRef } from "react";
import { ChevronDown, Palette, Check, Settings } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { ipc } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";
import { AppConfig } from "../core/types/config";
import orbitLogo from "../assets/orbit-logo.png";

const THEME_OPTIONS: Array<{ id: AppConfig["theme"]; label: string; color: string; borderGlow: string }> = [
  { id: "obsidian", label: "Obsidian Dark", color: "#22d3ee", borderGlow: "rgba(34, 211, 238, 0.25)" },
  { id: "espresso", label: "Espresso Warm", color: "#f59e0b", borderGlow: "rgba(245, 158, 11, 0.25)" },
  { id: "evergreen", label: "Evergreen", color: "#10b981", borderGlow: "rgba(16, 185, 129, 0.25)" },
  { id: "nord", label: "Nord Frost", color: "#818cf8", borderGlow: "rgba(129, 140, 248, 0.25)" },
  { id: "light", label: "Porcelain Light", color: "#0ea5e9", borderGlow: "rgba(14, 165, 233, 0.25)" },
];

export const IslandApp: React.FC = () => {
  const { config, loadConfig, updateConfig } = useConfigStore();

  const [isExpanded, setIsExpanded] = useState(false);
  const [currentTime, setCurrentTime] = useState(new Date());
  const [contextMenuPos, setContextMenuPos] = useState<{ x: number; y: number } | null>(null);

  // Transition generation ID and explicitly tracked timer refs to completely eliminate race conditions
  const transitionIdRef = useRef<number>(0);
  const collapseDebounceTimerRef = useRef<NodeJS.Timeout | null>(null);
  const nativeResizeTimerRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    loadConfig();

    // Listen for live configuration broadcasts from the Settings window
    const unlistenPromise = listen<AppConfig>("config-changed", (event) => {
      useConfigStore.setState({ config: event.payload });
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
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
    // 1. Invalidate any pending collapse callbacks
    transitionIdRef.current += 1;
    if (collapseDebounceTimerRef.current) {
      clearTimeout(collapseDebounceTimerRef.current);
      collapseDebounceTimerRef.current = null;
    }
    if (nativeResizeTimerRef.current) {
      clearTimeout(nativeResizeTimerRef.current);
      nativeResizeTimerRef.current = null;
    }

    // 2. Expand native window and region first so webview has room to render
    if (!isExpanded) {
      try {
        await ipc.setIslandState("expanded");
      } catch (err) {
        console.error("Failed to set island state to expanded:", err);
      }
      setIsExpanded(true);
    }
  };

  const handleMouseLeave = () => {
    // Only auto-collapse if context menu is not open and currently expanded
    if (!contextMenuPos && isExpanded) {
      const currentTransitionId = ++transitionIdRef.current;

      if (collapseDebounceTimerRef.current) {
        clearTimeout(collapseDebounceTimerRef.current);
      }
      if (nativeResizeTimerRef.current) {
        clearTimeout(nativeResizeTimerRef.current);
      }

      collapseDebounceTimerRef.current = setTimeout(() => {
        // Guard against race conditions if pointer re-entered
        if (transitionIdRef.current !== currentTransitionId) return;

        // Animate visual shrink back to compact pill in webview
        setIsExpanded(false);

        // Once the 250ms CSS shrink animation completes, resize native window back to 200x56
        nativeResizeTimerRef.current = setTimeout(async () => {
          if (transitionIdRef.current !== currentTransitionId) return;
          try {
            await ipc.setIslandState("compact");
          } catch (err) {
            console.error("Failed to restore compact island state:", err);
          }
        }, 260);
      }, 300);
    }
  };

  const handleCollapseClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setContextMenuPos(null);
    const currentTransitionId = ++transitionIdRef.current;

    if (collapseDebounceTimerRef.current) {
      clearTimeout(collapseDebounceTimerRef.current);
      collapseDebounceTimerRef.current = null;
    }
    if (nativeResizeTimerRef.current) {
      clearTimeout(nativeResizeTimerRef.current);
      nativeResizeTimerRef.current = null;
    }

    setIsExpanded(false);
    nativeResizeTimerRef.current = setTimeout(async () => {
      if (transitionIdRef.current !== currentTransitionId) return;
      try {
        await ipc.setIslandState("compact");
      } catch (err) {
        console.error("Failed to restore compact island state:", err);
      }
    }, 260);
  };

  // Right-click context menu (Only active when expanded)
  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (isExpanded) {
      setContextMenuPos({ x: Math.min(e.clientX, 240), y: Math.min(e.clientY, 50) });
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
      weekday: "long",
      month: "long",
      day: "numeric",
    });
  };

  const timeDisplay = formatTime(currentTime);
  const activeThemeMeta = THEME_OPTIONS.find((t) => t.id === config.theme) || THEME_OPTIONS[0];

  return (
    <div
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onContextMenu={handleContextMenu}
      onClick={() => setContextMenuPos(null)}
      className={`w-full h-full flex items-start justify-center overflow-hidden select-none relative bg-transparent font-sans theme-${config.theme}`}
    >
      {/* Primary Island Container */}
      <div
        style={{
          width: isExpanded ? "400px" : "200px",
          height: isExpanded ? "184px" : "56px",
          borderRadius: "28px",
          borderColor: isExpanded ? activeThemeMeta.borderGlow : undefined,
        }}
        className={`transition-all duration-250 ease-out shadow-2xl relative overflow-hidden ${
          isExpanded
            ? "glass-panel p-3 cursor-default border border-white/15"
            : "glass-pill px-3 cursor-pointer hover:border-cyan-500/50"
        }`}
      >
        {/* COMPACT PILL VIEW (200x56 px: Time and AM/PM only, prominent font, secondary AM/PM) */}
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
            <span
              className="text-xs font-semibold leading-none"
              style={{ color: activeThemeMeta.color }}
            >
              {timeDisplay.ampm}
            </span>
          )}
        </div>

        {/* EXPANDED PANEL VIEW (400x184 px: Matching reference-idle-clock.png, logo header, section dots) */}
        <div
          className={`w-[400px] h-[184px] p-3.5 flex flex-col justify-between absolute inset-0 transition-opacity ${
            isExpanded
              ? "opacity-100 duration-200 delay-75"
              : "opacity-0 pointer-events-none duration-100"
          }`}
        >
          {/* Header: Preserved ORBIT Logo & Discreet Collapse Button */}
          <div className="flex items-center justify-between pb-1.5 border-b border-white/10">
            <img
              src={orbitLogo}
              alt="ORBIT"
              className="h-4.5 w-auto object-contain opacity-90 select-none pointer-events-none"
            />
            <button
              onClick={handleCollapseClick}
              title="Collapse to Pill"
              className="p-1 rounded-lg hover:bg-white/10 text-slate-400 hover:text-white transition-colors cursor-pointer"
            >
              <ChevronDown className="w-3.5 h-3.5" />
            </button>
          </div>

          {/* Main Display: Centered Large Time & Date (matching reference-idle-clock.png) */}
          <div className="flex-1 flex items-center justify-between px-2">
            {/* Left Spacer to balance section dots */}
            <div className="w-3" />

            {/* Center: Large Time & Date */}
            <div className="flex flex-col items-center justify-center">
              <div className="text-[34px] font-bold tracking-tight text-white font-mono flex items-baseline space-x-2 leading-none">
                <span>{timeDisplay.timeStr}</span>
                {timeDisplay.ampm && (
                  <span
                    className="text-base font-bold leading-none"
                    style={{ color: activeThemeMeta.color }}
                  >
                    {timeDisplay.ampm}
                  </span>
                )}
              </div>
              <div className="text-xs text-slate-300/80 font-medium tracking-normal mt-1.5">
                {formatDate(currentTime)}
              </div>
            </div>

            {/* Right: Section Navigation Indicator Dots (Home, Calendar, Productivity, Hardware) */}
            <div className="flex flex-col space-y-1.5 items-center justify-center pl-2">
              <div
                className="w-1.5 h-1.5 rounded-full shadow-sm"
                style={{ backgroundColor: activeThemeMeta.color }}
              />
              <div className="w-1 h-1 rounded-full bg-white/20" />
              <div className="w-1 h-1 rounded-full bg-white/20" />
              <div className="w-1 h-1 rounded-full bg-white/20" />
            </div>
          </div>

          {/* Subtle Bottom Breathing Area */}
          <div className="h-0.5" />
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
                {config.theme === t.id && (
                  <Check className="w-3.5 h-3.5" style={{ color: t.color }} />
                )}
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
