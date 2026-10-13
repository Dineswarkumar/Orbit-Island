import { create } from "zustand";
import { AppConfig, AppConfigSchema } from "../types/config";
import { ipc } from "../ipc/client";

interface ConfigStore {
  config: AppConfig;
  isLoading: boolean;
  isSaving: boolean;
  error: string | null;
  loadConfig: () => Promise<void>;
  updateConfig: (patch: Partial<AppConfig>) => Promise<void>;
}

export const useConfigStore = create<ConfigStore>((set, get) => ({
  config: AppConfigSchema.parse({}),
  isLoading: false,
  isSaving: false,
  error: null,

  loadConfig: async () => {
    set({ isLoading: true, error: null });
    try {
      const config = await ipc.getConfig();
      set({ config, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  updateConfig: async (patch: Partial<AppConfig>) => {
    const current = get().config;
    const merged = { ...current, ...patch };
    const validated = AppConfigSchema.parse(merged);

    set({ isSaving: true, error: null });
    try {
      const saved = await ipc.saveConfig(validated);
      set({ config: saved, isSaving: false });
    } catch (err) {
      set({ error: String(err), isSaving: false });
    }
  },
}));
