import { create } from "zustand";
import { AppConfig, AppConfigSchema } from "../types/config";
import { ipc } from "../ipc/client";

interface ConfigStore {
  config: AppConfig;
  isLoaded: boolean;
  isLoading: boolean;
  isSaving: boolean;
  error: string | null;
  loadConfig: () => Promise<void>;
  updateConfig: (patch: Partial<AppConfig>) => Promise<boolean>;
}

export const useConfigStore = create<ConfigStore>((set, get) => ({
  config: AppConfigSchema.parse({}),
  isLoaded: false,
  isLoading: false,
  isSaving: false,
  error: null,

  loadConfig: async () => {
    set({ isLoading: true, error: null });
    try {
      const config = await ipc.getConfig();
      set({ config, isLoaded: true, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  updateConfig: async (patch: Partial<AppConfig>): Promise<boolean> => {
    const current = get().config;
    const merged = { ...current, ...patch };
    
    set({ isSaving: true, error: null });
    try {
      const validated = AppConfigSchema.parse(merged);
      const saved = await ipc.saveConfig(validated);
      set({ config: saved, isSaving: false, error: null });
      return true;
    } catch (err) {
      set({ error: String(err), isSaving: false });
      return false;
    }
  },
}));
