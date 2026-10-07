package ar.com.nvgtk

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.sp

/**
 * Minimal markdown renderer — same visible subset as desktop `markdown_spans`
 * (`src/window.rs`): headings, bold, italic, inline/block code, bullet +
 * ordered lists, links as `text (url)`, horizontal rules.
 *
 * Dependency-free on purpose: no markdown library resolves from the offline
 * Gradle cache, so this ships zero new dependencies. Inline nesting
 * (`**_both_**`) is not supported — the outer marker wins.
 */
private val BulletItem = Regex("""^(\s*)[-*+]\s+(.*)$""")
private val OrderedItem = Regex("""^(\s*)(\d+)[.)]\s+(.*)$""")
private val RuleLine = Regex("""^\s*(-{3,}|_{3,}|\*{3,})\s*$""")
private val InlineToken =
    Regex("""\*\*.+?\*\*|__.+?__|\*[^*\n]+?\*|_[^_\n]+?_?|`[^`\n]+?`|\[[^\]\n]+?\]\([^)\n]+?\)""")

private fun leadingIndent(line: String): String {
    val spaces = line.takeWhile { it == ' ' }.length
    return "  ".repeat(spaces / 2)
}

private fun AnnotatedString.Builder.appendInline(
    text: String,
    codeStyle: SpanStyle,
    linkStyle: SpanStyle,
    urlStyle: SpanStyle
) {
    var cursor = 0
    for (match in InlineToken.findAll(text)) {
        if (match.range.first > cursor) {
            append(text.substring(cursor, match.range.first))
        }
        cursor = match.range.last + 1
        val token = match.value
        when {
            token.startsWith("**") && token.endsWith("**") ->
                withStyle(SpanStyle(fontWeight = FontWeight.Bold)) {
                    appendInline(token.drop(2).dropLast(2), codeStyle, linkStyle, urlStyle)
                }
            token.startsWith("__") && token.endsWith("__") ->
                withStyle(SpanStyle(fontWeight = FontWeight.Bold)) {
                    appendInline(token.drop(2).dropLast(2), codeStyle, linkStyle, urlStyle)
                }
            token.startsWith("`") ->
                withStyle(codeStyle) { append(token.drop(1).dropLast(1)) }
            token.startsWith("[") -> {
                val split = token.indexOf("](")
                val label = token.drop(1).take(split - 1)
                val url = token.drop(split + 2).dropLast(1)
                withStyle(linkStyle) { appendInline(label, codeStyle, linkStyle, urlStyle) }
                withStyle(urlStyle) { append(" ($url)") }
            }
            token.startsWith("*") ->
                withStyle(SpanStyle(fontStyle = FontStyle.Italic)) {
                    append(token.drop(1).dropLast(1))
                }
            token.startsWith("_") ->
                withStyle(SpanStyle(fontStyle = FontStyle.Italic)) {
                    append(token.drop(1).dropLast(1))
                }
            else -> append(token)
        }
    }
    if (cursor < text.length) {
        append(text.substring(cursor))
    }
}

/**
 * Pure markdown-to-[AnnotatedString] conversion. Colors come in as parameters
 * so JVM unit tests can pass constants without the Android framework.
 */
fun renderMarkdownAnnotated(
    text: String,
    linkColor: Color,
    codeBackground: Color
): AnnotatedString {
    val codeStyle = SpanStyle(fontFamily = FontFamily.Monospace, background = codeBackground)
    val linkStyle = SpanStyle(color = linkColor, textDecoration = TextDecoration.Underline)
    val urlStyle = SpanStyle(color = linkColor)
    return buildAnnotatedString {
        var first = true
        fun nextLine() {
            if (!first) append('\n')
            first = false
        }
        var inCodeBlock = false
        for (rawLine in stripFrontmatter(text).split('\n')) {
            val fence = rawLine.trimStart().startsWith("```")
            if (fence) {
                inCodeBlock = !inCodeBlock
                continue
            }
            if (inCodeBlock) {
                nextLine()
                withStyle(codeStyle) { append(rawLine) }
                continue
            }
            val trimmed = rawLine.trimStart()
            when {
                trimmed.startsWith("#") -> {
                    val level = trimmed.takeWhile { it == '#' }.length
                    if (trimmed.length == level || trimmed[level].isWhitespace()) {
                        val size = when {
                            level <= 1 -> 24.sp
                            level == 2 -> 20.sp
                            else -> 17.sp
                        }
                        nextLine()
                        withStyle(SpanStyle(fontSize = size, fontWeight = FontWeight.Bold)) {
                            appendInline(
                                trimmed.drop(level).trimStart(),
                                codeStyle,
                                linkStyle,
                                urlStyle
                            )
                        }
                        continue
                    }
                    nextLine()
                    appendInline(rawLine, codeStyle, linkStyle, urlStyle)
                }
                trimmed.startsWith(">") -> {
                    nextLine()
                    append("> ")
                    appendInline(
                        trimmed.drop(1).trimStart(),
                        codeStyle,
                        linkStyle,
                        urlStyle
                    )
                }
                RuleLine.matches(rawLine) -> {
                    nextLine()
                    append("───")
                }
                BulletItem.matches(rawLine) -> {
                    val body = BulletItem.matchEntire(rawLine)!!.groupValues[2]
                    nextLine()
                    append(leadingIndent(rawLine) + "• ")
                    appendInline(body, codeStyle, linkStyle, urlStyle)
                }
                OrderedItem.matches(rawLine) -> {
                    val m = OrderedItem.matchEntire(rawLine)!!
                    nextLine()
                    append(leadingIndent(rawLine) + m.groupValues[2] + ". ")
                    appendInline(m.groupValues[3], codeStyle, linkStyle, urlStyle)
                }
                else -> {
                    nextLine()
                    appendInline(rawLine, codeStyle, linkStyle, urlStyle)
                }
            }
        }
    }
}

/**
 * Read-only scrollable preview column. Replaces the editor field (plus its
 * follow-chip/autocomplete chrome) while preview mode is on.
 */
@Composable
fun MarkdownPreview(
    text: String,
    modifier: Modifier = Modifier
) {
    val linkColor = MaterialTheme.colorScheme.primary
    val codeBackground = MaterialTheme.colorScheme.surfaceVariant
    val rendered = remember(text, linkColor, codeBackground) {
        renderMarkdownAnnotated(text, linkColor, codeBackground)
    }
    Column(modifier.verticalScroll(rememberScrollState())) {
        if (rendered.text.isBlank()) {
            Text(
                stringResource(R.string.editor_placeholder),
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        } else {
            Text(rendered)
        }
    }
}
