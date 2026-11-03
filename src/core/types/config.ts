export type AppTheme =
  | "system"
  | "obsidian"
  | "dark"
  | "dynamic"
  | "light"
  | "black"
  | "espresso"
  | "evergreen"
  | "nord"
  | "catppuccin"
  | "cyber"
  | "custom";

export type AppPosition = "top-center" | "top-left" | "top-right";
export type AppDensity = "compact" | "balanced" | "detailed";
export type AppTimeFormat = "12h" | "24h";
export type TransparencyMode = "glass" | "frosted" | "smoked" | "solid";
export type SurfaceOpacity = 20 | 50 | 80 | 100;
export type WeatherUnit = "C" | "F";
export type ExpandTrigger = "hover" | "click";
export type ShelfStorageMode = "reference" | "copy";
export type FontFamily = "inter" | "bold" | "claude" | "mono" | "rounded" | "cyber";
export type CompactRightSlot =
  | "weather"
  | "orbit_logo"
  | "windows_logo"
  | "apple_logo"
  | "linux_logo"
  | "custom_text"
  | "custom_image"
  | "smart";
export type CustomRightTextStyle = "neon_glow" | "cyber_badge" | "aurora_pill" | "liquid_gold";
export type NetworkActivityMode = "off" | "speed_only" | "full";
export type MediaBackgroundMode = "cover" | "theme" | "solid";
export type MediaVisualizerMode = "spectrum" | "beats" | "off";

export interface AppConfig {
  theme: AppTheme;
  position: AppPosition;
  density: AppDensity;
  timeFormat: AppTimeFormat;
  alwaysOnTop: boolean;
  showSystemTrayIcon: boolean;
  launchAtStartup: boolean;
  showHardwareMonitor: boolean;
  transparencyMode: TransparencyMode;
  notificationsEnabled: boolean;
  privacyMode: boolean;
  notificationPreviewDurationSec: number;
  showPrivacyIndicators: boolean;
  blockedApps: string[];
  surfaceOpacity: SurfaceOpacity;
  showWeather: boolean;
  weatherUnit: WeatherUnit;
  adaptiveAlbumAccent: boolean;
  expandTrigger: ExpandTrigger;
  pauseInFullscreen: boolean;
  shelfStorageMode: ShelfStorageMode;
  fontFamily: FontFamily;
  onboardingCompleted: boolean;
  compactRightSlot: CompactRightSlot;
  customRightText: string;
  customRightTextStyle: CustomRightTextStyle;
  customRightImageBase64: string;
  hideFromScreenCapture: boolean;
  shortcutsEnabled: boolean;
  autoHideInMeetings: boolean;
  networkActivityMode: NetworkActivityMode;
  onlineWeatherEnabled: boolean;
  mediaBackgroundMode: MediaBackgroundMode;
  mediaVisualizerMode: MediaVisualizerMode;
  mediaShowCoverArt: boolean;
  mediaCompactMode: boolean;
}

/** Normalize saved material + opacity combinations while retaining the opaque low-cost surface. */
export function normalizeGlassPreset(config: AppConfig): AppConfig {
  if (config.transparencyMode === "smoked") {
    return { ...config, transparencyMode: "smoked", surfaceOpacity: 50 };
  }
  if (config.transparencyMode === "frosted") {
    return { ...config, transparencyMode: "frosted", surfaceOpacity: 80 };
  }
  if (config.transparencyMode === "solid") {
    return { ...config, transparencyMode: "solid", surfaceOpacity: 100 };
  }

  if (config.surfaceOpacity <= 35) {
    return { ...config, transparencyMode: "glass", surfaceOpacity: 20 };
  }
  if (config.surfaceOpacity <= 65) {
    return { ...config, transparencyMode: "smoked", surfaceOpacity: 50 };
  }
  return { ...config, transparencyMode: "frosted", surfaceOpacity: 80 };
}

export const DEFAULT_APP_CONFIG: Readonly<AppConfig> = {
  theme: "obsidian",
  position: "top-center",
  density: "balanced",
  timeFormat: "12h",
  alwaysOnTop: true,
  showSystemTrayIcon: true,
  launchAtStartup: false,
  showHardwareMonitor: true,
  transparencyMode: "glass",
  notificationsEnabled: true,
  privacyMode: false,
  notificationPreviewDurationSec: 4,
  showPrivacyIndicators: true,
  blockedApps: [],
  surfaceOpacity: 20,
  showWeather: false,
  weatherUnit: "C",
  adaptiveAlbumAccent: true,
  expandTrigger: "hover",
  pauseInFullscreen: true,
  shelfStorageMode: "reference",
  fontFamily: "inter",
  onboardingCompleted: false,
  compactRightSlot: "weather",
  customRightText: "ORBIT",
  customRightTextStyle: "neon_glow",
  customRightImageBase64: "",
  hideFromScreenCapture: false,
  shortcutsEnabled: true,
  autoHideInMeetings: true,
  networkActivityMode: "speed_only",
  onlineWeatherEnabled: false,
  mediaBackgroundMode: "cover",
  mediaVisualizerMode: "spectrum",
  mediaShowCoverArt: true,
  mediaCompactMode: false,
};

export function createDefaultAppConfig(): AppConfig {
  return {
    ...DEFAULT_APP_CONFIG,
    blockedApps: [...DEFAULT_APP_CONFIG.blockedApps],
  };
}

export function mergeWithDefaultConfig(partial?: unknown): AppConfig {
  const base = createDefaultAppConfig();
  if (!partial || typeof partial !== "object") {
    return base;
  }
  const obj = partial as Record<string, unknown>;

  return {
    theme: typeof obj.theme === "string" ? (obj.theme as AppTheme) : base.theme,
    position: typeof obj.position === "string" ? (obj.position as AppPosition) : base.position,
    density: typeof obj.density === "string" ? (obj.density as AppDensity) : base.density,
    timeFormat: typeof obj.timeFormat === "string" ? (obj.timeFormat as AppTimeFormat) : base.timeFormat,
    alwaysOnTop: typeof obj.alwaysOnTop === "boolean" ? obj.alwaysOnTop : base.alwaysOnTop,
    showSystemTrayIcon: typeof obj.showSystemTrayIcon === "boolean" ? obj.showSystemTrayIcon : base.showSystemTrayIcon,
    launchAtStartup: typeof obj.launchAtStartup === "boolean" ? obj.launchAtStartup : base.launchAtStartup,
    showHardwareMonitor: typeof obj.showHardwareMonitor === "boolean" ? obj.showHardwareMonitor : base.showHardwareMonitor,
    transparencyMode: typeof obj.transparencyMode === "string" ? (obj.transparencyMode as TransparencyMode) : base.transparencyMode,
    notificationsEnabled: typeof obj.notificationsEnabled === "boolean" ? obj.notificationsEnabled : base.notificationsEnabled,
    privacyMode: typeof obj.privacyMode === "boolean" ? obj.privacyMode : base.privacyMode,
    notificationPreviewDurationSec: typeof obj.notificationPreviewDurationSec === "number" ? Math.max(2, Math.min(30, obj.notificationPreviewDurationSec)) : base.notificationPreviewDurationSec,
    showPrivacyIndicators: typeof obj.showPrivacyIndicators === "boolean" ? obj.showPrivacyIndicators : base.showPrivacyIndicators,
    blockedApps: Array.isArray(obj.blockedApps) ? obj.blockedApps.filter((x): x is string => typeof x === "string") : base.blockedApps,
    surfaceOpacity: (obj.surfaceOpacity === 20 || obj.surfaceOpacity === 50 || obj.surfaceOpacity === 80 || obj.surfaceOpacity === 100) ? obj.surfaceOpacity : base.surfaceOpacity,
    showWeather: typeof obj.showWeather === "boolean" ? obj.showWeather : base.showWeather,
    weatherUnit: typeof obj.weatherUnit === "string" ? (obj.weatherUnit as WeatherUnit) : base.weatherUnit,
    adaptiveAlbumAccent: typeof obj.adaptiveAlbumAccent === "boolean" ? obj.adaptiveAlbumAccent : base.adaptiveAlbumAccent,
    expandTrigger: typeof obj.expandTrigger === "string" ? (obj.expandTrigger as ExpandTrigger) : base.expandTrigger,
    pauseInFullscreen: typeof obj.pauseInFullscreen === "boolean" ? obj.pauseInFullscreen : base.pauseInFullscreen,
    shelfStorageMode: typeof obj.shelfStorageMode === "string" ? (obj.shelfStorageMode as ShelfStorageMode) : base.shelfStorageMode,
    fontFamily: typeof obj.fontFamily === "string" ? (obj.fontFamily as FontFamily) : base.fontFamily,
    onboardingCompleted: typeof obj.onboardingCompleted === "boolean" ? obj.onboardingCompleted : base.onboardingCompleted,
    compactRightSlot: typeof obj.compactRightSlot === "string" ? (obj.compactRightSlot as CompactRightSlot) : base.compactRightSlot,
    customRightText: typeof obj.customRightText === "string" ? obj.customRightText.slice(0, 5) : base.customRightText,
    customRightTextStyle: typeof obj.customRightTextStyle === "string" ? (obj.customRightTextStyle as CustomRightTextStyle) : base.customRightTextStyle,
    customRightImageBase64: typeof obj.customRightImageBase64 === "string" ? obj.customRightImageBase64 : base.customRightImageBase64,
    hideFromScreenCapture: typeof obj.hideFromScreenCapture === "boolean" ? obj.hideFromScreenCapture : base.hideFromScreenCapture,
    shortcutsEnabled: typeof obj.shortcutsEnabled === "boolean" ? obj.shortcutsEnabled : base.shortcutsEnabled,
    autoHideInMeetings: typeof obj.autoHideInMeetings === "boolean" ? obj.autoHideInMeetings : base.autoHideInMeetings,
    networkActivityMode: typeof obj.networkActivityMode === "string" ? (obj.networkActivityMode as NetworkActivityMode) : base.networkActivityMode,
    onlineWeatherEnabled: typeof obj.onlineWeatherEnabled === "boolean" ? obj.onlineWeatherEnabled : base.onlineWeatherEnabled,
    mediaBackgroundMode: obj.mediaBackgroundMode === "cover" || obj.mediaBackgroundMode === "theme" || obj.mediaBackgroundMode === "solid" ? obj.mediaBackgroundMode : base.mediaBackgroundMode,
    mediaVisualizerMode: obj.mediaVisualizerMode === "spectrum" || obj.mediaVisualizerMode === "beats" || obj.mediaVisualizerMode === "off" ? obj.mediaVisualizerMode : base.mediaVisualizerMode,
    mediaShowCoverArt: typeof obj.mediaShowCoverArt === "boolean" ? obj.mediaShowCoverArt : base.mediaShowCoverArt,
    mediaCompactMode: typeof obj.mediaCompactMode === "boolean" ? obj.mediaCompactMode : base.mediaCompactMode,
  };
}

export const AppConfigSchema = {
  parse: (input: unknown): AppConfig => mergeWithDefaultConfig(input),
};
