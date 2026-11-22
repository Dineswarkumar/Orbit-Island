# ORBIT egui visual parity report

## Summary

The visual update now has a flat Opaque surface, three levels of glass blur, a flat tinted no-blur fallback, timed ease-out expansion, a three-weight Segoe UI setup, consistent Phosphor Regular icons, and explicit toggle states. The saved default remains **Normal**.

- **Normal:** light glass, 14 px blur.
- **Realistic:** balanced glass, 22 px blur.
- **Blurred:** strongest glass, 34 px blur.
- **Opaque:** solid theme-aware surface with a crisp border; no blur, grain, gradient, or drop shadow.
- **Transparency fallback:** uses the same surface tint, rim, and top highlight with blur disabled.

The calendar and shelf geometry were left unchanged. Calendar text weights were adjusted only.

## Changes by file

- `crates/ui/src/theme.rs` — Added dark/light Opaque surface tokens, a real Segoe UI Bold face, and 19.5 px pill clock sizing (up from 16.5 px).
- `crates/platform/src/fonts.rs` — Loads `segoeuib.ttf` for the bold face; missing fonts still fall back through egui's proportional family.
- `crates/app/src/app.rs` — Passes the bold font asset and Opaque surface selection into the UI.
- `crates/ui/src/anim.rs` — Removed spring physics. Replaced it with a cubic ease-out tween that runs for 200 ms while opening and 160 ms while closing, with repaint requests only while motion is active.
- `crates/ui/src/panel/mod.rs` — Applies the tween to island width, height, and placement; draws Opaque separately from glass and the tinted fallback.
- `crates/ui/src/pill.rs` — Added the flat Opaque shell, uses the bold clock, and routes icons through the same Phosphor Regular family.
- `crates/ui/src/paint.rs` — Added a single-tint glass fallback surface with the same rim, inner edge, top highlight, and shadow, without gradient or grain.
- `crates/ui/src/bubble.rs` — Uses the shared open/close tween timing for bubble expansion.
- `crates/ui/src/panel/home.rs` — Makes the large clock bold at 37 px, uses regular body labels, and gives timer presets a visible pressed state.
- `crates/ui/src/panel/calendar.rs` — Uses regular weight for calendar grid labels and values; dimensions and positions are unchanged.
- `crates/ui/src/settings.rs` — Explains the four surface modes and shows On/Off beside every toggle.

## Checkpoints

| Checkpoint | Status | Evidence / reason |
|---|---|---|
| Opaque matches old 13 s Tauri reference side by side | **Unverified** | The named 13 s reference was not present. The available local videos are 103.1 s and 113.7 s and do not establish the requested old-build comparison. No screenshot of the rebuilt app was captured. |
| Blurred matches old 33 s reference on bright and dark wallpaper | **Unverified** | Same missing-reference and no rebuilt-app screenshot limitation; both wallpaper conditions were not exercised. |
| No pale ring or flash during open/close | **Unverified** | Existing Composition backdrop uses one rounded clip for the blur and rim, but the transition was not captured live. |
| Transparency-off fallback works | **Partly verified** | The fallback now uses a single tint with the same rim/highlight and blur is disabled in the passing `blur_sigma_for_mode` test. Windows transparency was not toggled for a screenshot. |
| Open/close visually matches old build side by side | **Unverified** | Timing is implemented in source; no old-build side-by-side capture was available. |
| Clock bold and larger; titles semibold; body regular | **Partly verified** | Font loading and role assignments are in source and the workspace compiles/tests. No live screenshot was captured. |
| Settings toggles and timer states are clear | **Partly verified** | Settings render test passes at 460, 640, and 960 px widths. Toggle text is explicit; preset buttons highlight while pressed, and the running timer retains its progress treatment. No live screenshot was captured. |
| Calendar and shelf layouts unchanged | **Verified by diff** | Calendar changes affect text weights only; no shelf source or geometry was changed. |
| Idle repaints remain 1–2 per minute | **Unverified at runtime** | Idle scheduling remains tied to the next minute boundary, with extra repaint requests only during active tweens. No idle runtime capture/trace was taken. |
| Condition A (Opaque) vs Condition F (Blurred) resource measurement | **Not measured** | Release build could not replace the existing `target/release/orbit-egui.exe` because Windows returned Access Denied. No running `orbit-egui` process was found to explain the lock, so no A/F runtime samples were taken. |
| `cargo test --workspace` | **Pass** | 62 tests passed; 0 failed. |

## Resource use

Condition A vs Condition F: **no measured difference available**. The release executable could not be rebuilt, so quoting memory or CPU values would be made up. The existing measurement script was not modified by this task; it already had an unrelated local edit before work began.

## References and licenses

- [Microsoft Fluent Acrylic material guidance](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic) — visual guidance for background blur, tint, depth, and solid fallback. Reference only; no code copied. The page is documentation, not an open-source code package.
- [glass-ui](https://github.com/lemuayala/glass-ui) — MIT license ([license file](https://github.com/lemuayala/glass-ui/blob/main/LICENSE)); frosted-surface visual reference only; no code copied.
- [UI Explorer](https://github.com/Chinto-lgtm/ui-explorer) — MIT license ([license file](https://github.com/Chinto-lgtm/ui-explorer/blob/main/LICENSE)); visual reference for component state and theme treatment only; no code copied.
- [shadcn/ui](https://github.com/shadcn-ui/ui) — MIT license; accessible switch and settings-control state reference only; no code copied.
- Existing adapted UI code remains attributed under the repository's [third-party notices](THIRD_PARTY_NOTICES.md) (Nook, MIT). No new third-party code or font files were added.

## Not completed

- Old Tauri side-by-side screenshots, bright/dark wallpaper checks, live ring/flash checks, and the A/F performance run remain unverified for the reasons above.
- `cargo fmt --check` could not run because the installed Rust toolchain does not include the `rustfmt` component.
- The initial request mentioned “Claude font” and “five sets” without identifying the intended typeface or whether those sets mean font families or theme presets. The detailed parity requirements specify regular, semibold, and bold roles; those are implemented. No additional named font family presets were guessed.
