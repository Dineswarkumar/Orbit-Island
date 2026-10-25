# ORBIT Landing Site — State

## Decisions (locked)
- Site is an independent npm project in `site/` and imports live island components from `src/`.
- React 18, Vite 6, Tailwind 3, and React Three Fiber 8 remain on the planned compatible version lines.
- The app's theme variables are shared by importing `src/styles/globals.css`; site page defaults override transparent-window styles.

## Progress
| Card | Status | Commit | Notes |
|---|---|---|---|
| S0.1 Scaffold | ☑ | `chore(site): scaffold landing site vite app` | Standalone Vite + React + Tailwind project; install and placeholder build pass |
| S0.2 Mock IPC | ☑ | `feat(site): mock tauri ipc and render live CompactCapsule` | All 39 IPC methods typed; demo fixtures and live `CompactCapsule`; site build passes |
| S0.3 Theme CSS | ☐ | | App theme variables with site overrides |
| S1.1 Lenis+GSAP | ☐ | | |
| S1.2 Stage | ☐ | | |
| S2.1–S2.8 Scenes | ☐ | | replicas used: none |
| S3.1 Model | ☐ | | screen mesh name: ___ · model source/license: ___ |
| S3.2 Dive-in | ☐ | | |
| S3.3 Outro | ☐ | | |
| S4 Music | ☐ | | |
| S5.1–S5.5 Polish | ☐ | | Lighthouse mobile: ___ |
| S6 Deploy | ☐ | | URL: ___ |
