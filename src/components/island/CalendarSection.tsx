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
  onAddReminder: (e?: React.FormEvent, category?: string) => void;
  onToggleReminder: (id: string) => void;
  onDeleteReminder: (id: string) => void;
  onImportCalendar?: () => void;
  onRefreshReminders?: () => void;
}

export interface CategoryStyle {
  hex: string;
  bg: string;
  border: string;
  text: string;
  badge: string;
}

const getCategoryStyle = (cat: string, themeColor: string): CategoryStyle => {
  const c = (cat || "").toLowerCase();
  switch (c) {
    case "exam":
      return {
        hex: "#fb7185",
        bg: "rgba(251, 113, 133, 0.28)",
        border: "#fb7185",
        text: "#ffe4e6",
        badge: "EXAM",
      };
    case "deadline":
    case "due":
      return {
        hex: "#f97316",
        bg: "rgba(249, 115, 22, 0.28)",
        border: "#f97316",
        text: "#ffedd5",
        badge: "DUE",
      };
    case "holiday":
      return {
        hex: "#34d399",
        bg: "rgba(52, 211, 153, 0.28)",
        border: "#34d399",
        text: "#d1fae5",
        badge: "HOLIDAY",
      };
    case "event":
      return {
        hex: "#fbbf24",
        bg: "rgba(251, 191, 36, 0.28)",
        border: "#fbbf24",
        text: "#fef3c7",
        badge: "EVENT",
      };
    case "meeting":
      return {
        hex: "#a855f7",
        bg: "rgba(168, 85, 247, 0.28)",
        border: "#a855f7",
        text: "#f3e8ff",
        badge: "MEETING",
      };
    case "class":
      return {
        hex: "#38bdf8",
        bg: "rgba(56, 189, 248, 0.28)",
        border: "#38bdf8",
        text: "#e0f2fe",
        badge: "CLASS",
      };
    default:
      return {
        hex: themeColor,
        bg: "rgba(255, 255, 255, 0.14)",
        border: themeColor,
        text: "#ffffff",
        badge: "TASK",
      };
  }
};

const getDayPrimaryCategory = (
  items: ReminderItem[],
  themeColor: string
): CategoryStyle | null => {
  if (items.length === 0) return null;
  const priorityOrder = [
    "exam",
    "deadline",
    "due",
    "holiday",
    "event",
    "meeting",
    "class",
    "task",
    "reminder",
  ];
  for (const prio of priorityOrder) {
    const match = items.find(
      (it) => (it.category || "task").toLowerCase() === prio
    );
    if (match) return getCategoryStyle(match.category, themeColor);
  }
  return getCategoryStyle(items[0].category || "task", themeColor);
};

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
  onImportCalendar,
}) => {
  const [showTaskDrawer, setShowTaskDrawer] = useState(false);
  const [newTaskCategory, setNewTaskCategory] = useState<string>("task");

  const curYear = calendarMonth.getFullYear();
  const curMonth = calendarMonth.getMonth();
  const daysInMonth = new Date(curYear, curMonth + 1, 0).getDate();
  const firstDayIndex = new Date(curYear, curMonth, 1).getDay();
  const monthName = selectedCalendarDate.toLocaleString("default", {
    month: "long",
  });
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

  const selectedDayStyle = getDayPrimaryCategory(
    selectedDateReminders,
    activeThemeMeta.color
  );

  return (
    <div className="flex items-stretch h-full gap-2.5 select-none pt-0.5 pb-1 pl-0.5">
      {/* Left 3D Date Hero Card */}
      <div
        className="w-[98px] shrink-0 rounded-2xl orbit-card-3d flex flex-col items-center justify-between py-2 px-1.5 relative overflow-hidden"
        style={
          selectedDayStyle
            ? {
                borderColor: `${selectedDayStyle.hex}50`,
                boxShadow: `0 4px 16px ${selectedDayStyle.hex}20, inset 0 1px 0 rgba(255,255,255,0.15)`,
              }
            : undefined
        }
      >
        <div className="text-center leading-tight">
          <div
            className="text-[12px] font-extrabold tracking-tight truncate cursor-pointer hover:opacity-80 transition-opacity"
            style={{
              color: selectedDayStyle ? selectedDayStyle.hex : activeThemeMeta.color,
            }}
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
          <div className="text-[10px] font-semibold text-white/50 tabular-nums">
            {selectedYear}
          </div>
        </div>

        {/* Sculpted 3D Day Number */}
        <div
          className="text-[40px] font-bold leading-none tracking-tight tabular-nums my-auto orbit-clock-3d"
          style={{
            color: "var(--island-text)",
            textShadow: selectedDayStyle
              ? `0 2px 12px ${selectedDayStyle.hex}40`
              : undefined,
          }}
        >
          {selectedDayNum}
        </div>

        {/* Bottom Weekday & Summary Badge */}
        <div className="w-full flex flex-col items-center">
          <span
            className="text-[11px] font-semibold truncate"
            style={{ color: "var(--island-muted)" }}
          >
            {selectedWeekday}
          </span>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setShowTaskDrawer((prev) => !prev);
            }}
            className="mt-1 px-2.5 py-0.5 rounded-full text-[9.5px] font-extrabold transition-all cursor-pointer orbit-pill-badge-3d max-w-[92px] truncate"
            style={
              selectedDayStyle
                ? {
                    backgroundColor: selectedDayStyle.bg,
                    border: `1px solid ${selectedDayStyle.border}`,
                    color: selectedDayStyle.text,
                    boxShadow: `0 1px 8px ${selectedDayStyle.hex}40`,
                  }
                : {
                    color: "var(--island-muted)",
                  }
            }
            title="View or add tasks for this date"
          >
            {showTaskDrawer
              ? "Calendar"
              : selectedDateReminders.length === 1 && selectedDayStyle
              ? selectedDayStyle.badge
              : selectedDateReminders.length > 1
              ? `${selectedDateReminders.length} Events`
              : "+ Add Task"}
          </button>
        </div>
      </div>

      {/* Right Column: 7-Day 3D Calendar Grid OR Task Drawer for Selected Date */}
      <div className="flex-1 flex flex-col justify-between min-w-0 py-0.5">
        {showTaskDrawer ? (
          <div className="flex flex-col h-full justify-between">
            <div className="flex items-center justify-between pb-1 border-b border-white/10 shrink-0">
              <span
                className="text-[11.5px] font-bold truncate"
                style={{ color: "var(--island-text)" }}
              >
                Schedule · {selectedCalendarDate.toLocaleString("default", { month: "short", day: "numeric" })}
              </span>
              <div className="flex items-center space-x-1.5">
                {onImportCalendar && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onImportCalendar();
                    }}
                    className="text-[9.5px] font-bold px-2 py-0.5 rounded bg-white/15 hover:bg-white/25 text-white/90 cursor-pointer flex items-center space-x-1 transition-colors"
                    title="Import Academic or Office Schedule (.ics, .csv, .json)"
                  >
                    <span>📥 Import</span>
                  </button>
                )}
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    setShowTaskDrawer(false);
                  }}
                  className="text-[9.5px] font-bold px-2 py-0.5 rounded bg-white/10 hover:bg-white/20 text-white/80 cursor-pointer"
                >
                  Grid
                </button>
              </div>
            </div>

            <div className="flex-1 overflow-y-auto space-y-1 my-1 pr-0.5">
              {selectedDateReminders.length === 0 ? (
                <div className="h-full flex items-center justify-center text-[11px] font-medium text-white/45">
                  No events scheduled for this date.
                </div>
              ) : (
                selectedDateReminders.map((rem) => {
                  const catStyle = getCategoryStyle(
                    rem.category || "task",
                    activeThemeMeta.color
                  );
                  const remDate = new Date(rem.dueTimestampMs);
                  const hasTime =
                    remDate.getHours() !== 0 || remDate.getMinutes() !== 0;
                  const timeStr = hasTime
                    ? remDate.toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                      })
                    : "";

                  return (
                    <div
                      key={rem.id}
                      className="flex items-center justify-between px-2.5 py-1 rounded-lg orbit-well-3d"
                    >
                      <div className="flex items-center space-x-2 min-w-0 flex-1 mr-1">
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onToggleReminder(rem.id);
                          }}
                          className={`w-4 h-4 rounded flex items-center justify-center shrink-0 border transition-colors cursor-pointer ${
                            rem.isCompleted
                              ? "bg-emerald-500/30 border-emerald-400 text-emerald-300"
                              : "border-white/30 hover:border-white/60"
                          }`}
                        >
                          {rem.isCompleted && (
                            <span className="text-[9px] font-bold">✓</span>
                          )}
                        </button>
                        <span
                          className="px-1.5 py-0.2 rounded text-[8.5px] font-extrabold uppercase shrink-0"
                          style={{
                            backgroundColor: catStyle.bg,
                            color: catStyle.text,
                            border: `1px solid ${catStyle.border}60`,
                          }}
                        >
                          {catStyle.badge}
                        </span>
                        {timeStr && (
                          <span className="text-[9.5px] font-mono text-white/60 tabular-nums shrink-0">
                            {timeStr}
                          </span>
                        )}
                        <span
                          className={`text-[11.5px] truncate font-semibold ${
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
                        className="text-white/40 hover:text-rose-400 text-[11px] px-1 cursor-pointer"
                        title="Delete event"
                      >
                        ✕
                      </button>
                    </div>
                  );
                })
              )}
            </div>

            <form
              onSubmit={(e) => {
                e.preventDefault();
                onAddReminder(e, newTaskCategory);
              }}
              className="flex items-center space-x-1.5 pt-1 border-t border-white/10 shrink-0"
            >
              <button
                type="button"
                onClick={(e) => {
                  e.stopPropagation();
                  const cats = [
                    "task",
                    "exam",
                    "event",
                    "holiday",
                    "meeting",
                    "class",
                    "deadline",
                  ];
                  const nextIdx =
                    (cats.indexOf(newTaskCategory) + 1) % cats.length;
                  setNewTaskCategory(cats[nextIdx]);
                }}
                className="px-2 py-0.5 rounded text-[9px] font-extrabold uppercase shrink-0 transition-colors cursor-pointer"
                style={{
                  backgroundColor: getCategoryStyle(
                    newTaskCategory,
                    activeThemeMeta.color
                  ).bg,
                  color: getCategoryStyle(
                    newTaskCategory,
                    activeThemeMeta.color
                  ).text,
                  border: `1px solid ${
                    getCategoryStyle(newTaskCategory, activeThemeMeta.color)
                      .border
                  }60`,
                }}
                title="Click to cycle event category"
              >
                {newTaskCategory}
              </button>
              <input
                type="text"
                value={newReminderTitle}
                onChange={(e) => setNewReminderTitle(e.target.value)}
                placeholder="New task or event..."
                className="flex-1 min-w-0 orbit-well-3d rounded-md px-2 py-0.5 text-[10.5px] text-white placeholder-white/40 focus:outline-none focus:border-white/30"
              />
              <button
                type="submit"
                disabled={!newReminderTitle.trim()}
                className="px-2.5 py-0.5 rounded-md orbit-pill-badge-3d font-bold text-[10.5px] disabled:opacity-30 cursor-pointer"
                style={{ color: activeThemeMeta.accent }}
              >
                Add
              </button>
            </form>
          </div>
        ) : (
          <>
            {/* Weekday Header + Month Prev/Next Controls */}
            <div className="flex items-center justify-between pb-1 border-b border-white/10 shrink-0">
              <div
                className="grid grid-cols-7 flex-1 text-center text-[10px] font-bold"
                style={{ color: "var(--island-muted)" }}
              >
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
                  className="w-4 h-4 rounded flex items-center justify-center text-white/50 hover:text-white hover:bg-white/10 text-[11px] font-bold cursor-pointer"
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
                  className="w-4 h-4 rounded flex items-center justify-center text-white/50 hover:text-white hover:bg-white/10 text-[11px] font-bold cursor-pointer"
                  title="Next month"
                >
                  ›
                </button>
              </div>
            </div>

            {/* 7-Column Calendar Matrix with Full-Cell Category Highlighting (Zero Micro-Dot Collisions) */}
            <div className="grid grid-cols-7 gap-y-1 text-center my-auto pr-4">
              {Array.from({ length: firstDayIndex }).map((_, i) => (
                <div key={`empty-${i}`} className="h-[20px]" />
              ))}
              {Array.from({ length: daysInMonth }).map((_, i) => {
                const dayNum = i + 1;
                const cellDate = new Date(curYear, curMonth, dayNum);
                const cellDateStr = cellDate.toDateString();
                const isToday = cellDateStr === todayStr;
                const isSelected = cellDateStr === selectedStr;
                const isWeekend =
                  cellDate.getDay() === 0 || cellDate.getDay() === 6;
                const dayReminders = reminders.filter(
                  (r) =>
                    new Date(r.dueTimestampMs).toDateString() === cellDateStr
                );
                const dayStyle = getDayPrimaryCategory(
                  dayReminders,
                  activeThemeMeta.accent
                );

                return (
                  <button
                    key={`day-${dayNum}`}
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      setSelectedCalendarDate(cellDate);
                    }}
                    className={`h-[20px] w-[24px] mx-auto rounded-full flex items-center justify-center relative text-[11px] font-bold tabular-nums transition-all cursor-pointer ${
                      isSelected
                        ? "scale-105"
                        : dayStyle
                        ? "hover:scale-105"
                        : isToday
                        ? "border border-white/40"
                        : isWeekend
                        ? "text-white/45 hover:bg-white/10 hover:text-white/80"
                        : "text-white/90 hover:bg-white/10"
                    }`}
                    style={
                      isSelected
                        ? {
                            backgroundColor: dayStyle
                              ? dayStyle.hex
                              : activeThemeMeta.accent,
                            color:
                              dayStyle &&
                              (dayStyle.badge === "EXAM" ||
                                dayStyle.badge === "MEETING")
                                ? "#ffffff"
                                : activeThemeMeta.id === "black" ||
                                  activeThemeMeta.id === "dark"
                                ? "#090a0f"
                                : "#ffffff",
                            boxShadow: `0 0 0 1.5px #ffffff, 0 2px 10px ${
                              dayStyle ? dayStyle.hex : activeThemeMeta.accent
                            }90`,
                          }
                        : dayStyle
                        ? {
                            backgroundColor: dayStyle.bg,
                            border: `1px solid ${dayStyle.border}`,
                            color: dayStyle.text,
                            boxShadow: `0 1px 6px ${dayStyle.hex}35`,
                          }
                        : isToday
                        ? {
                            color: activeThemeMeta.accent,
                          }
                        : undefined
                    }
                  >
                    <span>{dayNum}</span>
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
