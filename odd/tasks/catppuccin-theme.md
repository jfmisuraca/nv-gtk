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
- [ ] T1 Desktop: `src/theme.rs` with `pub fn load(display)` installing the Catppuccin provider (APPLICATION priority), called from `build_ui`.
- [ ] T2 Mobile: `android/app/src/main/java/ar/com/nvgtk/CatppuccinTheme.kt` with Latte/Mocha `ColorScheme`s; wire via `isSystemInDarkTheme()` in `MainActivity.setContent`.
- [ ] T3 Verify: `cargo check --workspace`, `xvfb-run -a cargo test -p nv-gtk --bin nv-gtk -- --test-threads=1`, `cd android && ./gradlew :app:assembleDebug` (no reinstall needed for this change; visual check is on next install).

## Authorized scope
Theme/colors only. No behavior changes, no new dependencies unless strictly required by the theme APIs already in use.

## Acceptance criteria
- Light variant renders Latte (`#eff1f5` base family), dark renders Mocha (`#1e1e2e` family)
- No test regressions; assembleDebug succeeds
- Autocomplete panel remains styled in both variants

## Route
Delegated direct writer (2 languages, 3+ files). Evidence per task recorded below.