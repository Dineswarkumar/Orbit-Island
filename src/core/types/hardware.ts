export interface HardwareMetrics {
  cpuUsage: number; // 0.0 to 100.0%
  cpuCores: number;
  ramUsedBytes: number;
  ramTotalBytes: number;
  ramPercentage: number;
  diskFreeBytes: number;
  diskTotalBytes: number;
  diskPercentage: number;
  batteryPercentage: number | null;
  batteryCharging: boolean | null;
  acPowerConnected: boolean | null;
  wifiConnected: boolean;
  bluetoothEnabled: boolean;
  hotspotActive: boolean;
  networkUploadSpeedBps: number;
  networkDownloadSpeedBps: number;
  gpuName: string;
  gpuUsage: number | null;
  screenRecordingActive: boolean;
  activeDownloadCount: number;
  lastCompletedDownload: string | null;
}

export interface MemoryCleanResult {
  freedBytes: number;
  freedMb: number;
  ramUsedBytes: number;
  ramPercentage: number;
  trimmedProcesses: number;
}

