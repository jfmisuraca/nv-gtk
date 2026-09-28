package ar.com.nvgtk

/**
 * Wiki-link helpers for the editor (parity slice 3).
 *
 * The matching rules live in the core (`nv-core/src/wiki.rs`, reached over
 * FFI): these helpers only do the cursor/line math around the Compose text
 * field so it stays unit-testable on the JVM without instrumentation.
 */

/**
 * The current line up to [cursor] (never spans lines). Feed it to the core
 * `openWikiQuery` trigger rule.
 */
fun lineBeforeCursor(text: String, cursor: Int): String {
    val safe = cursor.coerceIn(0, text.length)
    val lineStart = text.lastIndexOf('\n', safe - 1).let { if (it < 0) 0 else it + 1 }
    return text.substring(lineStart, safe)
}

/**
 * T8: autocomplete visibility gate. The core `openWikiQuery` only sees the
 * line BEFORE the cursor, so a cursor inside a closed `[[link]]` still
 * yields a partial query (e.g. `[[cl|osed]]` → `Some("cl")`). The core
 * `linkAtCursor` fires there too, and desktop semantics give follow priority:
 * a closed link navigates, only an open trigger autocompletes. So the panel
 * shows only for an open query with NO closed link under the cursor.
 */
fun shouldShowWikiAutocomplete(openQuery: String?, hasActiveLink: Boolean): Boolean =
    openQuery != null && !hasActiveLink

/**
 * Offset just after the pending `[[` for [cursor] (mirrors the desktop
 * `start_offset`): the `[[` itself is kept, the query after it is replaced.
 * Callers guarantee [query] is the core trigger result for this cursor.
 */
fun pendingWikiStart(cursor: Int, query: String): Int =
    (cursor - query.length).coerceAtLeast(0)

/**
 * Insert a chosen candidate: replace the pending `[[query` with
 * `displayTitle]]` (mirrors the desktop insert). Returns the new text and
 * the cursor position right after the closing brackets.
 */
fun insertWikiCompletion(
    text: String,
    cursor: Int,
    query: String,
    displayTitle: String
): Pair<String, Int> {
    val safe = cursor.coerceIn(0, text.length)
    val start = pendingWikiStart(safe, query).coerceAtMost(safe)
    val completed = "$displayTitle]]"
    return (text.substring(0, start) + completed + text.substring(safe)) to
        (start + completed.length)
}
