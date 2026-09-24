package ar.com.nvgtk

/**
 * Desktop-parity derived note title (UI-only, see `src/window.rs`).
 *
 * The filename stem stays the note identity (`title == stem` core contract);
 * these helpers only decide what the UI SHOWS: the first non-blank content
 * line, trimmed, with `#`/markdown kept verbatim and long lines untouched
 * (visual ellipsis only, never truncated in data).
 *
 * Pure JVM (no Android framework) so unit tests run without instrumentation.
 */

/** First non-blank trimmed content line, or [emptyTitle] when there is none. */
fun displayTitle(content: String, emptyTitle: String): String =
    content.lineSequence().map { it.trim() }.firstOrNull { it.isNotEmpty() } ?: emptyTitle

/** Every line after the title line, joined; "" when there is no body. */
fun displayRemainder(content: String): String {
    val lines = content.lines()
    val titleIndex = lines.indexOfFirst { it.trim().isNotEmpty() }
    if (titleIndex < 0) return ""
    return lines.drop(titleIndex + 1).joinToString("\n")
}
