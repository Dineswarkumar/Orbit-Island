# ORBIT Landing Site — State

## Decisions (locked)
- Site is an independent npm project in `site/` and imports live island components from `src/`.
- React 18, Vite 6, Tailwind 3, and React Three Fiber 8 remain on the planned compatible version lines.
- The app's theme variables are shared by importing `src/styles/globals.css`; site page defaults override transparent-window styles.
- App spectrum uses WASAPI loopback FFT with a meter fallback, so Section 04's real-spectrum claim is backed by the app implementation.
- Corrected copy per `PLAN.md` S2 (40px, no "kernel", no "RAM boost", no unmeasured "<1%").
- Implementation by Gemini Flash, one task card per commit.

## Progress
| Card | Status | Commit | Notes |
|---|---|---|---|
| F1 DSP module | ☑ | `26e04c8` | 20 log-spaced bands; four DSP tests pass |
| F2 Loopback capture | ☑ | `598f819` | Windows v0.62 WASAPI loopback; Windows compile passes |
| F3 Wire + fallback | ☑ | `c7c4436` | 33 ms active / 220 ms idle; meter fallback and device retry wired; manual audio/CPU check pending |
| F4 Docs | ☑ | `c788663` | README and Phase F state updated |
| S0.1 Scaffold | ☑ | `chore(site): scaffold landing site vite app` | Standalone Vite + React + Tailwind project; install and placeholder build pass |
| S0.2 Mock IPC | ☑ | `feat(site): mock tauri ipc and render live CompactCapsule` | All 39 IPC methods typed; demo fixtures and live `CompactCapsule`; site build passes |
| S0.3 Theme CSS | ☑ | `feat(site): share island theme tokens with landing site` | Imports app theme variables and restores a scrollable, selectable web page background |
| S1.1 Lenis+GSAP | ☑ | `feat(site): wire lenis smooth scroll to gsap ticker` | Registers ScrollTrigger/CustomEase once, adds the app's spring curves, and disables Lenis for reduced motion |
| S1.2 Stage | ☑ | `feat(site): add pinned island stage with label-driven morph timeline` | 950vh scroller, sticky viewport stage, eight label-driven shell/content states, responsive scale, and exact Phase S2 copy |
| S2.1–S2.8 Scenes | ☐ | | replicas used: none |
| S3.1 Model | ☐ | | screen mesh name: ___ · model source/license: ___ |
| S3.2 Dive-in | ☐ | | |
| S3.3 Outro | ☐ | | |
| S4 Music | ☐ | | |
| S5.1–S5.5 Polish | ☐ | | Lighthouse mobile: ___ |
| S6 Deploy | ☐ | | URL: ___ |

## Assets received
- [x] **Reference recording** `docs/Recording 2026-10-25 091301.mp4`: 596×416, 54 s, 30 fps, has an audio track. **Motion/state reference only, NOT shippable.** It's a cropped region, too low-res for the laptop screen, and it contains a copyrighted track (Montagem Alquimia) + audio. States shown: home w/ live call+mic pill, hardware, settings, compact media pill, Live Call & Privacy deck, music island, multi-task hub, compact with weather 24°.
- [ ] hero-loop recording (FULL screen ≥1920×1080, no audio, royalty-free or no music) · [ ] music · [ ] notif · [ ] bento
- [ ] laptop.glb (license: ___) · [ ] demo-track.mp3 · [ ] cover · [ ] release URL

## Open risks
1. **Component coupling.** `HomeSection.tsx` (58 KB) and `CompactCapsule.tsx` (59 KB) have large prop surfaces. If mocking is ugly, use the replica fallback (S2 rule).
2. **WASAPI loopback non-float mix formats** (rare, some exclusive-mode drivers): handled by the meter fallback.
3. **Vercel build resolving `../src`**: verify on the first deploy.
4. **Phase F manual runtime check**: Spotify bass/treble response, pause decay, output-device switching, and CPU before/after still need an on-device listening check.
