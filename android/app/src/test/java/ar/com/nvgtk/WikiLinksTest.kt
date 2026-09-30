package ar.com.nvgtk

import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pure-JVM tests for the wiki cursor helpers (parity slice 3). The trigger
 * rule (`openWikiQuery`) and ranking live in the core and are tested in
 * Rust; here only the Compose-side line/cursor math is covered (no native
 * library is loaded by these tests).
 */
class WikiLinksTest {

    @Test
    fun lineBeforeCursor_stopsAtLineStart() {
        assertEquals("segunda [[pr", lineBeforeCursor("primera\nsegunda [[pr", 20))
        assertEquals("", lineBeforeCursor("abc", 0))
        assertEquals("abc", lineBeforeCursor("abc", 3))
    }

    @Test
    fun lineBeforeCursor_clampsOutOfRangeCursors() {
        assertEquals("abc", lineBeforeCursor("abc", 99))
        assertEquals("", lineBeforeCursor("abc", -4))
    }

    @Test
    fun pendingWikiStart_landsJustAfterBrackets() {
        // "nota [[proy", cursor at end: query "proy" starts at index 7.
        assertEquals(7, pendingWikiStart(11, "proy"))
        assertEquals(4, pendingWikiStart(4, ""))
    }

    @Test
    fun insertWikiCompletion_replacesPendingQueryAndCloses() {
        val (text, cursor) = insertWikiCompletion("nota [[proy resto", 11, "proy", "Proyecto")
        assertEquals("nota [[Proyecto]] resto", text)
        assertEquals(17, cursor)
    }

    @Test
    fun insertWikiCompletion_atEndOfText() {
        val (text, cursor) = insertWikiCompletion("ver [[be", 8, "be", "beta")
        assertEquals("ver [[beta]]", text)
        assertEquals(text.length, cursor)
    }

    @Test
    fun insertWikiCompletion_emptyQuery_keepsBrackets() {
        val (text, cursor) = insertWikiCompletion("a [[ resto", 4, "", "beta")
        assertEquals("a [[beta]] resto", text)
        assertEquals(10, cursor)
    }

    @Test
    fun wikiTouchTarget_meetsMaterialMinimum() {
        // T6: suggestion rows + follow-chip share this minimum (Material 48dp).
        assertEquals(48.dp, WikiTouchTargetMinHeight)
    }

    @Test
    fun autocomplete_showsForOpenTriggerOnly() {
        // `[[query` without `]]`: core open query fires, no closed link.
        assertEquals(true, shouldShowWikiAutocomplete("query", false))
        // Bare `[[`: empty query still opens the panel.
        assertEquals(true, shouldShowWikiAutocomplete("", false))
    }

    @Test
    fun autocomplete_hiddenOnClosedLink_followWins() {
        // T8 regression: cursor inside `[[cl|osed]]` yields a partial open
        // query ("cl") AND a closed link — follow-chip wins, no suggestions.
        assertEquals(false, shouldShowWikiAutocomplete("cl", true))
        // Cursor just after `[[closed]]`: no open query, link under cursor.
        assertEquals(false, shouldShowWikiAutocomplete(null, true))
    }

    @Test
    fun autocomplete_hiddenWithoutTrigger() {
        // Plain text (no `[[` before cursor): neither UI shows.
        assertEquals(false, shouldShowWikiAutocomplete(null, false))
    }

    @Test
    fun highlightRanges_asciiByteOffsetsAreIdentity() {
        // "ver [[nota]] fin": "[[nota]]" spans bytes 4..12, same in UTF-16.
        val text = "ver [[nota]] fin"
        assertEquals(
            listOf(WikiHighlightRange(4, 12)),
            wikiHighlightRanges(text, listOf(4L to 12L))
        )
        assertEquals("[[nota]]", text.substring(4, 12))
    }

    @Test
    fun highlightRanges_multibyteShiftsByteSpace() {
        // 'é' is 2 bytes in UTF-8 but 1 UTF-16 unit: "café " is 6 bytes,
        // 5 UTF-16 units, so byte range (6, 14) maps to UTF-16 (5, 13).
        val text = "café [[nota]]"
        assertEquals(
            listOf(WikiHighlightRange(5, 13)),
            wikiHighlightRanges(text, listOf(6L to 14L))
        )
        assertEquals("[[nota]]", text.substring(5, 13))
        // A mid-char byte offset snaps back to the enclosing char start.
        assertEquals(3, byteOffsetToUtf16Index(text, 4))
    }

    @Test
    fun highlightRanges_emojiNonBmpCountsTwoUtf16Units() {
        // '😀' is 4 bytes in UTF-8 but 2 UTF-16 units (surrogate pair):
        // "😀 " is 5 bytes, 3 UTF-16 units.
        val text = "\uD83D\uDE00 [[nota]]"
        assertEquals(
            listOf(WikiHighlightRange(3, 11)),
            wikiHighlightRanges(text, listOf(5L to 13L))
        )
        assertEquals("[[nota]]", text.substring(3, 11))
    }

    @Test
    fun highlightRanges_openTriggerYieldsNoRanges() {
        // `[[query` without `]]` produces no byte ranges upstream (the
        // autocomplete panel owns that state), so nothing is highlighted.
        assertEquals(emptyList<WikiHighlightRange>(), wikiHighlightRanges("nota [[query", emptyList()))
    }

    @Test
    fun highlightRanges_emptyTextYieldsNoRanges() {
        assertEquals(emptyList<WikiHighlightRange>(), wikiHighlightRanges("", listOf(0L to 2L)))
        assertEquals(0, byteOffsetToUtf16Index("", 5))
    }

    @Test
    fun highlightRanges_outOfRangeClampsAndInvalidDrops() {
        // Past-the-end clamps to the text length (UTF-16).
        assertEquals(3, byteOffsetToUtf16Index("abc", 99))
        assertEquals(0, byteOffsetToUtf16Index("abc", -4))
        assertEquals(
            listOf(WikiHighlightRange(0, 3)),
            wikiHighlightRanges("abc", listOf(0L to 99L))
        )
        // Empty and reversed ranges are dropped, never crash.
        assertEquals(emptyList<WikiHighlightRange>(), wikiHighlightRanges("abc [[x]]", listOf(5L to 5L)))
        assertEquals(emptyList<WikiHighlightRange>(), wikiHighlightRanges("abc [[x]]", listOf(8L to 4L)))
    }
}
