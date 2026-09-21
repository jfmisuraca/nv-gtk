# M3 Expressive adoption (Android app)

## Objective
Bring the Android app (ar.com.nvgtk) to Material 3 Expressive: dynamic color, adaptive layout, expressive motion, emphasized typography, expressive shapes, and settle accessibility debts. Desktop (GTK) is out of scope for this feature — it keeps Catppuccin.

## Problem (from diagnosis, 2026-09-21)
- Motion: absent entirely (no animations, screen transitions are raw `when` swaps).
- Adaptive: no window size classes, no insets, no edge-to-edge, no predictive back.
- Typography: only 6 baseline M3 styles in use; nothing emphasized.
- Shapes: all defaults; no expressive shape tokens.
- Color: static Catppuccin only; no dynamic color. `errorContainer` mapped to non-red surface.
- Debts: text fields without labels, hand-rolled rows (no semantic roles), hardcoded Spanish strings, `Theme.Material.NoActionBar` XML theme, compose BOM `2024.10.00` (material3 1.3.x) predates Expressive.

## Decision (user, 2026-09-21)
- Color strategy: **dynamic color** from wallpaper (`dynamicLightColorScheme`/`dynamicDarkColorScheme` on API 31+), with the **existing Catppuccin Latte/Mocha schemes as fallback** on older devices — they stay as `CatppuccinTheme.kt` fallback. On API 31+ the app takes the wallpaper palette; the Catppuccin desktop look is a mobile-only fallback.
- Execution mode: Automatic (default) for phases; user was asked 1 product question and answered.

## Constraints
- Android-only (mobile package). No `nv-core`/FFI/`.so` changes expected (visual layer only).
- Min SDK 26, target/compile 35 (current). Dynamic color gated at API 31+.
- Keep state-based navigation unless a task explicitly replaces it.
- Existing behavioral flows (trash, search, rename, editor) must keep working; this is a visual/haptic/system integration layer.
- Verification per task: `cd android && ./gradlew :app:assembleDebug` green; when behavior changed, reinstall via adb and manual smoke on device.

## Tasks (ordered by impact; each closes with a work-unit commit on this branch)

- [ ] T0 **Bump Compose BOM to 2025.x (material3 1.4+)** — upgrade `android/app/build.gradle.kts` (BOM 2024.10.00 → newest stable with material3 1.4.x; keep Kotlin/AGP compatible), fix any API breaks (e.g. TopAppBar/experimental annotations may change). Verify assembleDebug + app still boots. Deliver: dependency upgrade commit.
- [ ] T1 **Dynamic color with Catppuccin fallback** — in theme layer: on `Build.VERSION.SDK_INT >= 31` use `dynamicLightColorScheme(context)` / `dynamicDarkColorScheme(context)` based on `isSystemInDarkTheme()`; below API 31 keep `CatppuccinLightColors`/`CatppuccinDarkColors`. Wrap app in `MaterialExpressiveTheme` (new API) while it stays compatible with current BOM; otherwise `MaterialTheme` with the expressive scheme. Fix error-container mapping in fallback (use red-tinted container) if staying static. Verify: assembleDebug + install; toggle wallpaper/dark to confirm palette shifts on API 31+, Catppuccin below.
- [ ] T2 **Adaptive layout + edge-to-edge + insets** — `enableEdgeToEdge()` in MainActivity; handle `WindowInsets` (`safeDrawing`, `ime`): editor must not hide under keyboard (`imePadding`), lists/content respect system bars; add window size class (material3-adaptive or foundation) and use it for orientation/large screens (current code has zero handling). Verify: rotate phone, open keyboard over editor, check landscape + (if possible) split-screen.
- [ ] T3 **Predictive back + navigation polish** — opt in `android:enableOnBackInvokedCallback="true"` in manifest (predictive back on Android 14+), keep `BackHandler` working. Verify: gesture back on editor/trash yields predictive animation on API 34+.
- [ ] T4 **Expressive motion** — add `AnimatedContent`/`Crossfade` for screen transitions (list→editor→trash→detail), spring-based appear for FAB, `animateContentSize`/`AnimatedVisibility` for search results counter and empty states, list item enter animations. Keep it subtle per M3 guidance. Verify: transitions feel smooth on device, no heavy jank.
- [ ] T5 **Emphasized typography** — upgrade to the full M3 type scale via the expressive theme's typography tokens (15 baseline + 15 emphasized); apply emphasized styles to: note titles in editor (`headlineMedium`-ish emphasized), search result count, trash detail title, empty states. Prefer existing default typefaces (Roboto) — no custom font download in this task; note variable-font option as future. Verify: visual check on device.
- [ ] T6 **Expressive shapes** — define `Shapes` tokens (e.g. `largeIncreased = RoundedCornerShape(36.dp)` style from M3 story, larger `extraLarge*`), apply corner emphasis to `ElevatedCard`, search field, dialogs, FAB. Keep contrast per M3: mix square/round intentionally. Verify: assembleDebug + visual check both themes.
- [ ] T7 **Accessibility + i18n debts** — labels (not just placeholders) on search/editor/rename fields; semantic roles (`Card(onClick)`, `role` on clickables) replacing raw `Modifier.clickable` rows; move all hardcoded Spanish strings to `res/values/strings.xml` (+ `values-es` if default should be neutral; decide default locale during task); consistent `contentDescription`s; touch targets ≥ 48dp. Verify: assembleDebug + TalkBack smoke (optional on device).
- [ ] T8 **Cleanup / XML theme** — replace platform `@android:style/Theme.Material.NoActionBar` with a proper Material3/Compose theme XML (e.g. `Theme.Material3.DayNight.NoActionBar`) and check manifest; remove the now-dead `@OptIn(ExperimentalMaterial3Api)` where stable, remove unused `ui-tooling-preview` if still unused or add `@Preview`s for the three screens (bonus). Verify: assembleDebug + boot.

## Authorized scope
Android UI/theme layer only. No storage, no FFI, no behavior changes to note lifecycle. Credit + chat language: keep persona in chat; artifacts English, UI strings per strings.xml task.

## Acceptance criteria
- BOM ≥ 2025.x with material3 1.4+; assembleDebug green on every task.
- API 31+: wallpaper drives scheme; dark follows system. Below: Catppuccin fallback still correct.
- Editor usable with keyboard open; no content under system bars; rotation works.
- Predictable back animations on 14+; existing flows keep working.
- Visual: expressive type/shape/motion visible but not noisy; both palettes accessible.
- Strings extracted; fields have real labels; rows announce roles; touch targets OK.
- Feature doc current; commit per task with conventional message.

## Notes / risks
- BOM bump may surface breaking changes in components (TopAppBar etc.) — T0 must not be skipped.
- `MaterialExpressiveTheme` may be experimental (`@ExperimentalExpressiveApi`) — decide annotation strategy in T0/T1.
- Dynamic color may shift brand feel drastically; fallback keeps identity on old devices. If the user dislikes wallpaper palettes on API 31+, revisit decision (would add toggle — out of scope now).