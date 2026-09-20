# Feature: android-scaffold (Gradle + Compose shell over nv-core)

## Objective
Minimal Android app that opens the real `nv-core` storage (via the UniFFI
bindings from `android-uniffi`) and lists notes with Jetpack Compose. No
editing yet — list-only shell to prove the full native → Kotlin path on a
device/emulator.

## Problem
The UniFFI surface (`NvStorage`, `NoteSnapshot`, `NvError`) and the generated
`nv_core.kt` exist, but there is no Gradle module consuming them: no
`applicationId`, no `jniLibs` wiring, no `MainActivity`.

## Why
Stack fixed by the user: Kotlin + Compose + `nv-core` via UniFFI/JNI,
`applicationId = ar.com.nvgtk` (user choice via question tool).

## Scope
- `android/` Gradle project (wrapper 8.10.2, AGP 8.5.2, Kotlin 2.0.21):
  `settings.gradle.kts`, root `build.gradle.kts`, `gradle.properties`,
  `:app` module (`namespace`/`applicationId` `ar.com.nvgtk`, `minSdk 26`,
  `compileSdk`/`targetSdk 35`, Compose BOM, JNA AAR for UniFFI).
- `MainActivity` (Material3, list-only): opens `NvStorage` at
  `filesDir/notes`, shows snapshots in a `LazyColumn` on `Dispatchers.IO`.
- Generated `nv_core.kt` copied into the `:app` source set (single-file copy;
  source of truth stays in `android-bindings/`).
- `app/src/main/jniLibs/<abi>/libnv_core.so` built via `cargo-ndk`
  (`arm64-v8a` + `x86_64`); the `.so` files are machine-built and gitignored,
  with a README documenting the exact rebuild commands.
- NADA más: no note editing, no navigation, no DI, no flavors.

## Constraints
- `nv-core` Rust code untouched by this feature (only consumed).
- No checked-in binaries: `*.so`, `.gradle/`, `build/`, `local.properties`
  are ignored. The SDK location comes from `ANDROID_HOME`.
- AGP runs on Java 17 (env default); Kotlin 2.0.x + `plugin.compose`.
- TDD: off. 400-line heuristic: orientative.

## Checklist
- [x] T1: Gradle files + manifest + MainActivity + bindings copy — route: inline.
- [x] T2: `cargo ndk` `.so` for both ABIs into `jniLibs` (gitignored).
- [x] T3: `./gradlew :app:assembleDebug` green.

## Authorized scope
User asked "continua" after the push; scaffold is the documented next step
(session memory). Push/merge of this branch: user decision.

## Acceptance criteria
- `:app:assembleDebug` produces an APK that lists desktop-created notes.
- `nv-core` tests still 22/22; desktop untouched.
- No `.so` or Gradle caches committed.

## Applicable checks
- `cargo test -p nv_core`, `cargo ndk -t arm64-v8a -t x86_64 build`,
  `cd android && ./gradlew :app:assembleDebug`.

## Verification evidence
- `cargo ndk -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs build -p nv_core --lib`
  → both `libnv_core.so` present (~95MB each, debug, gitignored).
- `cd android && ./gradlew :app:assembleDebug` → BUILD SUCCESSFUL (36 tasks),
  incl. `compileDebugKotlin` (MainActivity + copied `nv_core.kt`) and
  `mergeDebugNativeLibs`. Wrapper scripts generated canonically via
  `gradle wrapper --gradle-version 8.10.2` (hand-written `gradlew` had a
  syntax error and was replaced; stale daemon from the temp dist had to be
  killed once with `pkill -f GradleDaemon`).
- `nv-core` 22/22 green; desktop untouched by this branch.
- Language: Kotlin only (Compose + UniFFI bindings); no Java sources.
  No Kotlin LSP in this env — Gradle compile is the validator.

## Progress
- Branch `feature/android-scaffold` from `main@25e1a13`.

## Next step
- Editing, navigation, and autosave parity (new feature, after this shell
  runs on an emulator).
