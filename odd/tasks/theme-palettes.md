# Theme palettes (Dracula, Flexoki, Catppuccin, wallpaper-derived)

## Objective
Let the user pick the app's color theme on both platforms: **Dracula**, **Flexoki** and
**Catppuccin** (already present), each rendered in its **light and dark** variant, plus a
**wallpaper-derived** theme — Material You dynamic color on Android, **pywal** on desktop. The
light/dark variant keeps following the OS color scheme; what the user picks is the *palette*.

## Problem
- Desktop: `src/theme.rs` (228 lines) hardcodes one Catppuccin stylesheet (`CATPPUCCIN_CSS`),
  Latte for light and Mocha for dark. No other palette is reachable and there is no external
  color source.
- Android: `MainActivity.kt:102-117` picks dynamic color on API >= 31 (`Build.VERSION_CODES.S`)
  and falls back to Catppuccin; `CatppuccinTheme.kt` holds exactly one hardcoded scheme pair.
- No picker exists on either platform. Desktop has no header bar and no preferences surface at
  all; Android's NavHost has only `list`/`editor`/`trash`.
- Android persists nothing: `android/app/build.gradle.kts` has no DataStore, no Room, and no
  SharedPreferences usage anywhere.

## Scope
- Desktop: palette-driven CSS generation replacing the hardcoded Catppuccin constant, a pywal
  color source, a persistent theme choice, and a picker in a libadwaita header bar.
- Android: additional `ColorScheme`s, a persistent theme choice, a Settings screen, and a boot
  frame that matches the active palette.
- No `nv-core` FFI surface change and no `.so` rebuild: `Config` is **not** exposed over UniFFI
  (`nv-core/src/ffi.rs` uses it internally, it is not a `uniffi::Record`), so adding a plain serde
  field is source-only. Note the one compile impact: `nv-core/src/ffi.rs:111` builds `Config`
  with a struct literal, so the new field must be added there too (or the literal switched to
  `..Config::default()`).

## Constraints
- Keep the existing desktop CSS structure: the 37 libadwaita named-color overrides and the
  `@media (prefers-color-scheme: light|dark)` blocks driven by the `gtk-interface-color-scheme`
  binding (GTK >= 4.20). `@variant` is NOT supported by GTK CSS — do not reintroduce it.
- `scripts/verify-theme.sh` must keep passing, and the bespoke autocomplete CSS in
  `src/wiki_autocomplete.rs:162-163` must keep resolving (`@theme_base_color`, `@borders`).
- Android stays single-locale Rioplatense Spanish in `values/`; do not create `values-es/`.
- Prefer zero new dependencies. Android persistence uses `SharedPreferences`, not DataStore.
- `minSdk = 26`: dynamic color must stay behind the API 31 guard.

## Authoritative palettes (do not invent hexes)

Sources: `draculatheme.com/spec` (Dracula + Alucard, fetched), `kepano/flexoki` README
(fetched), and `catppuccin` Latte/Mocha already verbatim in `src/theme.rs:33-189`.

**Dracula** (dark): bg `#282A36`, bgDark `#21222C`, bgDarker `#191A21`, bgLight `#343746`,
bgLighter `#424450`, selection `#44475A`, comment `#6272A4`, fg `#F8F8F2`, red `#FF5555`,
orange `#FFB86C`, yellow `#F1FA8C`, green `#50FA7B`, cyan `#8BE9FD`, purple `#BD93F9`,
pink `#FF79C6`, lineFallback `#353747`.

**Alucard** (Dracula light): bg `#FFFBEB`, floating `#EFEDDC`, bgLighter `#ECE9DF`,
bgLight `#DEDCCF`, bgDark `#CECCC0`, bgDarker `#BCBAB3`, selection `#CFCFDE`, comment `#6C664B`,
fg `#1F1F1F`, red `#CB3A2A`, orange `#A34D14`, yellow `#846E15`, green `#14710A`,
cyan `#036A96`, purple `#644AC9`, pink `#A3144D`, lineFallback `#E2DECA`.

**Flexoki** (dark): bg `#100F0F` (black), ramp `#1C1B1A` `#282726` `#343331` `#403E3C`
`#575653` `#6F6E69` `#878580` `#9F9D96` `#B7B5AC`, text `#F2F0E5`, accents (600):
red `#AF3029`, orange `#BC5215`, yellow `#AD8301`, green `#66800B`, cyan `#24837B`,
blue `#205EA6`, purple `#5E409D`, magenta `#A02F6F`.

**Flexoki** (light): bg `#FFFCF0` (paper), ramp `#F2F0E5` `#E6E4D9` `#DAD8CE` `#CECDC3`
`#B7B5AC` `#9F9D96` `#878580` `#6F6E69` `#575653`, text `#100F0F`, accents (400):
red `#D14D41`, orange `#DA702C`, yellow `#D0A215`, green `#879A39`, cyan `#3AA99F`,
blue `#4385BE`, purple `#8B7EC8`, magenta `#CE5D97`.

**Catppuccin**: unchanged — Latte `src/theme.rs:35-56`, Mocha `src/theme.rs:114-135`.

### Mapping rule
Per theme/variant, define the 12 neutral slots (`base`, `mantle`, `crust`, `surface0..2`,
`overlay0..2`, `subtext0..1`, `text`) and the accent slots (`red`, `green`, `yellow`, `blue`,
`cyan`, `purple`, `pink`, `orange`). Where a theme does not define an explicit elevation step,
derive it deterministically by blending `base` toward `text` at fixed documented ratios
(direction chosen from `base` luminance), and record the ratio in a comment. Accent slots the
theme lacks are aliased to the theme's nearest defined hue — never to another theme's colors.

## Tasks
- [x] T1 **Desktop palette module** — `src/palettes.rs` (NEW): `ThemeId`
  (`Catppuccin`/`Dracula`/`Flexoki`/`Wallpaper`), `Variant` (`Light`/`Dark`), a `Palette` struct
  holding the neutral + accent slots above, and the authoritative tables for the three named
  themes. `ThemeId::parse` / `as_str` for persistence. Tests: every named theme defines every
  required slot in both variants; all values parse as `#rrggbb`; `text` vs `base` contrast
  >= 4.5:1 per theme/variant (WCAG AA, per the Dracula spec's own bar).
  - **Done (1fa4a33):** `src/palettes.rs` (421 lines). Evidence: `xvfb-run -a cargo test -p
    nv-gtk --bin nv-gtk` -> palettes::tests 5/5 ok (slot completeness in both variants,
    lowercase `#rrggbb`, WCAG AA text/base contrast, `ThemeId` round-trip, `Wallpaper` ->
    Catppuccin fallback).
- [x] T2 **Desktop CSS generator + live apply** — refactor `src/theme.rs` so the 37 libadwaita
  variables are generated from a `Palette` instead of the hardcoded constant, preserving the
  `@media (prefers-color-scheme)` structure and the GTK>=4.20 provider binding. Change
  `theme::load` to return a retained `gtk4::CssProvider` handle plus `theme::apply(&provider,
  theme_id)` so the palette can change at runtime without restarting. `verify-theme.sh` must
  still pass for Catppuccin.
  - **Done (1fa4a33):** `src/theme.rs` (367 lines) generates the 22 slots + 39 aliases per
    `@media` block from a `Palette`; `ThemeHandle::apply` re-targets the provider at runtime.
    Evidence: theme::tests 2/2 ok + `sh scripts/verify-theme.sh` -> PASS light `#EFF1F5` /
    dark `#1E1E2E` / alias parity 37 / autocomplete aliases resolve. (`%%TOKEN%%` +
    `String::replace` keeps the literal `@define-color` lines the script greps.)
- [ ] T3 **Desktop pywal source** — read `~/.cache/wal/colors.json`
  (`special.background`, `special.foreground`, `colors.color0..15`; fall back to the
  `~/.cache/wal/colors` 16-line file). Map to a `Palette` with the documented blend rule.
  Missing, malformed or partial input returns `None`; the caller falls back to Catppuccin.
  Tests over fixture JSON, including a missing-file case.
- [ ] T4 **Desktop persistence** — add the theme field to `nv_core::Config` with
  `#[serde(default)]` so an existing `~/.config/nv-gtk/config.json` without it still loads
  (backward compatibility), and update the `Config` struct literal at `nv-core/src/ffi.rs:111`.
  Tests: round-trip and legacy-file load.
- [ ] T5 **Desktop picker UI** — install an `adw::HeaderBar` on the `ApplicationWindow` with a
  menu button whose popover lists the four themes as radio items; selection applies the palette
  immediately and persists it. **Flagged decision:** this changes the window chrome from the
  plain titlebar to a libadwaita header bar.
- [ ] T6 **Desktop verification** — extend `scripts/verify-theme.sh` to assert rendered pixels
  per theme and variant (Dracula `#282A36`, Alucard `#FFFBEB`, Flexoki `#100F0F`/`#FFFCF0`,
  Catppuccin `#1E1E2E`/`#EFF1F5`), reusing the `GSETTINGS_BACKEND=keyfile` +
  `ADW_DISABLE_PORTAL=1` isolation already documented in `odd/tasks/catppuccin-theme.md`, plus
  one pywal-fixture run.
- [ ] T7 **Android palettes** — `ThemePalettes.kt` (NEW): Dracula/Alucard and Flexoki light/dark
  `ColorScheme`s mapped onto the same Material 3 roles `CatppuccinTheme.kt` already uses, an
  `NvTheme` enum, and `colorSchemeFor(theme, dark, context)` returning the dynamic scheme for
  `Wallpaper` on API >= 31.
- [ ] T8 **Android persistence** — a `SharedPreferences`-backed theme preference (no new
  dependency). Default is `Wallpaper`, which resolves to Catppuccin below API 31 — preserving
  today's effective behavior.
- [ ] T9 **Android picker UI** — a `SettingsScreen` on a new `settings` route reachable from the
  notes list top bar; `strings.xml` additions in Rioplatense Spanish; a11y semantics as in T7 of
  the M3 Expressive plan.
- [ ] T10 **Android boot frame** — set the window background from the active palette in
  `onCreate` before `setContent`, so the static day/night XML color in
  `values/themes.xml` / `values-night/themes.xml` never flashes the wrong palette.
- [ ] T11 **Verification** — `cargo check --workspace`, the desktop test suite, `./gradlew
  :app:assembleDebug`, and a Compose/unit test for the mapping and the default-preference
  resolution.

## Authorized scope
Theming and color only, plus the minimum navigation/settings surface needed to choose a theme.
No behavior changes to notes, search, storage or trash. No new runtime dependencies.

## Acceptance criteria
- [ ] Each of Catppuccin, Dracula and Flexoki renders its own light and dark palette on both
      platforms, following the OS variant.
- [ ] `Wallpaper` renders Material You on Android (API >= 31) and pywal colors on desktop when
      pywal output exists, and falls back to Catppuccin when it does not.
- [ ] The choice survives an app restart on both platforms.
- [ ] No test regressions; `assembleDebug` succeeds; `verify-theme.sh` passes.

## Route
Delegated direct writers, one writer per task group. Desktop (Rust) and Android (Kotlin) are
separate writer sequences; never two writers in the same worktree at once.

## Delivery forecast
~1300-1500 authored changed lines across 12+ files, well over the ~400-line budget, so the
delivery strategy applies before the first pull request: `ask-on-risk` -> ask once whether to
split into chained PRs (`stacked-to-main` or `feature-branch-chain`) or proceed with an explicit
`size:exception`. Work-unit commits on the feature branch are not gated by this.

**Delivery decision (2026-09-22):** strategy `ask-on-risk` resolved to **`stacked-to-main`** —
chained PRs, each merging to `main` in order. Slice boundaries are recorded here as they are cut;
work-unit commits on `feature/theme-palettes` are not gated by the budget.

**Review status:** `gentle-ai review assess` on `eb97911..HEAD` reports `risk: medium`,
`review_due: true`, `review_due_reason: slice_budget_reached` (855 changed lines). The preflight
with `--base-ref --committed-only` resolves to a START with `--consent=relay`, so a review is
available for this range (the earlier `immutable_review_transport_unsupported` refusal applies to
the selectorless/workspace path, not to this one).

## Open risks
- Adding a header bar visibly changes the desktop window chrome (T5). Veto-able.
- pywal palettes are arbitrary; the blend rule can produce low-contrast pairs. T1's contrast
  test covers the named themes; pywal gets a structural check plus a documented fallback.
- Android has no persistence layer today, so T8 introduces the first one; keep it minimal.
