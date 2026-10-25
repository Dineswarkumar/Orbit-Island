import type { AppConfig } from "@app/core/types/config";
import type { HardwareMetrics, MemoryCleanResult } from "@app/core/types/hardware";
import type { AudioSpectrumState, MediaSessionSnapshot, MediaSourceInfo } from "@app/core/types/media";
import type {
  NotificationItem,
  NotificationSnapshot,
  PrivacyState,
} from "@app/core/types/notifications";
import type { ReminderItem } from "@app/core/types/reminders";
import type { ShelfItem } from "@app/core/types/shelf";
import type { MemoryReport } from "@app/core/types/telemetry";
import { AppConfigSchema } from "@app/core/types/config";

export const appConfigFixture: AppConfig = AppConfigSchema.parse({
  theme: "obsidian",
  position: "top-center",
  density: "balanced",
  timeFormat: "12h",
  showWeather: true,
  weatherUnit: "C",
  transparencyMode: "glass",
  surfaceOpacity: 80,
  showPrivacyIndicators: true,
  showHardwareMonitor: true,
  notificationsEnabled: true,
});

export const mediaSessionFixture: MediaSessionSnapshot = {
  source_id: "orbit-demo-player",
  source_title: "ORBIT Demo Player",
  playback_status: "playing",
  track: {
    title: "Afterglow Drive",
    artist: "ORBIT Demo",
    album: "Signals in the Dark",
    thumbnail_base64: "/demo-cover.svg",
  },
  timeline: {
    position_secs: 92,
    duration_secs: 218,
  },
  controls: {
    can_play: true,
    can_pause: true,
    can_skip_next: true,
    can_skip_previous: true,
    can_seek: true,
  },
  is_pinned: false,
};

export const mediaSourcesFixture: MediaSourceInfo[] = [
  {
    source_id: "orbit-demo-player",
    source_title: "ORBIT Demo Player",
    is_playing: true,
    is_pinned: false,
  },
  {
    source_id: "chrome.exe",
    source_title: "Google Chrome",
    is_playing: false,
    is_pinned: false,
  },
];

export const audioSpectrumFixture: AudioSpectrumState = {
  peak: 0.56,
  compactBars: [0.72, 0.48, 0.81, 0.59],
  waveBars: [
    0.24, 0.38, 0.64, 0.82, 0.56, 0.34, 0.28, 0.46, 0.71, 0.59,
    0.43, 0.68, 0.86, 0.52, 0.31, 0.48, 0.74, 0.91, 0.62, 0.37,
  ],
};

export const notificationFixtures: NotificationItem[] = [
  {
    id: 301,
    appName: "WhatsApp",
    appId: "WhatsApp.exe",
    title: "Incoming voice call",
    body: "Maya Chen is calling",
    timestampMs: Date.now() - 12_000,
    isCritical: true,
    count: 1,
    actions: [
      { id: "answer", label: "Answer", isSupported: true },
      { id: "decline", label: "Decline", isSupported: true },
    ],
  },
  {
    id: 302,
    appName: "Mail",
    appId: "Microsoft.OutlookForWindows",
    title: "Design review moved",
    body: "The review now starts at 2:30 PM.",
    timestampMs: Date.now() - 46_000,
    isCritical: false,
    count: 1,
    actions: [],
  },
  {
    id: 303,
    appName: "Telegram",
    appId: "Telegram.exe",
    title: "Ari Patel",
    body: "The new build is ready to try.",
    timestampMs: Date.now() - 94_000,
    isCritical: false,
    count: 1,
    actions: [],
  },
];

export const notificationSnapshotFixture: NotificationSnapshot = {
  accessStatus: "allowed",
  activeNotification: notificationFixtures[0],
  notifications: notificationFixtures,
  overflowCount: 0,
  totalCount: notificationFixtures.length,
};

export const privacyFixture: PrivacyState = {
  cameraActive: false,
  cameraApp: null,
  microphoneActive: true,
  microphoneApp: "Microsoft Teams",
  isReliable: true,
};

const now = Date.now();
export const reminderFixtures: ReminderItem[] = [
  {
    id: "exam-physics",
    title: "Physics final exam",
    dueTimestampMs: now + 2 * 24 * 60 * 60 * 1000,
    isCompleted: false,
    source: "academic",
    category: "exam",
    createdAt: now - 86_400_000,
  },
  {
    id: "deadline-prototype",
    title: "Submit island prototype",
    dueTimestampMs: now + 4 * 60 * 60 * 1000,
    isCompleted: false,
    source: "local",
    category: "deadline",
    createdAt: now - 43_200_000,
  },
  {
    id: "fest-open-day",
    title: "Spring campus fest",
    dueTimestampMs: now + 7 * 24 * 60 * 60 * 1000,
    isCompleted: false,
    source: "academic",
    category: "fest",
    createdAt: now - 172_800_000,
  },
];

export const hardwareMetricsFixture: HardwareMetrics = {
  cpuUsage: 18.4,
  cpuCores: 8,
  ramUsedBytes: 8_321_200_128,
  ramTotalBytes: 17_179_869_184,
  ramPercentage: 48.4,
  diskFreeBytes: 312_056_217_600,
  diskTotalBytes: 1_000_204_886_016,
  diskPercentage: 68.8,
  batteryPercentage: 82,
  isCharging: false,
  wifiConnected: true,
  bluetoothEnabled: true,
  hotspotActive: false,
  networkUploadSpeedBps: 148_000,
  networkDownloadSpeedBps: 1_820_000,
  gpuName: "Demo Graphics Adapter",
  gpuUsage: 23,
  screenRecordingActive: false,
  activeDownloadCount: 1,
  lastCompletedDownload: "orbit-setup.exe",
};

export const shelfFixtures: ShelfItem[] = [
  {
    id: "shelf-design",
    filePath: "C:/Users/Demo/Documents/Island-Design.sketch",
    displayName: "Island-Design.sketch",
    fileSize: 4_718_592,
    extension: "sketch",
    addedAt: now - 3_600_000,
    exists: true,
    isManagedCopy: false,
  },
  {
    id: "shelf-notes",
    filePath: "C:/Users/Demo/Documents/Launch-Notes.pdf",
    displayName: "Launch-Notes.pdf",
    fileSize: 1_245_184,
    extension: "pdf",
    addedAt: now - 7_200_000,
    exists: true,
    isManagedCopy: false,
  },
];

export const memoryReportFixture: MemoryReport = {
  mainProcess: {
    pid: 10420,
    name: "ORBIT.exe",
    workingSetBytes: 104_857_600,
    workingSetMb: 100,
    privateBytes: 125_829_120,
    privateMb: 120,
    privateWorkingSetMb: 96,
  },
  childProcesses: [],
  unmeasuredDescendantCount: 0,
  totalWorkingSetMb: 100,
  totalPrivateMb: 120,
  totalPrivateWorkingSetMb: 96,
  aspirationMb: 150,
  targetMb: 250,
  maximumMb: 400,
  meetsAspiration: true,
  meetsTarget: true,
  meetsMaximum: true,
  measurementMethod: "demo fixture",
};

export const memoryCleanResultFixture: MemoryCleanResult = {
  freedBytes: 12_582_912,
  freedMb: 12,
  ramUsedBytes: hardwareMetricsFixture.ramUsedBytes - 12_582_912,
  ramPercentage: 48.3,
  trimmedProcesses: 1,
};
