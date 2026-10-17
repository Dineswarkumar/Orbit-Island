import React, { useState } from "react";
import { ThemeMeta } from "../../core/theme";
import { HardwareMetrics } from "../../core/types/hardware";

interface HardwareSectionProps {
  activeThemeMeta: ThemeMeta;
  hardwareMetrics: HardwareMetrics | null;
  formatNetworkSpeed: (bps: number) => string;
  onFreeMemory?: () => Promise<{ freedMb: number } | void>;
}

export const HardwareSection: React.FC<HardwareSectionProps> = ({
  activeThemeMeta: _activeThemeMeta,
  hardwareMetrics,
  formatNetworkSpeed,
  onFreeMemory,
}) => {
  const hw = hardwareMetrics;
  const cpuUsage = hw?.cpuUsage ?? 0;
  const ramUsedGb = ((hw?.ramUsedBytes ?? 0) / 1073741824).toFixed(1);
  const ramTotalGb = ((hw?.ramTotalBytes ?? 0) / 1073741824).toFixed(0);
  const ramPercent = hw?.ramPercentage ?? 0;
  const diskPercent = hw?.diskPercentage ?? 0;
  const diskFreeGb = ((hw?.diskFreeBytes ?? 0) / 1073741824).toFixed(0);
  const batteryPct = hw?.batteryPercentage;

  const [isCleaning, setIsCleaning] = useState(false);
  const [freedResult, setFreedResult] = useState<number | null>(null);

  const handleFreeMemory = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isCleaning || !onFreeMemory) return;
    setIsCleaning(true);
    try {
      const res = await onFreeMemory();
      if (res && typeof res.freedMb === "number") {
        setFreedResult(res.freedMb);
      } else {
        setFreedResult(0);
      }
      setTimeout(() => {
        setFreedResult(null);
      }, 2500);
    } catch (err) {
      console.error("Failed to free memory:", err);
    } finally {
      setIsCleaning(false);
    }
  };

  const gpuUsage = hw?.gpuUsage ?? 0;
  const batteryPctVal =
    batteryPct !== null && batteryPct !== undefined ? batteryPct : 100;
  const isCharging = Boolean(hw?.isCharging);

  // Charging color & threshold rules:
  // - While charging: always Green (#22c55e) + charging indicator
  // - On battery > 80%: Green (#22c55e)
  // - On battery 40% - 80%: Amber (#f59e0b)
  // - On battery < 40%: Red (#ef4444)
  const batteryColor = isCharging
    ? "#22c55e"
    : batteryPctVal > 80
    ? "#22c55e"
    : batteryPctVal >= 40
    ? "#f59e0b"
    : "#ef4444";

  const batteryGlow = isCharging
    ? "rgba(34, 197, 94, 0.4)"
    : batteryPctVal > 80
    ? "rgba(34, 197, 94, 0.35)"
    : batteryPctVal >= 40
    ? "rgba(245, 158, 11, 0.35)"
    : "rgba(239, 68, 68, 0.35)";

  const batteryInnerWidth = Math.max(
    1.5,
    Math.min(12.5, (batteryPctVal / 100) * 12.5)
  );

  return (
    <div className="flex flex-col h-full select-none pb-0.5 pl-0.5 space-y-1">
      {/* Compact Header Strip: System GPU label + Compact Battery & Charging Pill */}
      <div className="flex items-center justify-between px-1.5 py-0.5 rounded-lg bg-white/[0.04] border border-white/[0.07] shrink-0">
        <div className="flex items-center space-x-1.5 min-w-0 pr-2">
          <span className="text-[8.5px] font-bold uppercase tracking-wider text-white/45 shrink-0">
            SYS
          </span>
          <span
            className="text-[9.5px] font-semibold text-white/75 truncate"
            title={hw?.gpuName || "Graphics"}
          >
            {hw?.gpuName || "Graphics"}
          </span>
        </div>

        {/* Compact Battery Status Pill (No separate large grid card needed) */}
        <div
          className="flex items-center space-x-1.5 px-2 py-0.5 rounded-full bg-black/35 border shrink-0"
          style={{
            borderColor: isCharging
              ? "rgba(34, 197, 94, 0.45)"
              : "rgba(255, 255, 255, 0.1)",
            boxShadow: isCharging ? `0 0 8px ${batteryGlow}` : undefined,
          }}
          title={
            isCharging
              ? `Charging (${batteryPctVal}%)`
              : `Battery (${batteryPctVal}%)`
          }
        >
          <svg
            className="w-4 h-4 shrink-0"
            style={{
              color: batteryColor,
              filter: `drop-shadow(0 0 3px ${batteryGlow})`,
            }}
            viewBox="0 0 24 24"
            fill="none"
          >
            <rect
              x="2"
              y="7"
              width="17"
              height="10"
              rx="2.5"
              stroke={batteryColor}
              strokeWidth="1.9"
            />
            <path
              d="M20.5 10v4"
              stroke={batteryColor}
              strokeWidth="2"
              strokeLinecap="round"
            />
            <rect
              x="4"
              y="9"
              width={batteryInnerWidth}
              height="6"
              rx="1.2"
              fill={batteryColor}
            />
            {isCharging && (
              <path
                d="M11.5 7.5l-2.2 4h3.2l-1.8 4.5"
                stroke="#ffffff"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
                fill="none"
              />
            )}
          </svg>
          <span
            className="text-[10px] font-bold tabular-nums leading-none"
            style={{ color: batteryColor }}
          >
            {batteryPct !== null && batteryPct !== undefined
              ? `${batteryPct}%`
              : "AC"}
          </span>
          {isCharging && (
            <span className="text-[8.5px] font-bold uppercase tracking-tight text-emerald-400 leading-none">
              ⚡ Charging
            </span>
          )}
        </div>
      </div>

      {/* 2x3 3D Hardware Telemetry Cards (CPU, GPU, RAM, DISK, NET UP, NET DOWN) */}
      <div className="grid grid-cols-2 grid-rows-3 gap-1.5 flex-1 min-h-0">
        {/* Row 1, Col 1: CPU (Sapphire Blue) */}
        <div className="px-2.5 py-1 rounded-xl orbit-card-3d flex items-center space-x-2">
          <svg
            className="w-3.5 h-3.5 shrink-0"
            style={{
              color: "#60a5fa",
              filter: "drop-shadow(0 0 4px rgba(59, 130, 246, 0.35))",
            }}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <rect x="4" y="4" width="16" height="16" rx="2" />
            <rect x="9" y="9" width="6" height="6" />
            <path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3" />
          </svg>
          <div className="flex flex-col flex-1 min-w-0">
            <div className="flex items-baseline justify-between">
              <span className="text-[8.5px] font-bold uppercase tracking-wider text-white/50 leading-none">
                CPU
              </span>
              <span className="text-[11px] font-bold text-white tabular-nums leading-none">
                {cpuUsage.toFixed(1)}%
              </span>
            </div>
            <div className="w-full h-1.5 rounded-full orbit-well-3d mt-1 overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${Math.min(100, Math.max(4, cpuUsage))}%`,
                  backgroundColor: cpuUsage > 85 ? "#ef4444" : "#3b82f6",
                  boxShadow: `0 1px 4px ${
                    cpuUsage > 85
                      ? "rgba(239, 68, 68, 0.4)"
                      : "rgba(59, 130, 246, 0.35)"
                  }`,
                }}
              />
            </div>
          </div>
        </div>

        {/* Row 1, Col 2: GPU USAGE (Rose / Pink) */}
        <div
          className="px-2.5 py-1 rounded-xl orbit-card-3d flex items-center space-x-2"
          title={hw?.gpuName || "GPU Utilization"}
        >
          <svg
            className="w-3.5 h-3.5 shrink-0"
            style={{
              color: "#f472b6",
              filter: "drop-shadow(0 0 4px rgba(236, 72, 153, 0.35))",
            }}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <rect x="2" y="6" width="20" height="12" rx="2" />
            <circle cx="8" cy="12" r="2.5" />
            <circle cx="16" cy="12" r="2.5" />
          </svg>
          <div className="flex flex-col flex-1 min-w-0">
            <div className="flex items-baseline justify-between">
              <span className="text-[8.5px] font-bold uppercase tracking-wider text-white/50 leading-none">
                GPU
              </span>
              <span className="text-[11px] font-bold text-white tabular-nums leading-none">
                {gpuUsage.toFixed(1)}%
              </span>
            </div>
            <div className="w-full h-1.5 rounded-full orbit-well-3d mt-1 overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${Math.min(100, Math.max(4, gpuUsage))}%`,
                  backgroundColor: gpuUsage > 85 ? "#ef4444" : "#ec4899",
                  boxShadow: "0 1px 4px rgba(236, 72, 153, 0.35)",
                }}
              />
            </div>
          </div>
        </div>

        {/* Row 2, Col 1: RAM + Boost Cleaner Button (Linear Violet) */}
        <div className="px-2.5 py-1 rounded-xl orbit-card-3d flex items-center space-x-2">
          <svg
            className="w-3.5 h-3.5 shrink-0"
            style={{
              color: "#a78bfa",
              filter: "drop-shadow(0 0 4px rgba(139, 92, 246, 0.35))",
            }}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <rect x="2" y="6" width="20" height="12" rx="2" />
            <path d="M6 12h.01M10 12h.01M14 12h.01M18 12h.01M6 18v2M10 18v2M14 18v2M18 18v2" />
          </svg>
          <div className="flex flex-col flex-1 min-w-0">
            <div className="flex items-baseline justify-between">
              <div className="flex items-center space-x-1">
                <span className="text-[8.5px] font-bold uppercase tracking-wider text-white/50 leading-none">
                  RAM
                </span>
                {onFreeMemory && (
                  <button
                    type="button"
                    onClick={handleFreeMemory}
                    disabled={isCleaning}
                    title="MSI-Center Quick Memory Cleaner"
                    className="px-1 py-0.5 text-[7.5px] font-bold tracking-tight rounded orbit-card-3d hover:brightness-125 transition-all active:scale-95 leading-none cursor-pointer"
                    style={{
                      color: freedResult !== null ? "#22c55e" : "#c4b5fd",
                      border: "1px solid rgba(167, 139, 250, 0.45)",
                    }}
                  >
                    {isCleaning
                      ? "..."
                      : freedResult !== null
                      ? `-${freedResult.toFixed(0)}M`
                      : "Boost"}
                  </button>
                )}
              </div>
              <span className="text-[10.5px] font-bold text-white tabular-nums leading-none">
                {ramUsedGb}/{ramTotalGb}G
              </span>
            </div>
            <div className="w-full h-1.5 rounded-full orbit-well-3d mt-1 overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${Math.min(100, Math.max(4, ramPercent))}%`,
                  backgroundColor: "#8b5cf6",
                  boxShadow: "0 1px 4px rgba(139, 92, 246, 0.35)",
                }}
              />
            </div>
          </div>
        </div>

        {/* Row 2, Col 2: DISK C: (Warm Amber/Orange) */}
        <div className="px-2.5 py-1 rounded-xl orbit-card-3d flex items-center space-x-2">
          <svg
            className="w-3.5 h-3.5 shrink-0"
            style={{
              color: "#fbbf24",
              filter: "drop-shadow(0 0 4px rgba(245, 158, 11, 0.35))",
            }}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <ellipse cx="12" cy="5" rx="9" ry="3" />
            <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
            <path d="M3 12c0 1.66 4 3 9 3s9-1.34 9-3" />
          </svg>
          <div className="flex flex-col flex-1 min-w-0">
            <div className="flex items-baseline justify-between">
              <span className="text-[8.5px] font-bold uppercase tracking-wider text-white/50 leading-none">
                DISK C:
              </span>
              <span className="text-[10.5px] font-bold text-white tabular-nums leading-none">
                {diskPercent.toFixed(0)}% ({diskFreeGb}G)
              </span>
            </div>
            <div className="w-full h-1.5 rounded-full orbit-well-3d mt-1 overflow-hidden">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${Math.min(100, Math.max(4, diskPercent))}%`,
                  backgroundColor: "#f59e0b",
                  boxShadow: "0 1px 4px rgba(245, 158, 11, 0.35)",
                }}
              />
            </div>
          </div>
        </div>

        {/* Row 3, Col 1: NET UP (Emerald) */}
        <div className="px-2.5 py-1 rounded-xl orbit-card-3d flex items-center space-x-2">
          <svg
            className="w-3.5 h-3.5 shrink-0"
            style={{
              color: "#34d399",
              filter: "drop-shadow(0 0 4px rgba(16, 185, 129, 0.35))",
            }}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.2"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M12 19V5m-6 6l6-6 6 6"
            />
          </svg>
          <div className="flex items-center justify-between flex-1 min-w-0">
            <span className="text-[8.5px] font-bold uppercase tracking-wider text-white/50 leading-none">
              NET UP
            </span>
            <span
              className="text-[10.5px] font-bold tabular-nums truncate leading-none"
              style={{ color: "#34d399" }}
            >
              ↑ {formatNetworkSpeed(hw?.networkUploadSpeedBps ?? 0)}
            </span>
          </div>
        </div>

        {/* Row 3, Col 2: NET DOWN (Sky Cyan) */}
        <div className="px-2.5 py-1 rounded-xl orbit-card-3d flex items-center space-x-2">
          <svg
            className="w-3.5 h-3.5 shrink-0"
            style={{
              color: "#22d3ee",
              filter: "drop-shadow(0 0 4px rgba(6, 182, 212, 0.35))",
            }}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.2"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M12 5v14m-6-6l6 6 6-6"
            />
          </svg>
          <div className="flex items-center justify-between flex-1 min-w-0">
            <span className="text-[8.5px] font-bold uppercase tracking-wider text-white/50 leading-none">
              NET DOWN
            </span>
            <span
              className="text-[10.5px] font-bold tabular-nums truncate leading-none"
              style={{ color: "#22d3ee" }}
            >
              ↓ {formatNetworkSpeed(hw?.networkDownloadSpeedBps ?? 0)}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
