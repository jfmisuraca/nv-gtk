# Feature: android-uniffi (bindings UniFFI del core para Kotlin)

## Objective
Exponer `nv-core` a Kotlin vía UniFFI (Mozilla): scaffolding FFI en el crate + bindings Kotlin generados, con tipos aptos para FFI. Sin scaffold Gradle todavía (siguiente tarea).

## Problem
Los tipos actuales no cruzan FFI (`PathBuf`, `DateTime<Local>`, `Vec<Note>` con paths): hay que definir una superficie FFI deliberada (paths como String, fechas como i64 millis, records/enums UniFFI, errores como enum).

## Why
Stack fijado por el usuario: Kotlin + Compose + `nv-core` vía UniFFI/JNI. UniFFI evita escribir JNI a mano. Entorno verificado: ANDROID_HOME con NDK 27/28 + Java 17; falta instalar targets Rust + cargo-ndk + uniffi (el writer lo hace).

## Scope
- `nv-core`: dependencia `uniffi`, crate-type `cdylib` (+rlib), módulo de interfaz (`uniffi.udl` o macros `#[uniffi::export]`), API FFI: abrir storage en un dir, listar/buscar/crear/guardar/borrar notas, tipos record Snapshot (id/title/content/tags/modified_ms/created_ms).
- Bindings Kotlin generados y guardados bajo `android-bindings/` (artefacto versionado para el futuro scaffold).
- Adaptadores FFI puros (conversión tipos internos ↔ tipos FFI) con tests.
- NADA más: sin Gradle, sin Compose, sin JNI manual, sin cambios de comportamiento del desktop (el binario debe seguir verde).

## Constraints
- Desktop intacto: `cargo check --workspace` + suite 14/14 verdes.
- `nv-core` sigue sin GTK; uniffi es la única dependencia nueva.
- Android targets a instalar: `aarch64-linux-android` (+`x86_64-linux-android` para emulador); linkers del NDK vía cargo-ndk o `.cargo/config.toml` (el writer elige y documenta).
- Verificación T1: `cargo check -p nv_core`, generación de bindings OK, y `cargo build -p nv_core --target aarch64-linux-android` (o reporte `partial` honesto si el linker NDK bloquea).
- TDD: off. Heurística 400 líneas: orientativa, no criterio.

## Checklist
- [x] T1 (delegated): scaffolding UniFFI + bindings Kotlin + tests adaptadores — route: delegated. Hecho por writer: `ffi.rs` (~330 líneas: `NvStorage`, `NoteSnapshot`, `NvError`, adaptadores + 5 tests), bin `uniffi-bindgen`, uniffi 0.32 + cdylib, bindings en `android-bindings/` (1744 líneas generadas + README con comando exacto). Targets aarch64/x86_64 + cargo-ndk instalados (env, no commiteados).
- [x] T2 (parent): spot-check + work-unit commit — route: inline. Spot-check: 19/19 + `.so` aarch64 presente + bindings presentes.

## Authorized scope
Autorización del usuario al camino Kotlin+core; bindings como paso técnico. Push/PR: decisiones del usuario.

## Acceptance criteria
- `nv-core` expone API UniFFI documentada; bindings Kotlin generados en `android-bindings/`.
- Tests de adaptadores verdes; desktop 14/14 verde; `nv-core` compila para `aarch64-linux-android`.
- Sin cambios de comportamiento del binario.

## Applicable checks
- `cargo check -p nv_core`, `cargo test -p nv_core`, suite workspace completa, `cargo build -p nv_core --target aarch64-linux-android`.
- Ruido ambiental: MESA-EGL/DRI3 (no fallas).

## Verification evidence
- Writer: `cargo check -p nv_core` limpio; `cargo test -p nv_core` 17/17; workspace 19/19; `cargo build -p nv_core --target aarch64-linux-android` → `.so` generado (linker NDK 28 vía env); `cargo ndk -t arm64-v8a build` también verificado.
- Parent spot-check: workspace 19/19 (2 desktop + 17 nv-core), `.so` (95MB) y `nv_core.kt` presentes en disco. Sin flake del portrait en esta corrida.
- Decisiones FFI: proc-macros sobre UDL (single source, sin drift IDL); `PathBuf`→`String`, `DateTime`→`i64` millis, `Vec<Note>`→`Vec<NoteSnapshot>`; `Mutex` poisoning recuperado sin pánico; `setup_scaffolding!()` obligatorio en lib.rs.
- RDD: assess sobre el rango tras el commit.

## Progress
- Rama `feature/android-uniffi` sobre `d5dfabd` (apila sobre PR #1 + nv-core sin PR aún).

## Next step
- Crear rama, delegar T1; luego T2. Scaffold Gradle/Compose después (nueva feature).
