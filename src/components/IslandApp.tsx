import React, { useEffect, useState } from "react";
import { Settings, ChevronDown, Play, Pause, RotateCcw, Clock, Timer, Sparkles } from "lucide-react";
import { ipc } from "../core/ipc/client";
import { useConfigStore } from "../core/stores/useConfigStore";
import { useTimerStore } from "../core/stores/useTimerStore";

export const IslandApp: React.FC = () => {
  const { config, loadConfig } = useConfigStore();
  const { remainingSeconds, isRunning, startTimer, pauseTimer, resetTimer, tick } = useTimerStore();

  const [isExpanded, setIsExpanded] = useState(false);
  const [currentTime, setCurrentTime] = useState(new Date());

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // Clock ticker (1 second interval)
  useEffect(() => {
    const timer = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // Timer ticker (1 second interval)
  useEffect(() => {
    const timer = setInterval(() => {
      tick();
    }, 1000);
    return () => clearInterval(timer);
  }, [tick]);

  const toggleExpand = async () => {
    const next = !isExpanded;
    setIsExpanded(next);
    try {
      await ipc.setIslandState(next ? "expanded" : "compact");
    } catch (err) {
      console.error("Failed to set island state:", err);
    }
  };

  const handleOpenSettings = async (e: React.MouseEvent) => {
    e.stopPropagation();
    try {
      await ipc.openSettingsWindow();
    } catch (err) {
      console.error("Failed to open settings:", err);
    }
  };

  // Clock formatting
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

  // Timer formatting
  const formatTimer = (totalSeconds: number) => {
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  };

  const timeDisplay = formatTime(currentTime);

  return (
    <div className="w-screen h-screen flex items-start justify-center overflow-hidden select-none p-1">
      {/* Primary Island Container */}
      <div
        onClick={!isExpanded ? toggleExpand : undefined}
        className={`w-full h-full flex flex-col justify-between text-white transition-all duration-200 ease-out shadow-2xl ${
          isExpanded
            ? "glass-panel rounded-[28px] p-4 cursor-default border border-white/15"
            : "glass-pill rounded-full px-4 py-2 cursor-pointer hover:border-cyan-500/50"
        }`}
      >
        {/* COMPACT PILL VIEW */}
        {!isExpanded && (
          <div className="w-full h-full flex items-center justify-between">
            {/* Left slot: Brand & Status / Active priority indicator */}
            <div className="flex items-center space-x-2.5">
              <div className="w-2.5 h-2.5 rounded-full bg-gradient-to-r from-cyan-400 to-indigo-500 shadow-sm shadow-cyan-400/50 animate-pulse" />
              <span className="font-semibold tracking-wider text-xs uppercase text-slate-200">
                ORBIT
              </span>
            </div>

            {/* Center slot: Clock or Active Timer */}
            <div className="flex items-center space-x-1.5 font-mono text-xs text-slate-300">
              {isRunning ? (
                <div className="flex items-center space-x-1 text-cyan-300 font-semibold">
                  <Timer className="w-3.5 h-3.5 text-cyan-400 animate-spin-slow" />
                  <span>{formatTimer(remainingSeconds)}</span>
                </div>
              ) : (
                <div className="flex items-baseline space-x-1">
                  <span className="font-bold text-slate-100">{timeDisplay.timeStr}</span>
                  {timeDisplay.ampm && (
                    <span className="text-[10px] text-slate-400">{timeDisplay.ampm}</span>
                  )}
                </div>
              )}
            </div>

            {/* Right slot: Controls trigger */}
            <div className="flex items-center space-x-1">
              <button
                onClick={handleOpenSettings}
                title="Open Settings"
                className="p-1 rounded-full hover:bg-white/10 text-slate-400 hover:text-white transition-colors cursor-pointer"
              >
                <Settings className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        )}

        {/* EXPANDED PANEL VIEW */}
        {isExpanded && (
          <div className="w-full h-full flex flex-col justify-between">
            {/* Top Bar: Title, Collapse button, Settings */}
            <div className="flex items-center justify-between pb-2.5 border-b border-white/10">
              <div className="flex items-center space-x-2">
                <div className="w-2.5 h-2.5 rounded-full bg-cyan-400 shadow-sm shadow-cyan-400/60" />
                <span className="text-xs font-bold uppercase tracking-wider text-slate-200">
                  Home Panel
                </span>
                <span className="text-[10px] px-1.5 py-0.5 rounded bg-cyan-950/80 text-cyan-300 border border-cyan-800/40 font-mono">
                  Core Shell
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
                  onClick={toggleExpand}
                  title="Collapse to Pill"
                  className="p-1.5 rounded-lg hover:bg-white/10 text-slate-400 hover:text-white transition-colors cursor-pointer"
                >
                  <ChevronDown className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* Middle Grid: Live Clock (Left) & Persistent Timer (Right) */}
            <div className="grid grid-cols-2 gap-3 py-2 flex-1 items-stretch">
              {/* Home Clock Card */}
              <div className="bg-slate-900/60 rounded-2xl p-3 border border-white/5 flex flex-col justify-between">
                <div className="flex items-center justify-between text-slate-400 text-xs">
                  <div className="flex items-center space-x-1.5">
                    <Clock className="w-3.5 h-3.5 text-cyan-400" />
                    <span>System Time</span>
                  </div>
                  <span className="text-[10px]">{formatDate(currentTime)}</span>
                </div>

                <div className="flex flex-col items-center justify-center my-auto">
                  <div className="text-2xl font-extrabold tracking-tight text-white font-mono flex items-baseline space-x-1">
                    <span>{timeDisplay.timeStr}</span>
                    <span className="text-xs text-cyan-400 font-normal">:{timeDisplay.seconds}</span>
                    {timeDisplay.ampm && (
                      <span className="text-xs text-slate-400 font-normal ml-1">
                        {timeDisplay.ampm}
                      </span>
                    )}
                  </div>
                </div>

                <div className="text-[10px] text-slate-500 text-center">
                  Replaced when activity is active
                </div>
              </div>

              {/* Countdown Timer Card */}
              <div className="bg-slate-900/60 rounded-2xl p-3 border border-white/5 flex flex-col justify-between">
                <div className="flex items-center justify-between text-slate-400 text-xs">
                  <div className="flex items-center space-x-1.5">
                    <Timer className="w-3.5 h-3.5 text-indigo-400" />
                    <span>Countdown Timer</span>
                  </div>
                  {isRunning && (
                    <span className="text-[10px] text-emerald-400 font-semibold animate-pulse">
                      Active
                    </span>
                  )}
                </div>

                <div className="flex flex-col items-center justify-center my-auto">
                  <div
                    className={`text-2xl font-extrabold font-mono tracking-tight ${
                      remainingSeconds === 0 ? "text-rose-400 animate-bounce" : "text-slate-100"
                    }`}
                  >
                    {formatTimer(remainingSeconds)}
                  </div>
                </div>

                {/* Timer Controls & Presets */}
                <div className="flex items-center justify-between pt-1">
                  <div className="flex items-center space-x-1">
                    <button
                      onClick={() => resetTimer(300)}
                      className="px-1.5 py-0.5 rounded text-[10px] bg-slate-800 hover:bg-slate-700 text-slate-300 font-mono transition-colors"
                    >
                      5m
                    </button>
                    <button
                      onClick={() => resetTimer(900)}
                      className="px-1.5 py-0.5 rounded text-[10px] bg-slate-800 hover:bg-slate-700 text-slate-300 font-mono transition-colors"
                    >
                      15m
                    </button>
                    <button
                      onClick={() => resetTimer(1500)}
                      className="px-1.5 py-0.5 rounded text-[10px] bg-slate-800 hover:bg-slate-700 text-slate-300 font-mono transition-colors"
                    >
                      25m
                    </button>
                  </div>

                  <div className="flex items-center space-x-1">
                    {isRunning ? (
                      <button
                        onClick={pauseTimer}
                        title="Pause"
                        className="p-1 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 text-amber-300 transition-colors"
                      >
                        <Pause className="w-3.5 h-3.5" />
                      </button>
                    ) : (
                      <button
                        onClick={() => startTimer()}
                        title="Start"
                        className="p-1 rounded-lg bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-300 transition-colors"
                      >
                        <Play className="w-3.5 h-3.5" />
                      </button>
                    )}
                    <button
                      onClick={() => resetTimer()}
                      title="Reset"
                      className="p-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-white transition-colors"
                    >
                      <RotateCcw className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            </div>

            {/* Bottom Panel Status Banner */}
            <div className="flex items-center justify-between pt-1 text-[11px] text-slate-400">
              <span className="flex items-center space-x-1">
                <Sparkles className="w-3 h-3 text-cyan-400" />
                <span>ORBIT Core Shell Active</span>
              </span>
              <span className="text-slate-500 font-mono text-[10px]">
                Click chevron to collapse
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
