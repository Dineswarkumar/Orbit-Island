import { invoke } from "@tauri-apps/api/core";
import { AppConfig, AppConfigSchema } from "../types/config";
import { MemoryReport } from "../types/telemetry";
import { MediaSessionSnapshot, MediaSourceInfo } from "../types/media";
import { NotificationSnapshot, PrivacyState } from "../types/notifications";
import { ReminderItem } from "../types/reminders";
import { HardwareMetrics, MemoryCleanResult } from "../types/hardware";
import { ShelfItem } from "../types/shelf";

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

  hideIslandWindow: async (): Promise<void> => {
    return invoke<void>("hide_island_window");
  },

  getMemoryMetrics: async (): Promise<MemoryReport> => {
    return invoke<MemoryReport>("get_memory_metrics");
  },

  setIslandState: async (state: "compact" | "compact_alert" | "notification" | "expanded"): Promise<void> => {
    return invoke<void>("set_island_state", { state });
  },

  showContextMenu: async (): Promise<boolean> => {
    return invoke<boolean>("show_context_menu");
  },

  getMediaState: async (pinnedSource?: string | null): Promise<MediaSessionSnapshot | null> => {
    return invoke<MediaSessionSnapshot | null>("get_media_state", { pinnedSource: pinnedSource ?? null });
  },

  getMediaSources: async (pinnedSource?: string | null): Promise<MediaSourceInfo[]> => {
    return invoke<MediaSourceInfo[]>("get_media_sources", { pinnedSource: pinnedSource ?? null });
  },

  pinMediaSource: async (sourceId: string | null): Promise<MediaSessionSnapshot | null> => {
    return invoke<MediaSessionSnapshot | null>("pin_media_source", { sourceId: sourceId ?? null });
  },

  sendMediaCommand: async (command: string, pinnedSource?: string | null): Promise<MediaSessionSnapshot | null> => {
    return invoke<MediaSessionSnapshot | null>("media_send_command", { command, pinnedSource: pinnedSource ?? null });
  },

  seekMedia: async (positionSecs: number, pinnedSource?: string | null): Promise<MediaSessionSnapshot | null> => {
    return invoke<MediaSessionSnapshot | null>("media_seek", { positionSecs, pinnedSource: pinnedSource ?? null });
  },

  getNotificationState: async (): Promise<NotificationSnapshot> => {
    return invoke<NotificationSnapshot>("get_notification_state");
  },

  getNotificationAccessStatus: async (): Promise<string> => {
    return invoke<string>("get_notification_access_status");
  },

  requestNotificationAccess: async (): Promise<string> => {
    return invoke<string>("request_notification_access");
  },

  dismissNotification: async (id: number): Promise<void> => {
    return invoke<void>("dismiss_notification", { id });
  },

  clearAllNotifications: async (): Promise<void> => {
    return invoke<void>("clear_all_notifications");
  },

  getPrivacyState: async (): Promise<PrivacyState> => {
    return invoke<PrivacyState>("get_privacy_state");
  },

  // Reminders & Productivity Hub (SQLite persistence)
  getReminders: async (): Promise<ReminderItem[]> => {
    return invoke<ReminderItem[]>("get_reminders");
  },

  addReminder: async (
    title: string,
    dueTimestampMs: number,
    category = "task",
    source = "local"
  ): Promise<ReminderItem> => {
    return invoke<ReminderItem>("add_reminder", {
      title,
      dueTimestampMs,
      category,
      source,
    });
  },

  toggleReminder: async (id: string): Promise<boolean> => {
    return invoke<boolean>("toggle_reminder", { id });
  },

  deleteReminder: async (id: string): Promise<void> => {
    return invoke<void>("delete_reminder", { id });
  },

  // Hardware & System Monitor
  getHardwareMetrics: async (): Promise<HardwareMetrics> => {
    return invoke<HardwareMetrics>("get_hardware_metrics");
  },

  freeUpMemory: async (): Promise<MemoryCleanResult> => {
    return invoke<MemoryCleanResult>("free_up_memory");
  },

  // File Shelf (Max 10 held references, SQLite persistence)
  getShelfItems: async (): Promise<ShelfItem[]> => {
    return invoke<ShelfItem[]>("get_shelf_items");
  },

  addShelfItem: async (filePath: string): Promise<ShelfItem> => {
    return invoke<ShelfItem>("add_shelf_item", { filePath });
  },

  removeShelfItem: async (id: string): Promise<void> => {
    return invoke<void>("remove_shelf_item", { id });
  },

  openShelfFile: async (id: string): Promise<void> => {
    return invoke<void>("open_shelf_file", { id });
  },

  showShelfFileInFolder: async (id: string): Promise<void> => {
    return invoke<void>("show_shelf_file_in_folder", { id });
  },

  pickShelfFile: async (): Promise<string | null> => {
    return invoke<string | null>("pick_shelf_file");
  },

  clearShelf: async (): Promise<void> => {
    return invoke<void>("clear_shelf");
  },

  clearLocalData: async (): Promise<void> => {
    return invoke<void>("clear_local_data");
  },
};
