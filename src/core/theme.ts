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
    id: "obsidian",
    label: "Obsidian Studio",
    description: "Sculpted abyssal obsidian lens with refined sapphire blue depth",
    accent: "#60a5fa",
    accentSecondary: "#3b82f6",
    color: "#60a5fa",
    borderGlow: "rgba(96, 165, 250, 0.28)",
    coreGlow: "rgba(59, 130, 246, 0.16)",
    rimHighlight: "rgba(255, 255, 255, 0.22)",
    surfaceBase: "10, 12, 18",
    surfaceElevated: "rgba(255, 255, 255, 0.055)",
    text: "#f8fafc",
    muted: "#94a3b8",
  },
  {
    id: "dynamic",
    label: "Linear Indigo",
    description: "Linear-inspired dark workspace with soft indigo aura and solar accents",
    accent: "#818cf8",
    accentSecondary: "#6366f1",
    color: "#818cf8",
    borderGlow: "rgba(129, 140, 248, 0.28)",
    coreGlow: "rgba(99, 102, 241, 0.16)",
    rimHighlight: "rgba(255, 255, 255, 0.22)",
    surfaceBase: "12, 13, 22",
    surfaceElevated: "rgba(129, 140, 248, 0.06)",
    text: "#f8fafc",
    muted: "#94a3b8",
  },
  {
    id: "dark",
    label: "Titanium Slate",
    description: "Anodized gunmetal titanium chassis with silver specular bevels",
    accent: "#cbd5e1",
    accentSecondary: "#94a3b8",
    color: "#cbd5e1",
    borderGlow: "rgba(203, 213, 225, 0.24)",
    coreGlow: "rgba(148, 163, 184, 0.12)",
    rimHighlight: "rgba(255, 255, 255, 0.24)",
    surfaceBase: "16, 18, 23",
    surfaceElevated: "rgba(255, 255, 255, 0.05)",
    text: "#f8fafc",
    muted: "#94a3b8",
  },
  {
    id: "black",
    label: "OLED Pure Black",
    description: "True pitch-black OLED void with pure platinum highlights",
    accent: "#e2e8f0",
    accentSecondary: "#94a3b8",
    color: "#e2e8f0",
    borderGlow: "rgba(226, 232, 240, 0.22)",
    coreGlow: "rgba(148, 163, 184, 0.10)",
    rimHighlight: "rgba(255, 255, 255, 0.26)",
    surfaceBase: "3, 4, 7",
    surfaceElevated: "rgba(255, 255, 255, 0.04)",
    text: "#ffffff",
    muted: "#a1a1aa",
  },
  {
    id: "espresso",
    label: "Warm Bronze",
    description: "Warm smoked bronze glass with champagne amber highlights",
    accent: "#f59e0b",
    accentSecondary: "#d97706",
    color: "#f59e0b",
    borderGlow: "rgba(245, 158, 11, 0.26)",
    coreGlow: "rgba(217, 119, 6, 0.14)",
    rimHighlight: "rgba(255, 255, 255, 0.20)",
    surfaceBase: "17, 14, 12",
    surfaceElevated: "rgba(245, 158, 11, 0.05)",
    text: "#fafaf9",
    muted: "#a8a29e",
  },
  {
    id: "evergreen",
    label: "Sage Emerald",
    description: "Deep forest glass with refined sage emerald highlights",
    accent: "#34d399",
    accentSecondary: "#10b981",
    color: "#34d399",
    borderGlow: "rgba(52, 211, 153, 0.26)",
    coreGlow: "rgba(16, 185, 129, 0.14)",
    rimHighlight: "rgba(255, 255, 255, 0.20)",
    surfaceBase: "10, 16, 14",
    surfaceElevated: "rgba(52, 211, 153, 0.05)",
    text: "#f8fafc",
    muted: "#94a3b8",
  },
  {
    id: "nord",
    label: "Arctic Frost",
    description: "Nordic subzero slate with glacier sky blue highlights",
    accent: "#7dd3fc",
    accentSecondary: "#38bdf8",
    color: "#7dd3fc",
    borderGlow: "rgba(125, 211, 252, 0.26)",
    coreGlow: "rgba(56, 189, 248, 0.15)",
    rimHighlight: "rgba(255, 255, 255, 0.22)",
    surfaceBase: "22, 26, 34",
    surfaceElevated: "rgba(125, 211, 252, 0.055)",
    text: "#f8fafc",
    muted: "#94a3b8",
  },
  {
    id: "light",
    label: "Crystal Frost",
    description: "Deep glacier-platinum slate with bright pearl-silver specular rims and vibrant sky-blue accents",
    accent: "#38bdf8",
    accentSecondary: "#0284c7",
    color: "#38bdf8",
    borderGlow: "rgba(148, 163, 184, 0.42)",
    coreGlow: "rgba(56, 189, 248, 0.16)",
    rimHighlight: "rgba(241, 245, 249, 0.48)",
    surfaceBase: "24, 32, 48",
    surfaceElevated: "rgba(255, 255, 255, 0.06)",
    text: "#f8fafc",
    muted: "#cbd5e1",
  },
];

export const getThemeMeta = (id: string): ThemeMeta => {
  return THEMES.find((t) => t.id === id) || THEMES[0];
};
