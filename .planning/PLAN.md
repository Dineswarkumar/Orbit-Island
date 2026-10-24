# ORBIT — Execution Plan for Phase 12: The Elite Theme Engine & Visual Showcase Overhaul

> **Instructions for 3.8 Flash / Builder Agent**: Execute Tasks 1 through 4 in exact sequential order. Do NOT skip any file or CSS selector. After completing all code edits, run `npm run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, and `cargo build --release --manifest-path src-tauri/Cargo.toml`, then relaunch `src-tauri\target\release\orbit.exe`.

---

## Task 1: Complete Theme Token System & 10-Theme Curated Lineup (`src/core/theme.ts` & `src/styles/globals.css`)

### Goal
Replace murky backgrounds and shallow variable swaps with **10 world-class desktop themes** inspired by Apple Dynamic Island, Raycast Theme Studio, Linear, Catppuccin Mocha, and VisionOS. Provide complete token coverage so card backgrounds, borders, text, muted subtitles, and specular rims transform completely with each theme.

### 1.1 Update `src/core/theme.ts`
Expand the `ThemeMeta` interface and replace `THEMES` with the following 10 calibrated palettes:

```typescript
export interface ThemeMeta {
  id: AppConfig["theme"];
  label: string;
  tagline: string;
  accent: string;           // Primary vibrant action color
  accentSecondary: string;  // Supporting harmony color
  color: string;            // Compatibility alias for accent
  borderGlow: string;       // Outer orbital aura
  coreGlow: string;         // Volumetric center radial depth
  rimHighlight: string;     // Top specular catch-light
  surfaceBase: string;      // RGB triplet for base canvas (e.g., "0, 0, 0")
  surfaceElevated: string;  // Card background tint
  surfaceCardBorder: string;// Card border tint
  text: string;             // High-contrast primary text
  muted: string;            // Secondary / metadata text
  badgeBg: string;          // Pill / tag background
  swatches: string[];       // 3 hex colors for visual preview in Settings
}
```

#### The 10 Curated Themes:
1. **`cupertino_black` (Apple Dynamic Island - Pure OLED)**:
   - `id`: `"black"`
   - `surfaceBase`: `"0, 0, 0"`
   - `accent`: `"#0a84ff"` (Apple System Blue)
   - `accentSecondary`: `"#30d158"` (Apple System Green)
   - `text`: `"#f5f5f7"`, `muted`: `"#86868b"`
   - `rimHighlight`: `"rgba(255, 255, 255, 0.28)"`
   - `swatches`: `["#000000", "#0a84ff", "#30d158"]`

2. **`obsidian` (Midnight Sapphire)**:
   - `id`: `"obsidian"`
   - `surfaceBase`: `"8, 11, 20"`
   - `accent`: `"#38bdf8"` (Electric Cyan)
   - `accentSecondary`: `"#2563eb"` (Royal Cobalt)
   - `text`: `"#f8fafc"`, `muted`: `"#94a3b8"`
   - `rimHighlight`: `"rgba(255, 255, 255, 0.28)"`
   - `swatches`: `["#080b14", "#38bdf8", "#2563eb"]`

3. **`linear_indigo` (Linear & Raycast Studio)**:
   - `id`: `"dynamic"`
   - `surfaceBase`: `"11, 13, 20"`
   - `accent`: `"#6366f1"` (Linear Indigo)
   - `accentSecondary`: `"#a855f7"` (Electric Violet)
   - `text`: `"#f8fafc"`, `muted`: `"#a5b4fc"`
   - `rimHighlight`: `"rgba(129, 140, 248, 0.35)"`
   - `swatches`: `["#0b0d14", "#6366f1", "#a855f7"]`

4. **`vision_spatial` (VisionOS Spatial Glass)**:
   - `id`: `"dark"`
   - `surfaceBase`: `"15, 23, 42"`
   - `accent`: `"#2dd4bf"` (Spatial Teal)
   - `accentSecondary`: `"#38bdf8"` (Glacier Sky)
   - `text`: `"#ffffff"`, `muted`: `"#94a3b8"`
   - `rimHighlight`: `"rgba(255, 255, 255, 0.45)"`
   - `swatches`: `["#0f172a", "#2dd4bf", "#38bdf8"]`

5. **`catppuccin_mocha` (Catppuccin Mocha - Developer Favorite)**:
   - `id`: `"catppuccin"` (or update config validation)
   - `surfaceBase`: `"24, 24, 37"` (#181825 base)
   - `accent`: `"#cba6f7"` (Mocha Mauve)
   - `accentSecondary`: `"#fab387"` (Mocha Peach)
   - `text`: `"#cdd6f4"`, `muted`: `"#a6adc8"`
   - `rimHighlight`: `"rgba(203, 166, 247, 0.38)"`
   - `swatches`: `["#181825", "#cba6f7", "#fab387"]`

6. **`cyber_neon` (Cyberpunk Neo-Tokyo)**:
   - `id`: `"cyber"`
   - `surfaceBase`: `"3, 3, 6"`
   - `accent`: `"#00f0ff"` (Cyber Cyan)
   - `accentSecondary`: `"#ff007f"` (Neon Magenta)
   - `text`: `"#ffffff"`, `muted`: `"#67e8f9"`
   - `rimHighlight`: `"rgba(0, 240, 255, 0.45)"`
   - `swatches`: `["#030306", "#00f0ff", "#ff007f"]`

7. **`sunset_crimson` (Warm Sunset Velvet & Amber)**:
   - `id`: `"espresso"`
   - `surfaceBase`: `"18, 7, 12"` (Deep velvety plum, not muddy brown!)
   - `accent`: `"#f43f5e"` (Sunset Rose)
   - `accentSecondary`: `"#f59e0b"` (Radiant Amber Gold)
   - `text`: `"#fff1f2"`, `muted`: `"#fda4af"`
   - `rimHighlight`: `"rgba(251, 113, 133, 0.36)"`
   - `swatches`: `["#12070c", "#f43f5e", "#f59e0b"]`

8. **`emerald_aurora` (Nordic Botanical Emerald)**:
   - `id`: `"evergreen"`
   - `surfaceBase`: `"6, 18, 14"` (Deep abyssal pine, not murky pond!)
   - `accent`: `"#10b981"` (Vivid Emerald)
   - `accentSecondary`: `"#34d399"` (Fresh Mint)
   - `text`: `"#ecfdf5"`, `muted`: `"#6ee7b7"`
   - `rimHighlight`: `"rgba(52, 211, 153, 0.38)"`
   - `swatches`: `["#06120e", "#10b981", "#34d399"]`

9. **`nord_ocean` (Abyssal Ocean & Solar Gold)**:
   - `id`: `"nord"`
   - `surfaceBase`: `"10, 18, 32"`
   - `accent`: `"#fbbf24"` (Solar Gold)
   - `accentSecondary`: `"#38bdf8"` (Glacier Sky)
   - `text`: `"#f0f9ff"`, `muted`: `"#93c5fd"`
   - `rimHighlight`: `"rgba(251, 191, 36, 0.35)"`
   - `swatches`: `["#0a1220", "#fbbf24", "#38bdf8"]`

10. **`ceramic_pearl` (Apple Ceramic Alabaster - Light Theme)**:
    - `id`: `"light"`
    - `surfaceBase`: `"241, 245, 249"`
    - `accent`: `"#0066cc"` (Apple Royal Blue)
    - `accentSecondary`: `"#4f46e5"` (Deep Indigo)
    - `text`: `"#0f172a"`, `muted`: `"#64748b"`
    - `rimHighlight`: `"rgba(255, 255, 255, 0.95)"`
    - `swatches`: `["#f1f5f9", "#0066cc", "#ffffff"]`

11. **`system` (System Auto - Follow Windows)**:
    - Automatically switches to `light` when Windows is in Light Mode, or `black` when in Dark Mode.

### 1.2 Update `src/styles/globals.css`
For each theme class (`.theme-black`, `.theme-obsidian`, `.theme-dynamic`, `.theme-dark`, `.theme-catppuccin`, `.theme-cyber`, `.theme-espresso`, `.theme-evergreen`, `.theme-nord`, `.theme-light`), declare full CSS variables:
- `--theme-accent`
- `--theme-accent-secondary`
- `--core-glow`
- `--rim-highlight`
- `--island-panel`
- `--island-pill`
- `--island-card-bg`
- `--island-card-border`
- `--island-badge-bg`
- `--island-text`
- `--island-muted`
- `--island-border`

---

## Task 2: Deep Theme Token Binding in Island Components (`HomeSection.tsx`, `MultiTaskSection.tsx`, `CalendarSection.tsx`, `CompactCapsule.tsx`)

### Goal
Eliminate hardcoded generic Tailwind classes (`bg-white/10`, `border-white/10`, `text-white/40`) so all island surfaces, buttons, wells, headers, and badges dynamically adopt the active theme's colors, borders, and text tones.

### 2.1 Update `HomeSection.tsx`
- Replace hardcoded `bg-white/10` and `border-white/10` on top pills, quick action cards, and timer presets with `var(--island-card-bg)` and `var(--island-card-border)`.
- Use `style={{ color: "var(--island-text)" }}` and `style={{ color: "var(--island-muted)" }}` or dedicated utility classes so titles and subtitles match the theme.
- Ensure the Media Player progress bar, frequency bars, and volume slider use `activeThemeMeta.accent` and `activeThemeMeta.accentSecondary`.

### 2.2 Update `MultiTaskSection.tsx`
- Update the 4 Bento slots to use `style={{ background: "var(--island-card-bg)", borderColor: "var(--island-card-border)" }}`.
- Update icons and badges to use `activeThemeMeta.accent`.

### 2.3 Update `CalendarSection.tsx`
- Ensure the month navigation buttons, category pills, and add-task inputs use `var(--island-card-bg)` and `var(--island-card-border)`.
- Keep the category colors for exams, holidays, and events crisp and distinct against all theme backgrounds.

---

## Task 3: Interactive Visual Theme Showcase Gallery in Settings (`src/components/SettingsApp.tsx`)

### Goal
Replace the plain `<select>` theme dropdown with a **Visual Bento Theme Gallery**:
- A responsive 2-column or 3-column grid of interactive Theme Cards.
- Each Theme Card features:
  - Mini **Dynamic Island Preview Pill** showing the theme's background, specular rim highlight, and live accent glow.
  - Theme Label and descriptive Tagline.
  - 3 Circular Swatch Dots displaying the theme's canvas, primary accent, and secondary accent.
  - Active Radio Checkmark with glowing ring when selected.
  - Instant live IPC update to the island overlay on click.

---

## Task 4: Configuration Validation & Backward Compatibility (`src-tauri/src/services/config.rs` & `src/core/types/config.ts`)

### Goal
Ensure any newly introduced theme IDs (e.g. `"catppuccin"`, `"cyber"`) are fully validated in Rust (`services/config.rs`) and TypeScript (`core/types/config.ts`), with seamless fallback to `"obsidian"` for invalid/legacy IDs so existing user config files never crash.

### Verification Steps
1. Run `npm run build` — must pass with 0 TypeScript/Vite errors.
2. Run `cargo test --manifest-path src-tauri/Cargo.toml` — all 34+ Rust backend unit tests must pass.
3. Run `cargo build --release --manifest-path src-tauri/Cargo.toml` — must compile clean release binary.
4. Launch `src-tauri\target\release\orbit.exe` and confirm live PID.
5. Open Settings (`Alt+S` or right-click System Tray -> Settings) and verify the Theme Showcase Gallery displays all themes with live previews and immediate island transformation upon selection.
