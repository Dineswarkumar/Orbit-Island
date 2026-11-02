import React, { useState, useEffect } from "react";
import { useConfigStore } from "../../core/stores/useConfigStore";

export interface ClockDisplayProps {
  className?: string;
  timeClassName?: string;
  ampmClassName?: string;
  timeStyle?: React.CSSProperties;
  ampmStyle?: React.CSSProperties;
  onClick?: (e: React.MouseEvent) => void;
  title?: string;
}

export const ClockDisplay: React.FC<ClockDisplayProps> = React.memo(({
  className = "",
  timeClassName = "",
  ampmClassName = "",
  timeStyle,
  ampmStyle,
  onClick,
  title,
}) => {
  const { config } = useConfigStore();
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout>;
    let intervalId: ReturnType<typeof setInterval>;

    const updateTime = () => setNow(new Date());

    const scheduleNext = () => {
      const delay = 60000 - (Date.now() % 60000);
      timeoutId = setTimeout(() => {
        updateTime();
        intervalId = setInterval(updateTime, 60000);
      }, delay);
    };

    scheduleNext();

    return () => {
      clearTimeout(timeoutId);
      clearInterval(intervalId);
    };
  }, []);

  const is12h = config.timeFormat === "12h";
  let hours = now.getHours();
  const minutes = String(now.getMinutes()).padStart(2, "0");
  let ampm = "";

  if (is12h) {
    ampm = hours >= 12 ? "PM" : "AM";
    hours = hours % 12 || 12;
  }

  const timeStr = is12h ? `${hours}:${minutes}` : `${String(hours).padStart(2, "0")}:${minutes}`;

  return (
    <div className={className} onClick={onClick} title={title}>
      <span className={timeClassName} style={timeStyle}>
        {timeStr}
      </span>
      {ampm ? (
        <span className={ampmClassName} style={ampmStyle}>
          {ampm}
        </span>
      ) : null}
    </div>
  );
});

ClockDisplay.displayName = "ClockDisplay";
