import React, { useEffect, useState, useRef } from "react";
import { ChevronDown } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { ipc } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";
import { AppConfig } from "../core/types/config";
import orbitLogo from "../assets/orbit-logo.png";

const THEME_OPTIONS: Array<{ id: AppConfig["theme"]; label: string; color: string; borderGlow: string }> = [
  { id: "obsidian", label: "Obsidian Dark", color: "#22d3ee", borderGlow: "rgba(34, 211, 238, 0.25)" },
  { id: "dark", label: "Dark", color: "#22d3ee", borderGlow: "rgba(34, 211, 238, 0.25)" },
  { id: "dynamic", label: "Dynamic", color: "#38bdf8", borderGlow: "rgba(56, 189, 248, 0.25)" },
  { id: "light", label: "Porcelain Light", color: "#0ea5e9", borderGlow: "rgba(14, 165, 233, 0.25)" },
  { id: "black", label: "Black", color: "#38bdf8", borderGlow: "rgba(56, 189, 248, 0.25)" },
  { id: "espresso", label: "Espresso Warm", color: "#f59e0b", borderGlow: "rgba(245, 158, 11, 0.25)" },
  { id: "evergreen", label: "Evergreen", color: "#10b981", borderGlow: "rgba(16, 185, 129, 0.25)" },
  { id: "nord", label: "Nord Frost", color: "#818cf8", borderGlow: "rgba(129, 140, 248, 0.25)" },
];

export const IslandApp: React.FC = () => {
  const { config, loadConfig } = useConfigStore();

  const [isExpanded, setIsExpanded] = useState(false);
  const [currentTime, setCurrentTime] = useState(new Date());

  // Transition generation ID and explicitly tracked timer refs to completely eliminate race conditions
  const transitionIdRef = useRef<number>(0);
  const collapseDebounceTimerRef = useRef<NodeJS.Timeout | null>(null);
  const nativeResizeTimerRef = useRef<NodeJS.Timeout | null>(null);

  useEffect(() => {
    loadConfig();

    // Listen for live configuration broadcasts from the Settings window or native menu
    const unlistenConfig = listen<AppConfig>("config-changed", (event) => {
      useConfigStore.setState({ config: event.payload });
    });

    // Listen for collapse requests from native context menu Dismiss action
    const unlistenCollapse = listen("collapse-island", () => {
      handleTriggerCollapse();
    });

    return () => {
      unlistenConfig.then((unlisten) => unlisten());
      unlistenCollapse.then((unlisten) => unlisten());
    };
  }, [loadConfig]);

  // Live system clock ticker (1-second interval)
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  const handleTriggerCollapse = () => {
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

  // Hover expansion triggers
  const handleMouseEnter = async () => {
    transitionIdRef.current += 1;
    if (collapseDebounceTimerRef.current) {
      clearTimeout(collapseDebounceTimerRef.current);
      collapseDebounceTimerRef.current = null;
    }
    if (nativeResizeTimerRef.current) {
      clearTimeout(nativeResizeTimerRef.current);
      nativeResizeTimerRef.current = null;
    }

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
    if (isExpanded) {
      const currentTransitionId = ++transitionIdRef.current;

      if (collapseDebounceTimerRef.current) {
        clearTimeout(collapseDebounceTimerRef.current);
      }
      if (nativeResizeTimerRef.current) {
        clearTimeout(nativeResizeTimerRef.current);
      }

      collapseDebounceTimerRef.current = setTimeout(() => {
        if (transitionIdRef.current !== currentTransitionId) return;
        handleTriggerCollapse();
      }, 300);
    }
  };

  const handleCollapseClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    handleTriggerCollapse();
  };

  // Right-click: trigger native Windows context menu (positioned beside pointer, unclipped)
  const handleContextMenu = async (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    try {
      await ipc.showContextMenu();
    } catch (err) {
      console.error("Failed to show native context menu:", err);
    }
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
        {/* COMPACT PILL VIEW (200x56 px: Rendered ONLY when not expanded - eliminates clock doubling) */}
        {!isExpanded && (
          <div className="absolute inset-0 flex items-center justify-center space-x-1.5 transition-opacity duration-150">
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
        )}

        {/* EXPANDED PANEL VIEW (400x184 px: Rendered ONLY when expanded - matching reference-idle-clock.png) */}
        {isExpanded && (
          <div className="w-[400px] h-[184px] p-3 flex flex-col justify-between absolute inset-0 transition-opacity duration-150">
            {/* Header: Preserved ORBIT Logo & Discreet Collapse Button */}
            <div className="flex items-center justify-between pb-1.5 border-b border-white/10">
              <img
                src={orbitLogo}
                alt="ORBIT"
                style={{ height: "18px", maxHeight: "18px", maxWidth: "80px", width: "auto" }}
                className="h-[18px] max-h-[18px] max-w-[80px] w-auto object-contain opacity-90 select-none pointer-events-none"
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
        )}
      </div>
    </div>
  );
};
