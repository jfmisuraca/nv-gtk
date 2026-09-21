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
- [ ] T1: core + tests + spec regla 9.
- [ ] T2: desktop (Ctrl+D a papelera + diálogo Ctrl+T).
- [ ] T3: FFI + bindings + móvil (diálogo + pantalla papelera).
- [ ] T4: verificar todo, commits por área, instalar en celu.

## Authorized scope
Pedido del usuario ("implementes una papelera, tanto en movil como desktop").
Merge/push: usuario.

## Acceptance criteria
- Borrar en ambas apps mueve a papelera (recuperable); vaciar/purgar elimina.
- `cargo test -p nv_core` verde con tests de papelera; desktop y assemble verdes.

## Applicable checks
- `cargo test -p nv_core`, suite desktop xvfb, `:app:assembleDebug`.

## Verification evidence
- (to fill)

## Progress
- Branch `feature/trash` apilada sobre `feature/android-editor`.

## Next step
- Búsqueda móvil (sigue pendiente del roadmap del editor).
