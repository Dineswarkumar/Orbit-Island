import React, { useState } from "react";
import { ThemeMeta } from "../../core/theme";
import { ReminderItem } from "../../core/types/reminders";

interface CalendarSectionProps {
  activeThemeMeta: ThemeMeta;
  calendarMonth: Date;
  setCalendarMonth: (d: Date) => void;
  selectedCalendarDate: Date;
  setSelectedCalendarDate: (d: Date) => void;
  reminders: ReminderItem[];
  newReminderTitle: string;
  setNewReminderTitle: (val: string) => void;
  onAddReminder: (e?: React.FormEvent) => void;
  onToggleReminder: (id: string) => void;
  onDeleteReminder: (id: string) => void;
}

export const CalendarSection: React.FC<CalendarSectionProps> = ({
  activeThemeMeta,
  calendarMonth,
  setCalendarMonth,
  selectedCalendarDate,
  setSelectedCalendarDate,
  reminders,
  newReminderTitle,
  setNewReminderTitle,
  onAddReminder,
  onToggleReminder,
  onDeleteReminder,
}) => {
  const [showTaskDrawer, setShowTaskDrawer] = useState(false);

  const curYear = calendarMonth.getFullYear();
  const curMonth = calendarMonth.getMonth();
  const daysInMonth = new Date(curYear, curMonth + 1, 0).getDate();
  const firstDayIndex = new Date(curYear, curMonth, 1).getDay();
  const monthName = selectedCalendarDate.toLocaleString("default", { month: "long" });
  const selectedYear = selectedCalendarDate.getFullYear();
  const selectedDayNum = selectedCalendarDate.getDate();
  const selectedWeekday = selectedCalendarDate.toLocaleString("default", {
    weekday: "long",
  });

  const todayStr = new Date().toDateString();
  const selectedStr = selectedCalendarDate.toDateString();

  const selectedDateReminders = reminders.filter(
    (r) => new Date(r.dueTimestampMs).toDateString() === selectedStr
  );

  return (
    <div className="flex items-stretch h-full gap-2.5 select-none pt-0.5 pb-1 pl-0.5">
      {/* Left 3D Date Hero Card (Matches reference-calendar.png) */}
      <div className="w-[96px] shrink-0 rounded-2xl orbit-card-3d flex flex-col items-center justify-between py-2 px-1.5 relative overflow-hidden">
        <div className="text-center leading-tight">
          <div
            className="text-[11px] font-bold tracking-tight truncate cursor-pointer hover:opacity-80"
            style={{ color: activeThemeMeta.color }}
            onClick={(e) => {
              e.stopPropagation();
              const now = new Date();
              setCalendarMonth(now);
              setSelectedCalendarDate(now);
            }}
            title="Click to jump to Today"
          >
            {monthName}
          </div>
          <div className="text-[9.5px] font-medium text-white/45 tabular-nums">
            {selectedYear}
          </div>
        </div>

        {/* Sculpted 3D Day Number */}
        <div
          className="text-[40px] font-bold leading-none tracking-tight tabular-nums my-auto orbit-clock-3d"
          style={{ color: "var(--island-text)" }}
        >
          {selectedDayNum}
        </div>

        {/* Bottom Weekday or Task Toggle Pill */}
        <div className="w-full flex flex-col items-center">
          <span className="text-[10.5px] font-semibold text-white/55 truncate">
            {selectedWeekday}
          </span>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setShowTaskDrawer((prev) => !prev);
            }}
            className="mt-1 px-2 py-0.5 rounded-full text-[8.5px] font-bold transition-colors cursor-pointer orbit-pill-badge-3d"
            style={{
              color:
                selectedDateReminders.length > 0
                  ? activeThemeMeta.color
                  : "rgba(255,255,255,0.65)",
            }}
            title="View or add tasks for this date"
          >
            {showTaskDrawer
              ? "Calendar"
              : selectedDateReminders.length > 0
              ? `${selectedDateReminders.length} Task${
                  selectedDateReminders.length > 1 ? "s" : ""
                }`
              : "+ Add Task"}
          </button>
        </div>
      </div>

      {/* Right Column: 7-Day 3D Calendar Grid OR Task Drawer for Selected Date */}
      <div className="flex-1 flex flex-col justify-between min-w-0 py-0.5">
        {showTaskDrawer ? (
          <div className="flex flex-col h-full justify-between">
            <div className="flex items-center justify-between pb-1 border-b border-white/10">
              <span className="text-[10.5px] font-bold text-white/85 truncate">
                Tasks · {selectedCalendarDate.toLocaleString("default", { month: "short", day: "numeric" })}
              </span>
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  setShowTaskDrawer(false);
                }}
                className="text-[9.5px] px-1.5 py-0.5 rounded bg-white/10 hover:bg-white/20 text-white/80 cursor-pointer"
              >
                Grid
              </button>
            </div>

            <div className="flex-1 overflow-y-auto space-y-1 my-1 pr-0.5">
              {selectedDateReminders.length === 0 ? (
                <div className="h-full flex items-center justify-center text-[10px] text-white/40">
                  No tasks scheduled for this date.
                </div>
              ) : (
                selectedDateReminders.map((rem) => (
                  <div
                    key={rem.id}
                    className="flex items-center justify-between px-2 py-1 rounded-lg orbit-well-3d"
                  >
                    <div className="flex items-center space-x-1.5 min-w-0 flex-1 mr-1">
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onToggleReminder(rem.id);
                        }}
                        className={`w-3.5 h-3.5 rounded flex items-center justify-center shrink-0 border transition-colors cursor-pointer ${
                          rem.isCompleted
                            ? "bg-emerald-500/30 border-emerald-400 text-emerald-300"
                            : "border-white/30 hover:border-white/60"
                        }`}
                      >
                        {rem.isCompleted && (
                          <span className="text-[8px] font-bold">✓</span>
                        )}
                      </button>
                      <span
                        className={`text-[10px] truncate font-medium ${
                          rem.isCompleted
                            ? "line-through text-white/40"
                            : "text-white/95"
                        }`}
                        title={rem.title}
                      >
                        {rem.title}
                      </span>
                    </div>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteReminder(rem.id);
                      }}
                      className="text-white/35 hover:text-rose-400 text-[10px] px-1 cursor-pointer"
                      title="Delete task"
                    >
                      ✕
                    </button>
                  </div>
                ))
              )}
            </div>

            <form
              onSubmit={(e) => {
                e.preventDefault();
                onAddReminder(e);
              }}
              className="flex items-center space-x-1 pt-1 border-t border-white/10 shrink-0"
            >
              <input
                type="text"
                value={newReminderTitle}
                onChange={(e) => setNewReminderTitle(e.target.value)}
                placeholder="New task title..."
                className="flex-1 min-w-0 orbit-well-3d rounded-md px-2 py-0.5 text-[9.5px] text-white placeholder-white/35 focus:outline-none focus:border-white/30"
              />
              <button
                type="submit"
                disabled={!newReminderTitle.trim()}
                className="px-2 py-0.5 rounded-md orbit-pill-badge-3d font-bold text-[9.5px] disabled:opacity-30 cursor-pointer"
                style={{ color: activeThemeMeta.accent }}
              >
                Add
              </button>
            </form>
          </div>
        ) : (
          <>
            {/* Weekday Header + Month Prev/Next Controls (Matches reference-calendar.png) */}
            <div className="flex items-center justify-between pb-1 border-b border-white/10 shrink-0">
              <div className="grid grid-cols-7 flex-1 text-center text-[9.5px] font-bold text-white/40">
                {["S", "M", "T", "W", "T", "F", "S"].map((d, idx) => (
                  <span key={`${d}-${idx}`}>{d}</span>
                ))}
              </div>
              <div className="flex items-center space-x-0.5 ml-1 shrink-0">
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    const prev = new Date(curYear, curMonth - 1, 1);
                    setCalendarMonth(prev);
                    setSelectedCalendarDate(prev);
                  }}
                  className="w-4 h-4 rounded flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 text-[10px] font-bold cursor-pointer"
                  title="Previous month"
                >
                  ‹
                </button>
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    const next = new Date(curYear, curMonth + 1, 1);
                    setCalendarMonth(next);
                    setSelectedCalendarDate(next);
                  }}
                  className="w-4 h-4 rounded flex items-center justify-center text-white/45 hover:text-white hover:bg-white/10 text-[10px] font-bold cursor-pointer"
                  title="Next month"
                >
                  ›
                </button>
              </div>
            </div>

            {/* 7-Column Calendar Matrix with 3D Glowing Selected Sphere */}
            <div className="grid grid-cols-7 gap-y-0.5 text-center my-auto pr-4">
              {Array.from({ length: firstDayIndex }).map((_, i) => (
                <div key={`empty-${i}`} className="h-[18px]" />
              ))}
              {Array.from({ length: daysInMonth }).map((_, i) => {
                const dayNum = i + 1;
                const cellDate = new Date(curYear, curMonth, dayNum);
                const cellDateStr = cellDate.toDateString();
                const isToday = cellDateStr === todayStr;
                const isSelected = cellDateStr === selectedStr;
                const isWeekend =
                  cellDate.getDay() === 0 || cellDate.getDay() === 6;
                const hasReminders = reminders.some(
                  (r) =>
                    new Date(r.dueTimestampMs).toDateString() === cellDateStr
                );

                return (
                  <button
                    key={`day-${dayNum}`}
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      setSelectedCalendarDate(cellDate);
                    }}
                    className={`h-[18px] w-[22px] mx-auto rounded-full flex flex-col items-center justify-center relative text-[10px] font-bold tabular-nums transition-all cursor-pointer ${
                      isSelected
                        ? "scale-105"
                        : isToday
                        ? "border border-white/25"
                        : isWeekend
                        ? "text-white/45 hover:bg-white/10 hover:text-white/80"
                        : "text-white/90 hover:bg-white/10"
                    }`}
                    style={
                      isSelected
                        ? {
                            backgroundColor: activeThemeMeta.accent,
                            color:
                              activeThemeMeta.id === "black" ||
                              activeThemeMeta.id === "dark"
                                ? "#090a0f"
                                : "#ffffff",
                            boxShadow: `0 2px 8px ${activeThemeMeta.borderGlow}`,
                          }
                        : isToday
                        ? {
                            color: activeThemeMeta.accent,
                          }
                        : undefined
                    }
                  >
                    <span>{dayNum}</span>
                    {hasReminders && !isSelected && (
                      <span
                        className="w-1 h-1 rounded-full absolute -bottom-0.5"
                        style={{ backgroundColor: activeThemeMeta.accent }}
                      />
                    )}
                  </button>
                );
              })}
            </div>
          </>
        )}
      </div>
    </div>
  );
};
