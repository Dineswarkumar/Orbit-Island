import React, { useEffect, useState } from "react";

type CachedWeather = {
  tempC: number;
  tempF: number;
  conditionCode: "clear" | "partly_cloudy" | "cloudy" | "rain" | "snow" | "storm";
  label: string;
};

const readCachedWeather = (): CachedWeather | null => {
  try {
    const parsed = JSON.parse(localStorage.getItem("orbit_weather_cache") || "null");
    const data = parsed?.data;
    if (
      data &&
      typeof data.tempC === "number" &&
      typeof data.tempF === "number" &&
      typeof data.label === "string"
    ) {
      return data as CachedWeather;
    }
  } catch {
    // The normal app will fetch weather once its main view loads.
  }
  return null;
};

export const StartupCapsule: React.FC = () => {
  const [now, setNow] = useState(() => new Date());
  const [weather] = useState(readCachedWeather);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 15_000);
    return () => window.clearInterval(timer);
  }, []);

  const timeParts = new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    minute: "2-digit",
  }).formatToParts(now);
  const hour = timeParts.find((part) => part.type === "hour")?.value ?? "12";
  const minute = timeParts.find((part) => part.type === "minute")?.value ?? "00";
  const period = timeParts.find((part) => part.type === "dayPeriod")?.value;

  return (
    <div className="island-container w-full h-full flex items-start justify-center pt-[3px] overflow-hidden select-none relative bg-transparent">
      <div
        className="relative flex items-center justify-between px-3.5 border overflow-hidden"
        aria-label="ORBIT is starting"
        style={{
          width: "132px",
          height: "40px",
          borderRadius: "20px",
          background: "linear-gradient(180deg, rgba(17, 20, 29, 0.99), rgba(5, 7, 12, 0.99))",
          borderColor: "rgba(255, 255, 255, 0.2)",
          boxShadow: "0 5px 18px rgba(0,0,0,0.42), inset 0 1px 1px rgba(255,255,255,0.19)",
          color: "#f8fafc",
        }}
      >
        <div className="flex items-baseline gap-1.5 min-w-0">
          <span className="text-[13px] font-bold tabular-nums tracking-tight">
            {hour}:{minute}
          </span>
          {period && <span className="text-[8px] font-bold text-white/65">{period}</span>}
        </div>
        <div
          className="flex items-center gap-1 border-l border-white/15 pl-2 text-[10px] font-bold text-white/75"
          title={weather?.label ?? "Weather offline or unavailable"}
        >
          <svg className="w-3.5 h-3.5 shrink-0" viewBox="0 0 24 24" aria-hidden="true" fill="none">
            {weather ? (
              weather.conditionCode === "rain" || weather.conditionCode === "storm" || weather.conditionCode === "snow" || weather.conditionCode === "cloudy" ? (
                <path d="M19.4 10.1A7.5 7.5 0 0 0 5.1 8.4 5.5 5.5 0 0 0 6 19h12a4.5 4.5 0 0 0 1.4-8.9Z" fill="#e2e8f0" />
              ) : (
                <>
                  <circle cx="12" cy="12" r="4.2" fill="#fde047" />
                  <path d="M12 2v2m0 16v2M2 12h2m16 0h2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4m0-14.2-1.4 1.4M6.3 17.7l-1.4 1.4" stroke="#fbbf24" strokeWidth="1.8" strokeLinecap="round" />
                </>
              )
            ) : (
              <g opacity="0.6">
                <path
                  d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96z"
                  fill="currentColor"
                />
                <line
                  x1="2"
                  y1="2"
                  x2="22"
                  y2="22"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              </g>
            )}
          </svg>
          <span>{weather ? `${weather.tempC}°` : "--°"}</span>
        </div>
      </div>
    </div>
  );
};
