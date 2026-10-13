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
});

export type AppConfig = z.infer<typeof AppConfigSchema>;
