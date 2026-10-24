import { AppConfig } from "./types/config";

export interface ThemeMeta {
  id: AppConfig["theme"];
  label: string;
  description: string;
  accent: string;
  accentSecondary: string;
  color: string; // alias to accent for compatibility
  borderGlow: string;
  coreGlow: string; // 3D volumetric center radial aura (like reference-idle-clock.png)
  rimHighlight: string; // 3D top specular catch-light
  surfaceBase: string; // RGB triplet, e.g. "7, 11, 20"
  surfaceElevated: string; // Inner 3D card surface
  text: string;
  muted: string;
}

export const THEMES: ThemeMeta[] = [
  {
    id: "system",
    label: "System Auto (Follow Windows)",
    description: "Automatically matches Windows light and dark appearance",
    accent: "#38bdf8",
    accentSecondary: "#2563eb",
    color: "#38bdf8",
    borderGlow: "rgba(56, 189, 248, 0.32)",
    coreGlow: "rgba(56, 189, 248, 0.18)",
    rimHighlight: "rgba(255, 255, 255, 0.28)",
    surfaceBase: "8, 11, 20",
    surfaceElevated: "rgba(56, 189, 248, 0.08)",
    text: "#f8fafc",
    muted: "#94a3b8",
  },
  {
    id: "obsidian",
    label: "Midnight Sapphire",
    description: "Deep space-obsidian lens with electric iOS sapphire depth and cyan glows",
    accent: "#38bdf8",
    accentSecondary: "#2563eb",
    color: "#38bdf8",
    borderGlow: "rgba(56, 189, 248, 0.32)",
    coreGlow: "rgba(56, 189, 248, 0.18)",
    rimHighlight: "rgba(255, 255, 255, 0.28)",
    surfaceBase: "8, 11, 20",
    surfaceElevated: "rgba(56, 189, 248, 0.08)",
    text: "#f8fafc",
    muted: "#94a3b8",
  },
  {
    id: "dynamic",
    label: "Nebula Violet",
    description: "Linear-inspired cosmic indigo-violet workspace with vibrant electric magenta rim",
    accent: "#c084fc",
    accentSecondary: "#7c3aed",
    color: "#c084fc",
    borderGlow: "rgba(192, 132, 252, 0.35)",
    coreGlow: "rgba(192, 132, 252, 0.18)",
    rimHighlight: "rgba(255, 255, 255, 0.28)",
    surfaceBase: "18, 13, 34",
    surfaceElevated: "rgba(192, 132, 252, 0.10)",
    text: "#faf5ff",
    muted: "#c4b5fd",
  },
  {
    id: "dark",
    label: "VisionOS Spatial Titanium",
    description: "Anodized spatial graphite-slate with bright titanium specular rim and teal accents",
    accent: "#2dd4bf",
    accentSecondary: "#0ea5e9",
    color: "#2dd4bf",
    borderGlow: "rgba(226, 232, 240, 0.32)",
    coreGlow: "rgba(45, 212, 191, 0.16)",
    rimHighlight: "rgba(255, 255, 255, 0.42)",
    surfaceBase: "26, 31, 44",
    surfaceElevated: "rgba(255, 255, 255, 0.11)",
    text: "#f8fafc",
    muted: "#cbd5e1",
  },
  {
    id: "black",
    label: "OLED Cyber Pitch",
    description: "True pitch-black OLED void with high-voltage neon emerald-cyan accents",
    accent: "#00f5d4",
    accentSecondary: "#10b981",
    color: "#00f5d4",
    borderGlow: "rgba(0, 245, 212, 0.34)",
    coreGlow: "rgba(0, 245, 212, 0.15)",
    rimHighlight: "rgba(255, 255, 255, 0.30)",
    surfaceBase: "2, 3, 5",
    surfaceElevated: "rgba(0, 245, 212, 0.06)",
    text: "#ffffff",
    muted: "#a1a1aa",
  },
  {
    id: "espresso",
    label: "Sunset Rose & Amber",
    description: "Rich warm crimson-mahogany glass with warm sunset rose-gold and amber highlights",
    accent: "#fb7185",
    accentSecondary: "#f59e0b",
    color: "#fb7185",
    borderGlow: "rgba(251, 113, 133, 0.34)",
    coreGlow: "rgba(251, 113, 133, 0.18)",
    rimHighlight: "rgba(255, 255, 255, 0.26)",
    surfaceBase: "30, 14, 20",
    surfaceElevated: "rgba(251, 113, 133, 0.10)",
    text: "#fff1f2",
    muted: "#fda4af",
  },
  {
    id: "evergreen",
    label: "Aurora Botanical Emerald",
    description: "Deep lush aurora forest glass with luminous mint-emerald highlights",
    accent: "#34d399",
    accentSecondary: "#059669",
    color: "#34d399",
    borderGlow: "rgba(52, 211, 153, 0.34)",
    coreGlow: "rgba(52, 211, 153, 0.18)",
    rimHighlight: "rgba(255, 255, 255, 0.26)",
    surfaceBase: "8, 24, 18",
    surfaceElevated: "rgba(52, 211, 153, 0.10)",
    text: "#ecfdf5",
    muted: "#6ee7b7",
  },
  {
    id: "nord",
    label: "Arc Ocean & Solar Gold",
    description: "Deep abyssal ocean navy with solar gold and glacier sky dual accents",
    accent: "#fbbf24",
    accentSecondary: "#38bdf8",
    color: "#fbbf24",
    borderGlow: "rgba(251, 191, 36, 0.35)",
    coreGlow: "rgba(251, 191, 36, 0.16)",
    rimHighlight: "rgba(255, 255, 255, 0.28)",
    surfaceBase: "11, 22, 40",
    surfaceElevated: "rgba(56, 189, 248, 0.10)",
    text: "#f0f9ff",
    muted: "#93c5fd",
  },
  {
    id: "light",
    label: "Apple Ceramic Pearl",
    description: "Bright frosted alabaster-pearl surface with royal cobalt-indigo accents and ink text",
    accent: "#2563eb",
    accentSecondary: "#4f46e5",
    color: "#2563eb",
    borderGlow: "rgba(37, 99, 235, 0.32)",
    coreGlow: "rgba(37, 99, 235, 0.12)",
    rimHighlight: "rgba(255, 255, 255, 0.75)",
    surfaceBase: "241, 245, 252",
    surfaceElevated: "rgba(255, 255, 255, 0.82)",
    text: "#0f172a",
    muted: "#475569",
  },
];

export const getThemeMeta = (id: string): ThemeMeta => {
  if (id === "system") {
    const isLight =
      typeof window !== "undefined" &&
      window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: light)").matches;
    const targetId = isLight ? "light" : "obsidian";
    return THEMES.find((t) => t.id === targetId) || THEMES[1];
  }
  return THEMES.find((t) => t.id === id) || THEMES[1];
};
