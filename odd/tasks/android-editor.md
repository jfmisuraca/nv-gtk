# Feature: android-editor (crear, editar y borrar notas en Compose)

## Objective
Convertir el shell list-only en una app usable: abrir nota, editar contenido,
guardar, crear con FAB y borrar desde el editor — todo contra el `NvStorage`
real via UniFFI.

## Problem
El scaffold solo lista: sin navegación, sin edición, sin altas ni bajas. El
título es el id (filename stem, regla 4 del contrato) y no hay API de rename,
así que el título se muestra read-only y solo el contenido es editable.

## Why
Roadmap acordado con el usuario tras probar el shell en su celu (lista OK con
30 notas reales). Siguiente paso natural: edición.

## Scope
- Navegación por estado (`selectedId`, sin `navigation-compose`): lista ↔ editor.
- `NotesListScreen`: tarjetas clicables + FAB crear (`createNote("Nota nueva")`,
  colisiones las resuelve el core) + error inline.
- `NoteEditorScreen`: título read-only, `OutlinedTextField` de contenido,
  guardar-si-cambió al volver (botón + gesto back), borrar sin confirmación
  (v1), errores `NvError` como texto inline, I/O en `Dispatchers.IO`.
- Nueva dep `material-icons-core` (Add/Delete/ArrowBack).
- NADA más: sin rename de título, sin confirmación de borrado, sin búsqueda
  móvil, sin undo, sin tests instrumentados.

## Constraints
- `nv-core` y bindings intactos (solo consumo).
- Sin nuevas dependencias salvo iconos.
- TDD: off. Sin Kotlin LSP en la sesión viva (requiere restart) — el
  compilador (`compileDebugKotlin`) es el validador.

## Checklist
- [x] T1: `NotesListScreen` + `NoteEditorScreen` + `NvApp` + iconos — route: inline.
- [x] T2: `assembleDebug` verde.
- [x] T3: commit en la rama (merge a main: decisión del usuario).

## Authorized scope
Usuario: "si y continua con lo que dijiste del roadmap". Push/merge: usuario.

## Acceptance criteria
- Crear, editar (persiste y reordena), borrar y volver funcionan en el celu.
- `assembleDebug` verde; `nv-core` 22/22 intacto.

## Applicable checks
- `cd android && ./gradlew :app:assembleDebug`.

## Verification evidence
- `./gradlew :app:assembleDebug` → BUILD SUCCESSFUL, `compileDebugKotlin`
  limpio al primer intento (sin LSP en la sesión viva; el compilador validó).
- Sin correr en dispositivo todavía: instalar por adb wireless + probar
  crear/editar/borrar queda pendiente.
- Deuda v1 anotada: título read-only (sin API rename), borrado sin
  confirmación, sin búsqueda móvil.

## Progress
- Branch `feature/android-editor` from `main@a5b8209`.

## Next step
- Búsqueda móvil (ranked via `searchNotes`), confirmación de borrado,
  rename de título (requiere API en el core: no existe).
