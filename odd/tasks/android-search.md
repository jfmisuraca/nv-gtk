# Feature: android-search (búsqueda rankeada en el móvil)

## Objective
Barra de búsqueda sobre la lista que filtra con el `searchNotes` rankeado del
core (mismo motor fuzzy + density gate que el desktop).

## Problem
Con 30 notas la lista ya exige scroll; sin búsqueda la app no escala.

## Why
Roadmap acordado; el core ya expone `searchNotes` por FFI y está probado.

## Scope
- `OutlinedTextField` de búsqueda arriba de la lista (lupa + limpiar),
  debounce 300ms, query vacía = lista completa.
- Resultados en orden de score del core; contador "N resultados" filtrando.
- NADA más: sin crear-desde-búsqueda (el FAB cubre altas), sin historial,
  sin resaltado de matches.

## Constraints
- Reusar `searchNotes` tal cual (paridad desktop, sin lógica duplicada).
- Búsqueda en `Dispatchers.IO`; cancelar la anterior en cada tecla
  (`LaunchedEffect(query)`).
- TDD: off.

## Checklist
- [x] T1: query/results en `NvApp` + barra en `NotesListScreen`.
- [x] T2: `assembleDebug` verde.
- [x] T3: commit + instalar en celu y probar con las 29 notas.

## Authorized scope
Usuario: "si" a la propuesta de búsqueda móvil. Merge/push: usuario.

## Acceptance criteria
- Escribir filtra rankeado; borrar la query restaura la lista completa.
- Sin regresiones: crear/editar/borrar/papelera intactos.

## Applicable checks
- `cd android && ./gradlew :app:assembleDebug`.

## Verification evidence
- `assembleDebug` verde al primer intento (LSP Kotlin ya diagnostica en vivo
  en la sesión y adelantó los params faltantes).
- Debounce 300ms vía `LaunchedEffect(query)` (cancela el pass anterior);
  `refresh()` re-corre el filtro para que crear/guardar no lo deje viejo.

## Progress
- Branch `feature/android-search` from `main@9f8b17d`.
