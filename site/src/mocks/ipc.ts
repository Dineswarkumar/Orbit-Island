import type { AppConfig } from "@app/core/types/config";
import type { MemoryReport } from "@app/core/types/telemetry";
import type { MediaSessionSnapshot, MediaSourceInfo } from "@app/core/types/media";
import type { NotificationSnapshot, PrivacyState } from "@app/core/types/notifications";
import type { ReminderItem } from "@app/core/types/reminders";
import type { HardwareMetrics, MemoryCleanResult } from "@app/core/types/hardware";
import type { ShelfItem } from "@app/core/types/shelf";
import {
  appConfigFixture,
  hardwareMetricsFixture,
  mediaSessionFixture,
  mediaSourcesFixture,
  memoryCleanResultFixture,
  memoryReportFixture,
  notificationSnapshotFixture,
  privacyFixture,
  reminderFixtures,
  shelfFixtures,
} from "./fixtures";

type RealIpc = typeof import("../../../src/core/ipc/client").ipc;

export const ipc: RealIpc = {
  getConfig: async (): Promise<AppConfig> => appConfigFixture,
  saveConfig: async (config: AppConfig): Promise<AppConfig> => config,
  openSettingsWindow: async (): Promise<void> => undefined,
  closeSettingsWindow: async (): Promise<void> => undefined,
  hideIslandWindow: async (): Promise<void> => undefined,
  getMemoryMetrics: async (): Promise<MemoryReport> => memoryReportFixture,
  setIslandState: async (_state): Promise<void> => undefined,
  updateIslandRegion: async (_state, _width): Promise<void> => undefined,
  showContextMenu: async (): Promise<boolean> => false,
  getMediaState: async (_pinnedSource?: string | null): Promise<MediaSessionSnapshot | null> =>
    mediaSessionFixture,
  getMediaSources: async (_pinnedSource?: string | null): Promise<MediaSourceInfo[]> =>
    mediaSourcesFixture,
  pinMediaSource: async (_sourceId: string | null): Promise<MediaSessionSnapshot | null> =>
    mediaSessionFixture,
  sendMediaCommand: async (
    _command: string,
    _pinnedSource?: string | null,
  ): Promise<MediaSessionSnapshot | null> => mediaSessionFixture,
  seekMedia: async (
    _positionSecs: number,
    _pinnedSource?: string | null,
  ): Promise<MediaSessionSnapshot | null> => mediaSessionFixture,
  getNotificationState: async (): Promise<NotificationSnapshot> => notificationSnapshotFixture,
  getNotificationAccessStatus: async (): Promise<string> => "allowed",
  requestNotificationAccess: async (): Promise<string> => "allowed",
  dismissNotification: async (_id: number): Promise<void> => undefined,
  clearAllNotifications: async (): Promise<void> => undefined,
  getPrivacyState: async (): Promise<PrivacyState> => privacyFixture,
  triggerPrivacyTest: async (): Promise<void> => undefined,
  getReminders: async (): Promise<ReminderItem[]> => reminderFixtures,
  addReminder: async (
    title: string,
    dueTimestampMs: number,
    category = "task",
    source = "local",
  ): Promise<ReminderItem> => ({
    id: `demo-${Date.now()}`,
    title,
    dueTimestampMs,
    category,
    source,
    isCompleted: false,
    createdAt: Date.now(),
  }),
  toggleReminder: async (_id: string): Promise<boolean> => true,
  deleteReminder: async (_id: string): Promise<void> => undefined,
  pickAndImportCalendar: async (
    _sourceTag: "academic" | "office" | "ics" | string = "ics",
  ): Promise<number> => 0,
  clearImportedCalendar: async (_sourceFilter?: string | null): Promise<number> => 0,
  getHardwareMetrics: async (): Promise<HardwareMetrics> => hardwareMetricsFixture,
  freeUpMemory: async (): Promise<MemoryCleanResult> => memoryCleanResultFixture,
  getShelfItems: async (): Promise<ShelfItem[]> => shelfFixtures,
  addShelfItem: async (filePath: string): Promise<ShelfItem> => ({
    id: `demo-${Date.now()}`,
    filePath,
    displayName: filePath.split(/[\\/]/).pop() ?? filePath,
    fileSize: 0,
    extension: filePath.split(".").pop() ?? "",
    addedAt: Date.now(),
    exists: true,
    isManagedCopy: false,
  }),
  removeShelfItem: async (_id: string): Promise<void> => undefined,
  openShelfFile: async (_id: string): Promise<void> => undefined,
  showShelfFileInFolder: async (_id: string): Promise<void> => undefined,
  pickShelfFile: async (): Promise<string | null> => null,
  clearShelf: async (): Promise<void> => undefined,
  clearLocalData: async (): Promise<void> => undefined,
  activateAppSource: async (
    _appId: string,
    _appName: string,
    _titleHint?: string | null,
  ): Promise<boolean> => true,
  revealDiagnosticLog: async (): Promise<string> => "",
};

export function setCompactWidthHint(_width: number | null): void {
  // The native hit-region sizing call has no effect in the browser preview.
}
