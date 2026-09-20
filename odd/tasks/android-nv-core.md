# Feature: nv-core (extraer núcleo portable a crate separada)

## Objective
Extraer la lógica portable (`note`, `storage`, `search`, `config` + helpers puros de `app_state`) a una crate de librería `nv-core`, dejando en el binario `nv-gtk` solo lo atado a GTK. Es el paso que habilita reutilizar el core en Android vía JNI/UniFFI.

## Problem
Hoy todo vive en un solo crate binario con dependencias GTK al alcance; no se puede compilar la lógica sola para otra plataforma.

## Why
Decisión de stack pendiente, pero la extracción es común a ambas rutas (Kotlin+core o incluso como referencia para RN). Autorización explícita del usuario ("hacelos y segui con el proximo paso").

## Scope
- Workspace Cargo: root sigue siendo el binario + miembro `nv-core/` (mínima disrupción, sin mover `src/main.rs` de lugar).
- Mover a `nv-core/src/`: `note.rs`, `storage.rs`, `search.rs`, `config.rs` + helpers puros `timestamp_title*` (para cortar el dep de `storage.rs` → `app_state`).
- Mover dependencias puras a `nv-core` (chrono, regex, serde/serde_json, dirs, fuzzy-matcher); gtk4/glib/etc. quedan en la app.
- Actualizar imports en la app a `nv_core::...`; los tests se mudan con sus módulos.
- NADA más: sin cambios de comportamiento, sin JNI todavía, sin código Android.

## Constraints
- Suite verde antes y después (`cargo test --workspace` bajo xvfb con `--test-threads=1`); los 14 tests deben seguir pasando (movidos, no borrados).
- `cargo check` limpio para ambos miembros.
- TDD: off (checks funcionales ordinarios).
- Esta rama apila sobre `feature/storage-contract` (PR #1 abierto) — el PR del core sale después de que #1 mergee, o como stacked.

## Checklist
- [x] T1 (delegated): extracción completa del workspace + crate — route: delegated. Hecho por writer: `nv-core/` (lib `nv_core`, `lib.rs`, `util.rs`, 4 módulos movidos con `git mv` + tests), `[workspace]` en root, imports a `nv_core::...`. `regex` queda en ambos crates (wiki_link lo usa); `notify` sin uso intacto.
- [x] T2 (parent): spot-check + work-unit commit — route: inline. Ver evidencia abajo.

## Authorized scope
Autorización explícita a la extracción del core. Push/PR: decisiones del usuario.

## Acceptance criteria
- `cargo check` verde en workspace; `nv-core` compila sin ninguna dependencia GTK.
- Los 14 tests pasan (reubicados con su módulo).
- El binario funciona igual (sin cambios de comportamiento).

## Applicable checks
- `cargo check --workspace` y `xvfb-run -a cargo test --workspace -- --test-threads=1`.
- Ruido ambiental: warnings MESA-EGL/DRI3 (no son fallas).

## Progress
- Rama `feature/nv-core` creada sobre `63df026` (apila sobre PR #1).
- Spec `docs/storage-contract.md`: rutas `src/*.rs` → `nv-core/src/*.rs` (+helpers a `util.rs`) corregidas por el move (edición mecánica del parent).

## Verification evidence
- Writer: `cargo check --workspace` limpio; `cargo test --workspace` 14/14; `nv-core` sin deps GTK (grep sobre `cargo tree -p nv_core` vacío).
- Parent spot-checks: `cargo check` verde; suite completa verde en corridas alternadas.
- HALLAZGO HONESTO — flake preexistente: `window::tests::responsive_portrait_layout_and_overlay` falla intermitentemente SOLO en suite (~1/5 en binario, aislado siempre pasa; mismo patrón visto en `main@3e9f49d` pre-extracción). Causa: test sensible a timing (pump con iteraciones fijas + resize bajo render software). NO es regresión del move (solo se movieron módulos y reescribieron imports). Queda registrado; arreglarlo es tarea nueva fuera de este scope.
- RDD: tras el commit se corre `review assess` sobre el rango.
- Assess (base `63df026`, committed-only): tier `medium`, `review_due: true` (`slice_budget_reached`) → preflight + START; consentimiento `granted` por el usuario; lente nativo `review-reliability`.
- REVIEW NO COMPLETADA (causa ambiental, no del código): el relay de subagentes (Task) rechaza lanzar el reviewer ("free tier usable only from within OpenCode") — falló 2 veces (explore + review-reliability). Slot declarado inalanzable (`capture-unachievable`, recorded), lifecycle detenido en `unachievable_lens_slot`. Sin acknowledgement: no hay receipt, la autoridad no se quemó. Prueba funcional en pie: 14/14 + spot-checks + `nv-core` sin GTK.

## Next step
- Commit work-unit + assess; push/PR quedan a decisión del usuario.
