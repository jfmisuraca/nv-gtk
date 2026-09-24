# Note display title — desktop parity on Android

## Objective
Android shows the same title logic as desktop: the filename stays `AAAAMMDD-HHMM.md`
(with seconds on collision) and the DISPLAYED title is the first non-blank line of the
file content.

## Problem
- Android renders `NoteSnapshot.title` (filename stem) in 4 places: notes list
  (`NotesListScreen.kt:186`), editor TopAppBar (`NoteEditorScreen.kt:204`), trash list
  (`TrashScreen.kt:194`), trash detail + purge dialog (`TrashScreen.kt:255`, `:236`).
- Desktop derives the displayed title UI-only: first non-blank trimmed content line,
  fallback `"(nota vacía)"`, `#` shown verbatim, long lines untruncated in data
  (`src/window.rs:450-456`).
- Android creates the first note as `"Nota nueva"` instead of a timestamp
  (`MainActivity.kt:227` + `strings.xml:33`); timestamps appear only via collision.
- No first-line derivation exists anywhere in `android/app/src/main` (0 hits).

## Scope
Android UI layer only (Kotlin). No `nv-core` / FFI / binding changes: `title == stem`
contract stays, derivation happens in Kotlin.

## Constraints
- Single-locale Rioplatense Spanish in `values/`; no new locale dirs.
- Code identifiers/comments in English; user-visible strings in Spanish.
- Keep `key = { it.id }` (stem) for list identity; keep `padding(20.dp)` card lines intact.
- No commits by the writer; changes stay uncommitted for user review.

## Decisions (user-approved 2026-09-23)
- List preview shows content from the SECOND non-blank line on (`take(140)` +
  `maxLines=3` kept on the remainder); hidden when the remainder is blank.
- Empty/blank note title fallback: `"(nota vacía)"` (new `note_empty_title` resource,
  desktop parity).
- `#`/markdown shown verbatim after trim; no truncation in data (visual ellipsis only).
- Editor rename keeps renaming the FILE; TopAppBar displays the derived title.
- Purge dialog interpolates the derived title.
- Creation passes Kotlin `yyyyMMdd-HHmm` timestamp; core already disambiguates
  collisions with seconds (desktop-identical path).

## Tasks
- [x] T1 **Display helper + tests** — `displayTitle(content)` and remainder helper
  (first non-blank trimmed line / `"(nota vacía)"`; body after that line), new
  `NoteDisplayTitleTest` (JUnit4): empty, all-blank, leading blanks, trim, `#`
  verbatim, long line untouched, remainder blank vs non-blank.
- [x] T2 **Notes list** — title uses helper; preview uses remainder (`take(140)`,
  `maxLines=3`), hidden when blank.
- [x] T3 **Editor + trash** — editor TopAppBar shows derived title (rename flow
  untouched, still operates on filename); trash list, detail TopAppBar and purge
  dialog use derived title.
- [x] T4 **Timestamp creation + strings** — `MainActivity` creates with Kotlin
  timestamp instead of `new_note_default_title`; remove the now-unused resource
  iff unreferenced.
- [x] T5 **Verification** — `./gradlew :app:testDebugUnitTest` + `:app:assembleDebug`,
  both green.

## Acceptance criteria
- [ ] No stem shown as a title anywhere in list/editor/trash/purge; empty notes show
      `"(nota vacía)"`.
- [ ] Preview never repeats the title line; hidden when there is no body.
- [ ] New notes land as `AAAAMMDD-HHMM.md`; same-minute collision yields the
      seconds variant without overwrite.
- [ ] Rename still renames the file; editor autosave unaffected.
- [ ] New unit tests green; no regressions in existing tests; `assembleDebug` green.

## Checks
- `./gradlew :app:testDebugUnitTest` (from `android/`)
- `./gradlew :app:assembleDebug` (from `android/`)

## Verification (2026-09-23, uncommitted worktree)
- `NoteDisplayTitleTest` 8/8 + `ThemeMappingTest` 7/7 = 15/15 en
  `:app:testDebugUnitTest`, BUILD SUCCESSFUL.
- `:app:assembleDebug` BUILD SUCCESSFUL (solo 2 warnings preexistentes).
- Editor deriva del texto vivo (no del snapshot) para no congelar el TopAppBar.
- Aceptación T1–T5 cumplida según evidencia de arriba; queda validación visual en
  dispositivo (títulos vacíos, colisión mismo-minuto) solo a pedido explícito.

## Route
One delegated writer (all files listed above, single worktree, no parallel writers).
Small coherent slice: no PR splitting, no delivery-strategy ceremony.

## Delivery forecast
~5 touched files, well under any budget. Uncommitted working-tree delivery for user
review; device install only on explicit request with `--user 0` (memory #52).
