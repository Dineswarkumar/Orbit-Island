import React, { useEffect, useState, useRef } from "react";
import { ChevronDown, ChevronRight, Check, Settings } from "lucide-react";
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
  const [showThemeSubmenu, setShowThemeSubmenu] = useState(false);

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
    setShowThemeSubmenu(false);
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
      // Menu width is 170px, height is ~90px. Native window is 400x184.
      // Clamp x so menu stays within [12, 400 - 180]
      // Clamp y so menu stays within [8, 184 - 95]
      const clampedX = Math.max(12, Math.min(e.clientX, 220));
      const clampedY = Math.max(8, Math.min(e.clientY, 85));
      setContextMenuPos({ x: clampedX, y: clampedY });
      setShowThemeSubmenu(false);
    }
  };

  const handleOpenSettings = async (e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    setContextMenuPos(null);
    setShowThemeSubmenu(false);
    try {
      await ipc.openSettingsWindow();
    } catch (err) {
      console.error("Failed to open settings:", err);
    }
  };

  const handleSelectTheme = async (theme: AppConfig["theme"]) => {
    setContextMenuPos(null);
    setShowThemeSubmenu(false);
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
      onClick={() => {
        setContextMenuPos(null);
        setShowThemeSubmenu(false);
      }}
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
          className={`w-[400px] h-[184px] p-3 flex flex-col justify-between absolute inset-0 transition-opacity ${
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
              style={{ height: "18px", maxHeight: "18px", maxWidth: "90px" }}
              className="h-[18px] max-h-[18px] max-w-[90px] w-auto object-contain opacity-90 select-none pointer-events-none"
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

      {/* Custom Context Menu on Right Click (Hierarchical: Theme submenu + Settings action) */}
      {isExpanded && contextMenuPos && (
        <div
          onClick={(e) => e.stopPropagation()}
          style={{ top: contextMenuPos.y, left: contextMenuPos.x }}
          className="absolute z-50 w-44 bg-slate-900/98 backdrop-blur-xl border border-white/15 rounded-xl shadow-2xl py-1 text-xs text-slate-200 font-sans select-none"
        >
          {/* Item 1: Theme submenu trigger */}
          <div
            onMouseEnter={() => setShowThemeSubmenu(true)}
            onClick={() => setShowThemeSubmenu(!showThemeSubmenu)}
            className="relative px-3 py-1.5 flex items-center justify-between hover:bg-white/10 transition-colors cursor-pointer"
          >
            <div className="flex items-center space-x-2">
              <span
                className="w-2 h-2 rounded-full border border-white/20"
                style={{ backgroundColor: activeThemeMeta.color }}
              />
              <span className="font-medium text-slate-200">Theme</span>
            </div>
            <ChevronRight className="w-3.5 h-3.5 text-slate-400" />

            {/* Theme Submenu Flyout (opens to right or left depending on x position) */}
            {showThemeSubmenu && (
              <div
                style={{
                  top: -4,
                  left: contextMenuPos.x > 180 ? "-158px" : "100%",
                }}
                className="absolute z-50 w-38 bg-slate-900/98 backdrop-blur-xl border border-white/15 rounded-xl shadow-2xl py-1 text-xs text-slate-200"
              >
                <div className="px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400 border-b border-white/10">
                  Select Theme
                </div>
                {THEME_OPTIONS.map((t) => (
                  <button
                    key={t.id}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleSelectTheme(t.id);
                    }}
                    className="w-full px-2.5 py-1.5 flex items-center justify-between hover:bg-white/10 text-left transition-colors cursor-pointer"
                  >
                    <div className="flex items-center space-x-2">
                      <span
                        className="w-2 h-2 rounded-full border border-white/20"
                        style={{ backgroundColor: t.color }}
                      />
                      <span className="truncate">{t.label}</span>
                    </div>
                    {config.theme === t.id && (
                      <Check className="w-3.5 h-3.5" style={{ color: t.color }} />
                    )}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Item 2: ORBIT Settings (Prominently visible and clickable) */}
          <button
            onClick={handleOpenSettings}
            className="w-full px-3 py-1.5 flex items-center space-x-2 hover:bg-white/10 text-left transition-colors cursor-pointer border-t border-white/10"
          >
            <Settings className="w-3.5 h-3.5 text-cyan-400" />
            <span className="font-medium text-slate-200">ORBIT Settings...</span>
          </button>

          {/* Item 3: Collapse to Pill */}
          <button
            onClick={handleCollapseClick}
            className="w-full px-3 py-1.5 flex items-center space-x-2 hover:bg-white/10 text-left transition-colors cursor-pointer border-t border-white/10"
          >
            <ChevronDown className="w-3.5 h-3.5 text-slate-400" />
            <span className="text-slate-300">Collapse</span>
          </button>
        </div>
      )}
    </div>
  );
};
