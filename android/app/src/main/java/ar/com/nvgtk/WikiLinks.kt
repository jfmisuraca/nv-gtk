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

/**
 * Highlight range over the editor text in UTF-16 code units (Kotlin/Compose
 * indices). Produced from the core `extractWikiLinks` byte offsets via
 * [wikiHighlightRanges]; kept free of FFI types so JVM unit tests never load
 * the native library.
 */
data class WikiHighlightRange(val start: Int, val end: Int)

/**
 * Convert a UTF-8 byte offset (core `extractWikiLinks` semantics) into a
 * UTF-16 index over [text].
 *
 * The core counts bytes while Compose counts UTF-16 code units, so multibyte
 * chars (e.g. `é`, `中`) and non-BMP chars (e.g. emoji, 4 bytes UTF-8 but 2
 * UTF-16 units) shift the two coordinate spaces apart. Offsets are clamped:
 * negative → 0, past-the-end → [text.length], and mid-char offsets snap back
 * to the start of the enclosing char (core offsets always land on char
 * boundaries; this only guards synthetic/stale input).
 */
fun byteOffsetToUtf16Index(text: String, byteOffset: Long): Int {
    if (text.isEmpty() || byteOffset <= 0) return 0
    var bytes = 0L
    var i = 0
    while (i < text.length) {
        val codePoint = text.codePointAt(i)
        val byteLen = when {
            codePoint < 0x80 -> 1L
            codePoint < 0x800 -> 2L
            codePoint < 0x10000 -> 3L
            else -> 4L
        }
        if (bytes + byteLen > byteOffset) break
        bytes += byteLen
        i += Character.charCount(codePoint)
    }
    return i
}

/**
 * Map core-style UTF-8 [byteRanges] (`start`/`end` pairs in document order)
 * to Compose-ready [WikiHighlightRange]s over [text].
 *
 * Invalid entries are dropped, never crash: empty/reversed ranges
 * (`start >= end` after conversion) and ranges fully outside the text yield
 * nothing; partially out-of-range ends clamp to the text bounds. An open
 * trigger (`[[query` without `]]`) produces no byte ranges upstream, so it
 * maps to an empty highlight list — the panel owns that state, not the
 * transformation.
 */
fun wikiHighlightRanges(
    text: String,
    byteRanges: List<Pair<Long, Long>>
): List<WikiHighlightRange> {
    if (text.isEmpty() || byteRanges.isEmpty()) return emptyList()
    return byteRanges.mapNotNull { (startBytes, endBytes) ->
        val start = byteOffsetToUtf16Index(text, startBytes)
        val end = byteOffsetToUtf16Index(text, endBytes)
        if (start >= end) null else WikiHighlightRange(start, end)
    }
}
