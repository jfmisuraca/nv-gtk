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

/**
 * Strips a leading YAML frontmatter block, if present.
 *
 * Leading blank lines are skipped; the first non-blank line (trimmed) must
 * be exactly `---`. The block ends at the first subsequent line trimmed to
 * exactly `---`; everything through that closing line (inclusive) is
 * removed. Without a closing line the content is returned intact, so a lone
 * `---` keeps rendering as a horizontal rule. Same rule as
 * `nv_core::wiki::strip_frontmatter` (desktop parity).
 */
fun stripFrontmatter(content: String): String {
    val lines = content.lines()
    var i = 0
    while (i < lines.size && lines[i].trim().isEmpty()) i++
    if (i >= lines.size || lines[i].trim() != "---") return content
    i++
    while (i < lines.size) {
        if (lines[i].trim() == "---") {
            return lines.drop(i + 1).joinToString("\n")
        }
        i++
    }
    return content
}

/** Every line after the title line, joined; "" when there is no body. */
fun displayRemainder(content: String): String {
    val lines = stripFrontmatter(content).lines()
    val titleIndex = lines.indexOfFirst { it.trim().isNotEmpty() }
    if (titleIndex < 0) return ""
    return lines.drop(titleIndex + 1).joinToString("\n")
}

/**
 * True when [line] is a level-1 markdown heading: trimmed line is exactly
 * `"#"`, or starts with `"# "` / `"#\t"`. `"##..."` is not H1.
 */
fun isH1(line: String): Boolean {
    val trimmed = line.trim()
    return trimmed == "#" || trimmed.startsWith("# ") || trimmed.startsWith("#\t")
}

/**
 * Compact card title for the notes list: the first non-blank trimmed line
 * when it is an H1 heading, the [filename] stem verbatim otherwise, and
 * [emptyTitle] when the content has no non-blank line.
 */
fun cardTitle(content: String, filename: String, emptyTitle: String): String {
    val first = content.lineSequence().map { it.trim() }.firstOrNull { it.isNotEmpty() }
        ?: return emptyTitle
    return if (isH1(first)) first else filename
}
