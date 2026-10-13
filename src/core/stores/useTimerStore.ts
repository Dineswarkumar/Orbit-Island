import { create } from "zustand";

export interface TimerState {
  durationSeconds: number;
  remainingSeconds: number;
  isRunning: boolean;
  targetEndTime: number | null; // Unix timestamp in ms for persistent recovery
  startTimer: (duration?: number) => void;
  pauseTimer: () => void;
  resetTimer: (newDuration?: number) => void;
  tick: () => void;
}

const STORAGE_KEY = "orbit_timer_state";

interface StoredTimer {
  durationSeconds: number;
  remainingSeconds: number;
  isRunning: boolean;
  targetEndTime: number | null;
}

function loadInitialState(): { durationSeconds: number; remainingSeconds: number; isRunning: boolean; targetEndTime: number | null } {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed: StoredTimer = JSON.parse(raw);
      if (parsed.isRunning && parsed.targetEndTime) {
        const now = Date.now();
        const diffSecs = Math.max(0, Math.round((parsed.targetEndTime - now) / 1000));
        return {
          durationSeconds: parsed.durationSeconds,
          remainingSeconds: diffSecs,
          isRunning: diffSecs > 0,
          targetEndTime: diffSecs > 0 ? parsed.targetEndTime : null,
        };
      }
      return {
        durationSeconds: parsed.durationSeconds || 1500, // 25 min default
        remainingSeconds: parsed.remainingSeconds ?? 1500,
        isRunning: false,
        targetEndTime: null,
      };
    }
  } catch {
    // Fallback on clean state
  }

  return {
    durationSeconds: 1500,
    remainingSeconds: 1500,
    isRunning: false,
    targetEndTime: null,
  };
}

function saveState(state: { durationSeconds: number; remainingSeconds: number; isRunning: boolean; targetEndTime: number | null }) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // Silent storage write error handling
  }
}

export const useTimerStore = create<TimerState>((set, get) => {
  const initial = loadInitialState();

  return {
    durationSeconds: initial.durationSeconds,
    remainingSeconds: initial.remainingSeconds,
    isRunning: initial.isRunning,
    targetEndTime: initial.targetEndTime,

    startTimer: (customDuration?: number) => {
      const duration = customDuration ?? get().remainingSeconds;
      const targetEndTime = Date.now() + duration * 1000;
      const next = {
        durationSeconds: customDuration ? duration : get().durationSeconds,
        remainingSeconds: duration,
        isRunning: true,
        targetEndTime,
      };
      saveState(next);
      set(next);
    },

    pauseTimer: () => {
      const remainingSeconds = get().remainingSeconds;
      const next = {
        ...get(),
        isRunning: false,
        targetEndTime: null,
      };
      saveState({
        durationSeconds: get().durationSeconds,
        remainingSeconds,
        isRunning: false,
        targetEndTime: null,
      });
      set(next);
    },

    resetTimer: (newDuration?: number) => {
      const duration = newDuration ?? get().durationSeconds;
      const next = {
        durationSeconds: duration,
        remainingSeconds: duration,
        isRunning: false,
        targetEndTime: null,
      };
      saveState(next);
      set(next);
    },

    tick: () => {
      const { isRunning, targetEndTime } = get();
      if (!isRunning || !targetEndTime) return;

      const now = Date.now();
      const diffSecs = Math.max(0, Math.round((targetEndTime - now) / 1000));

      if (diffSecs <= 0) {
        const next = {
          remainingSeconds: 0,
          isRunning: false,
          targetEndTime: null,
        };
        saveState({
          durationSeconds: get().durationSeconds,
          remainingSeconds: 0,
          isRunning: false,
          targetEndTime: null,
        });
        set(next);
      } else {
        set({ remainingSeconds: diffSecs });
      }
    },
  };
});
