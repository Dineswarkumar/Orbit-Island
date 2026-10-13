import { invoke } from "@tauri-apps/api/core";
import { AppConfig, AppConfigSchema } from "../types/config";
import { MemoryReport } from "../types/telemetry";

export const ipc = {
  getConfig: async (): Promise<AppConfig> => {
    const raw = await invoke<unknown>("get_config");
    return AppConfigSchema.parse(raw);
  },

  saveConfig: async (config: AppConfig): Promise<AppConfig> => {
    const validated = AppConfigSchema.parse(config);
    const raw = await invoke<unknown>("save_config", { config: validated });
    return AppConfigSchema.parse(raw);
  },

  openSettingsWindow: async (): Promise<void> => {
    return invoke<void>("open_settings_window");
  },

  closeSettingsWindow: async (): Promise<void> => {
    return invoke<void>("close_settings_window");
  },

  getMemoryMetrics: async (): Promise<MemoryReport> => {
    return invoke<MemoryReport>("get_memory_metrics");
  },

  centerIslandWindow: async (): Promise<void> => {
    return invoke<void>("center_island_window");
  },
};
