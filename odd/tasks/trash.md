# Feature: papelera (trash en la app, no la del sistema)

## Objective
Borrar mueve la nota a una papelera propia (`notes_dir/.trash/`); restaurar,
eliminar definitivo y vaciar — en desktop GTK y en Android, sobre el mismo
core.

## Problem
`delete_note` borraba el archivo sin retorno. El usuario quiere papelera de la
aplicación (no la del sistema operativo).

## Why
Pedido explícito del usuario tras probar el editor en su celu, junto con el
retoque del diálogo ("Borrar nota NOMBRE-NOTA?", sin "para siempre").

## Scope
- Core (`nv-core/src/storage.rs`): `TRASH_DIR_NAME = ".trash"`, `delete_note`
  mueve (rename, conserva fechas) con desambiguación si el nombre existe en la
  papelera; `trash_notes()`, `restore_note()` (desambigua si el nombre fue
  retomado), `purge_note()`, `empty_trash()`; `reload` ignora `.trash`.
  Regla 9 del spec.
- Desktop: Ctrl+D mueve a papelera; diálogo Papelera con Ctrl+T (lista con
  Restaurar/Eliminar por fila + Vaciar + Cerrar); refresh vía `update_search`.
- FFI: `list_trash`, `restore_note`, `purge_note`, `empty_trash`; bindings
  regenerados.
- Móvil: diálogo "Borrar nota X?" (mueve a papelera); pantalla Papelera
  (restaurar, eliminar definitivo con confirmación, vaciar con confirmación).
- NADA más: sin expiración automática, sin límite de tamaño.

## Constraints
- `rename` mismo filesystem (notas y papelera comparten dir): sin copy fallback.
- Desktop sigue keyboard-driven (sin botones nuevos en la ventana principal).
- TDD: off.

## Checklist
- [x] T1: core + tests + spec regla 9.
- [x] T2: desktop (Ctrl+D a papelera + diálogo Ctrl+T).
- [x] T3: FFI + bindings + móvil (diálogo + pantalla papelera).
- [x] T4: verificar todo, commits por área, instalar en celu.

## Authorized scope
Pedido del usuario ("implementes una papelera, tanto en movil como desktop").
Merge/push: usuario.

## Acceptance criteria
- Borrar en ambas apps mueve a papelera (recuperable); vaciar/purgar elimina.
- `cargo test -p nv_core` verde con tests de papelera; desktop y assemble verdes.

## Applicable checks
- `cargo test -p nv_core`, suite desktop xvfb, `:app:assembleDebug`.

## Verification evidence
- Core: `cargo test -p nv_core` 26/26 (3 trash tests + reworked io test).
- Desktop: suite xvfb 2/2 + 26/26; `cargo check` limpio (un `Rc::new_cyclic`
  con `dyn` no compila — se resolvió con función libre `rebuild_trash_rows`).
- FFI: `cargo build` + bindgen regen OK (`listTrash/restoreNote/purgeNote/
  emptyTrash` presentes); test `trash_roundtrip_through_ffi_object` verde.
- Móvil: `assembleDebug` verde (`RestoreFromTrash` no está en icons-core →
  botón de texto "Restaurar"); instalado por adb wireless, 29 notas intactas
  (el usuario había borrado 1 probando el build anterior, pre-papelera).
- Commits: `74c7660` core+spec, `15202cd` desktop, `109046d` ffi+móvil.

## Progress
- Branch `feature/trash` apilada sobre `feature/android-editor`.

## Next step
- Búsqueda móvil (sigue pendiente del roadmap del editor).
