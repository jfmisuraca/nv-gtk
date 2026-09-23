# Catppuccin theme (light/dark following system)

## Objective
Give the app a Catppuccin palette on both platforms: Latte for light, Mocha for dark. The active variant follows the OS theme automatically (libadwaita + Compose already switch on their own); no manual toggle.

## Problem
- Desktop currently uses stock Adwaita colors (libadwaita `ApplicationWindow`); only the autocomplete panel has custom CSS that already references theme variables (`@theme_base_color`).
- Mobile (`MainActivity.kt`) uses stock `MaterialTheme` with no custom scheme — always light-ish defaults.

## Scope
- Desktop: one application-scoped `GtkCssProvider` with `@variant (light)` / `@variant (dark)` blocks redefining Adwaita color variables to Latte/Mocha. No manual mode switching.
- Mobile: Catppuccin `ColorScheme` (Latte light, Mocha dark), selected via `isSystemInDarkTheme()` in `MainActivity`.
- No `nv-core`/FFI changes → no UniFFI bindings or `.so` rebuild.

## Constraints
- Existing autocomplete CSS uses `@theme_base_color` and `@borders` — it must keep working once overrides redefine those variables.
- Files stay under `window.rs`/`main.rs` and the `android/app/src/main/java/ar/com/nvgtk/` tree.
- No .so/bindings regeneration.

## Tasks
- [x] T1 **Desktop theme** — `src/theme.rs` (NEW, 228 lines): one
  application-priority `GtkCssProvider` overriding the libadwaita named colors
  with Latte/Mocha; `theme::load(&display)` is called from `window::build_ui`
  before the window is built (`src/window.rs:177-181`), so the first frame
  already carries the palette. **Deviation from the original spec:** the plan
  asked for `@variant (light)` / `@variant (dark)` blocks, but the GTK CSS
  parser (verified against 4.22.5) rejects that at-rule and drops the block
  ("Unknown @ rule"). `@media (prefers-color-scheme: ...)` is GTK's supported
  conditional — the same one libadwaita itself uses — so the OS-following
  behavior is identical. On GTK >= 4.20 the media feature is evaluated against
  the provider's own `prefers-color-scheme`, so `load` binds it to the display
  settings' `gtk-interface-color-scheme` (binding deliberately leaked, matching
  the provider registration lifetime).
  Deliver: `c341f24 feat(theme): catppuccin palette following system on both platforms`.
- [x] T2 **Mobile scheme** — `android/app/src/main/java/ar/com/nvgtk/CatppuccinTheme.kt`
  (NEW): `CatppuccinLightColors` (Latte) / `CatppuccinDarkColors` (Mocha) mapped
  onto Material 3 semantic roles, selected with `isSystemInDarkTheme()`.
  **Partly superseded after delivery:** `m3-expressive` T1 made wallpaper
  dynamic color the primary scheme on API 31+, so on modern devices Catppuccin
  is the **fallback** below API 31 (`MainActivity.kt:103-112`; the
  `Catppuccin*Colors` branch is the `else`). That is the documented mobile color
  strategy, not a regression. Deliver: `c341f24`.
- [x] T3 **Verify** — all three commands run against the merged tree:
  `cargo check --workspace` → exit 0; `xvfb-run -a cargo test -p nv-gtk --bin
  nv-gtk -- --test-threads=1` → `2 passed; 0 failed`; `cd android &&
  ./gradlew :app:assembleDebug` → `BUILD SUCCESSFUL`. The visual check was still
  pending at delivery; it is now covered by `scripts/verify-theme.sh` (below).

## Theme verification (2026-09-22, retroactive)

`scripts/verify-theme.sh` runs the real binary headless and asserts rendered
pixels, not the stylesheet source:

| Variant | Window background | Theme surface | Palette |
| --- | --- | --- | --- |
| `color-scheme=prefer-light` | `#EFF1F5` | `#E6E9EF` | Latte |
| `color-scheme=prefer-dark` | `#1E1E2E` | `#181825` | Mocha |

- "Light renders Latte / dark renders Mocha": **verified on pixels** — exact
  Catppuccin base/mantle hexes on a running app window.
- "Autocomplete panel remains styled in both variants": **verified statically** —
  both blocks define the same 37 unique color names, and both aliases the panel
  uses (`@theme_base_color`, `@borders`, `src/wiki_autocomplete.rs:162-163`)
  resolve in each variant.

**Gotcha (cost a false pass):** libadwaita's `AdwStyleManager` — not
`GtkSettings` — decides the effective scheme, and it prefers the
`org.freedesktop.appearance` portal, which proxies the *real* session
preference. A plain headless run therefore silently reports the developer's own
theme: a first attempt "verified" light mode while actually rendering Mocha.
Pin the preference with `GSETTINGS_BACKEND=keyfile` **and**
`ADW_DISABLE_PORTAL=1`, or the check is vacuous. That isolation is what makes a
light-mode assertion meaningful, since a real `prefer-light` system is otherwise
easy to fake.

**Not verified:** switching the OS theme while the app is running (the binding
is designed for it, but no test exercises a live switch).

## Authorized scope
Theme/colors only. No behavior changes, no new dependencies.

## Acceptance criteria
- [x] Light variant renders Latte (`#eff1f5` base family), dark renders Mocha (`#1e1e2e` family)
- [x] No test regressions; assembleDebug succeeds
- [x] Autocomplete panel remains styled in both variants

## Route
Delegated direct writer (2 languages, 3+ files). Evidence per task recorded above.

## Delivery note
Delivered as `c341f24`, which reached `main` absorbed by the m3-expressive merge
(`c07c74f`). These checkboxes stayed unticked because the feature document was
never updated after delivery: the work was complete, the bookkeeping was not.
Closed on 2026-09-22 together with the retroactive pixel verification and
`scripts/verify-theme.sh`.