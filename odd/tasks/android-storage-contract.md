# Feature: storage-contract (congelar contrato de storage)

## Objective
Dejar por escrito el contrato de storage de nv-gtk como spec + tests del core, para que la futura versión Android lo cumpla igual que el desktop y no diverjan.

## Problem
El contrato hoy está implícito en el código (`storage.rs`, `note.rs`, `config.rs`). Si Android lo reimplementa de memoria, cualquier regla distinta (ids, orden, colisiones) rompe la paridad silenciosamente.

## Why
Paso previo obligatorio a cualquier UI mobile. Pedido explícito del usuario ("perfecto, hacelo") tras proponer: "esas 8 reglas escritas como spec + tests del core".

## Scope
- `docs/storage-contract.md`: las 8 reglas (directorio, extensiones válidas, identidad id=filename, título=primera línea no vacía, contenido crudo + tags derivados por regex, fechas desde filesystem, orden por modified_at desc, colisiones con segundos+contador, guardado con re-sort).
- Tests de contrato en `src/storage.rs` / `src/note.rs` que verifican cada regla.
- NADA más: sin refactors de lógica, sin UI, sin código Android.

## Constraints
- No cambiar comportamiento existente; solo documentar + testear lo que ya hace el código.
- Tests existentes deben seguir verdes (`xvfb-run -a cargo test -- --test-threads=1`).
- TDD: off (sin sdd-init en memoria; checks funcionales ordinarios con `cargo test`).
- Heurística ~400 líneas por tarea: solo orientativa, no es criterio de aceptación.

## Checklist
- [x] T1 (delegated): `docs/storage-contract.md` con las 8 reglas — route: delegated (write trigger). Hecho por writer, +doc nuevo, 0 líneas borradas.
- [x] T2 (delegated): tests de contrato cubriendo las 8 reglas — route: delegated (mismo writer). 10 tests nuevos (4 en storage.rs + 6 en note.rs); diff total +222/-0, sin cambios de comportamiento.
- [x] T3 (parent): verificación spot + work-unit commit en la rama — route: inline. Spot-check propio: 14/14 verde. Commit work-unit en `feature/storage-contract` (4 archivos, +387/-0).

## Authorized scope
Autorización explícita del usuario a spec + tests del contrato. Push/PR: decisiones del usuario (política del repo).

## Acceptance criteria
- `docs/storage-contract.md` existe y describe las 8 reglas observables en el código actual.
- Cada regla tiene al menos un test que la verifica; suite verde.
- Ningún test existente roto; ningún cambio de comportamiento.

## Applicable checks
- `xvfb-run -a cargo test -- --test-threads=1` (verificado baseline 4/4 verde en main@3e9f49d).
- Ruido ambiental conocido: warnings MESA-EGL/DRI3 bajo xvfb (no son fallas).

## Progress
- Rama `feature/storage-contract` creada desde `main@3e9f49d`.

## Verification evidence
- Writer: `xvfb-run -a cargo test -- --test-threads=1` → 14 passed, 0 failed (solo ruido MESA-EGL/DRI3).
- Parent spot-check (mismo comando): 14 passed, 0 failed.
- RDD global ON → tras el commit se evalúa riesgo con `review assess` sobre el rango commiteado.
- Assess del rango commiteado de la rama (base `3e9f49d`, committed-only): tier `medium` (motivo: `executable_change` en src/note.rs — tests nuevos), `review_due: false`, motivo `under_budget` → sin revisión, el slice sigue pendiente hasta llegar al presupuesto.

## Hallazgo (regla 4, para paridad Android)
El brief decía "título = primera línea no vacía del contenido", pero el modelo hace `title = filename stem` (`Note::from_file`); la primera línea solo se usa para **mostrar** la fila en la UI (`build_note_row` en window.rs). La spec documenta el comportamiento real del código y el test `rule4_title_is_stem_not_first_content_line` lo fija. Decisión pendiente (fuera de este scope): Android debe replicar AMBAS cosas — `title=id=stem` en el modelo y primera-línea-solo-para-display en la fila.

## Next step
- Completado. Pendiente solo decisión del usuario: push/PR de `feature/storage-contract` (política del repo) y decisión regla-4 para Android.
