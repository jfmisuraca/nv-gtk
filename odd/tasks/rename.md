# Feature: rename (renombrar notas en ambas apps)

## Objective
Cambiar el título de una nota = cambiar su filename stem (id), conservando
contenido, tags y fechas. En core + FFI + desktop + móvil.

## Problem
El título es el id y no hay API de rename: en móvil el título es read-only,
en desktop no hay forma de renombrar.

## Why
Pedido del usuario ("renombrar notas") tras cerrar papelera y búsqueda.

## Scope
- Core: `rename_note(id, new_title) -> Result<Option<Note>>` (None = unknown):
  trim, vacío→"Untitled", `/`→`-`, colisión→timestamp desambiguado (misma
  regla que create, excluyendo la propia nota), `fs::rename` (conserva fechas,
  no toca mtime → no reordena solo), fallo→reinserta + Err. Spec regla 3/10.
- FFI: `rename_note` → snapshot o `NotFound`; bindings regen.
- Desktop: Ctrl+R abre diálogo con Entry pretextuado (tras flush de autosave),
  Enter renombra y re-selecciona el nuevo id.
- Móvil: tap en el título del editor lo vuelve editable; Done renombra,
  back cancela la edición (no sale).
- NADA más: sin rename en papelera (restaurar ya desambigua), sin batch.

## Constraints
- Sin tocar mtime en rename (el orden por modificado no debe moverse).
- Desktop keyboard-driven (diálogo solo para el input, como la papelera).
- TDD: off.

## Checklist
- [x] T1: core + tests + spec.
- [x] T2: FFI + test + bindings.
- [x] T3: desktop Ctrl+R.
- [x] T4: móvil título editable.
- [x] T5: verificar, commits, instalar en celu.

## Authorized scope
Usuario: "si" al diseño propuesto. Merge/push: usuario.

## Acceptance criteria
- Renombrar conserva contenido/tags/fechas, no pisa, desambigua colisiones.
- `cargo test -p nv_core` y suites verdes; assemble verde.

## Applicable checks
- `cargo test -p nv_core`, desktop xvfb, `:app:assembleDebug`.

## Verification evidence
- Core 29/29 (2 rename tests); refactor `note_title_exists` → `title_taken`
  con exclusión (create/restore migran sin cambios de conducta).
- Desktop suite verde; `cargo check` limpio.
- Móvil `assembleDebug` verde; instalado por adb en el celu.
- Commits: `cb960af` core+spec, `36f6a21` ffi+bindings, `5c10507` desktop,
  `b520ade` móvil+taskdoc.

## Progress
- Branch `feature/rename` from `main@40ed5d6`.
