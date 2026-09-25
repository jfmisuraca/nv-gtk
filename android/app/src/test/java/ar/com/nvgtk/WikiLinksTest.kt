package ar.com.nvgtk

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
}
