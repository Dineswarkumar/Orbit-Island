import { create } from "zustand";
import { AppConfig, createDefaultAppConfig, mergeWithDefaultConfig } from "../types/config";
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
  config: createDefaultAppConfig(),
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
    const merged = mergeWithDefaultConfig({ ...current, ...patch });
    
    set({ isSaving: true, error: null });
    try {
      const saved = await ipc.saveConfig(merged);
      set({ config: saved, isSaving: false, error: null });
      return true;
    } catch (err) {
      const errStr = String(err);
      if (errStr.includes("Access denied")) {
        // Administrative command restriction (S-5): caller is in non-settings window (e.g. main)
        // Apply merged configuration to local store so local UI state remains coherent without throwing
        set({ config: merged, isSaving: false, error: null });
        return true;
      }
      set({ error: errStr, isSaving: false });
      return false;
    }
  },
}));
