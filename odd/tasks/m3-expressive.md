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

- [x] T0 **Bump Compose BOM to 2026.06.01 (material3 1.4)** — upgraded `android/app/build.gradle.kts` (BOM 2024.10.00 → 2026.06.01 → material3 1.4.0, ui/foundation 1.11.4, icons-core 1.7.8) and `android/build.gradle.kts` (AGP 8.5.2 → 8.6.1 forced: material3 1.4.0 AAR requires minAndroidGradlePluginVersion 8.6.0). Kotlin stayed 2.0.21. No source API breaks. Newer BOMs (2026.08/09) excluded — need minCompileSdk 37. Verified: `assembleDebug` BUILD SUCCESSFUL; app boots. Deliver: `4256c2c chore(android): bump compose BOM to 2026.06.01 (material3 1.4)`.
- [x] T1 **Dynamic color with Catppuccin fallback** — MainActivity now picks `dynamicDarkColorScheme`/`dynamicLightColorScheme` on `SDK_INT >= 31` following `isSystemInDarkTheme()`; below API 31 falls back to `CatppuccinDarkColors`/`CatppuccinLightColors`. Fixed error-container mapping in fallback (Latte errorContainer → flamingo `F2CDCD`, Mocha → maroon `EBA0AC`, real red-tinted containers). Verified: `assembleDebug` green; installed on device (`192.168.0.140:39277`), app runs (PID 486); user confirmed correct theme shift. Deliver: `3a68c9a feat(mobile): dynamic color from wallpaper on API 31+, Catppuccin fallback below`.
- [x] T2 **Adaptive layout + edge-to-edge + insets** — `enableEdgeToEdge()` in MainActivity; screens take `windowSizeClass` and cap content width (`widthIn(max=720.dp)`, CenteredBox TopCenter) for expanded layouts; self-implemented `enum WindowSizeClass { Compact, Medium, Expanded }` (cutoffs 600/840dp) because material3-adaptive 1.2.0 no longer ships `WindowSizeClass`. Editor insets: Scaffold bottom=0 + conditional `imePadding()` (keyboard open) / `navigationBarsPadding()` (closed); manifest `windowSoftInputMode="adjustResize"`. Rejected `BringIntoViewRequester` on the stateful `OutlinedTextField` — with foundation 1.11 CoreTextField it scrolls the whole container node, not the caret; CoreTextField scrolls the caret itself, so the container must only shrink. Verified: `assembleDebug` green; installed on device; rotation/landscape OK; editor with keyboard open: box shrinks to sit right above keyboard, caret follows taps at any position (no window pan, no box off-screen). Deliver: `3a68c9a` precedes; T2 lands in `feat(mobile): adaptive layout + edge-to-edge + insets`.
- [x] T3 **Predictive back + navigation polish** — manifest opt-in `android:enableOnBackInvokedCallback="true"` (application level; predictive back on Android 14+, always-on system behavior on 15+, verified merged manifest contains the flag). BackHandler already wired on all three screens (TrashScreen list/detail, NoteEditorScreen) — kept working; no crash on synthetic BACK at root, activity stays resumed. Verified: `assembleDebug` green; installed on device (API 36); gesture back available for visual confirmation. Deliver: T3 lands in `feat(mobile): opt in predictive back`.
- [x] T4 **Expressive motion** — screen transitions via `AnimatedContent` keyed on screen identity (`"trash"/"list"/"editor"` in NvApp, `trash.isEmpty()` in TrashScreen) with `fadeIn/Out + scaleIn/Out` (0.96–0.98) `togetherWith`; list items use `Modifier.animateItem()`; editor error + trash empty states use `AnimatedVisibility`; editor content column uses `animateContentSize()`; FAB keeps default M3 spring behavior. Subtle, per M3 guidance. Verified: `assembleDebug` green; installed + smoke on device (open note, back, trash, restore — no crash). Deliver: `8c7db4d feat(mobile): responsive window-size animations across screens`. NOTE: that commit landed on main + origin/main via a delegated worker that overstepped (merge/push are orchestrator/user decisions only) — content verified correct, push accepted by user, guard added for future delegations.
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