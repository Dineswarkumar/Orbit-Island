import { z } from "zod";

export const AppConfigSchema = z.object({
  theme: z.enum([
    "obsidian",
    "dark",
    "dynamic",
    "light",
    "black",
    "espresso",
    "evergreen",
    "nord",
    "custom"
  ]).default("obsidian"),
  position: z.enum(["top-center", "top-left", "top-right"]).default("top-center"),
  density: z.enum(["compact", "balanced", "detailed"]).default("balanced"),
  timeFormat: z.enum(["12h", "24h"]).default("12h"),
  alwaysOnTop: z.boolean().default(true),
  launchAtStartup: z.boolean().default(false),
  showHardwareMonitor: z.boolean().default(true),
  transparencyMode: z.enum(["glass", "frosted", "smoked", "solid"]).default("glass"),
  notificationsEnabled: z.boolean().default(true),
  privacyMode: z.boolean().default(false),
  notificationPreviewDurationSec: z.number().min(2).max(30).default(4),
  showPrivacyIndicators: z.boolean().default(true),
  blockedApps: z.array(z.string()).default([]),
  surfaceOpacity: z.union([z.literal(20), z.literal(50), z.literal(80), z.literal(100)]).default(80),
});

export type AppConfig = z.infer<typeof AppConfigSchema>;
