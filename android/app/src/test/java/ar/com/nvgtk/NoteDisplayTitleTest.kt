package ar.com.nvgtk

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM tests for [displayTitle]/[displayRemainder] (desktop parity:
 * first non-blank trimmed line, verbatim `#`, untruncated long lines).
 * No Robolectric: the helpers take [emptyTitle] as a parameter instead of
 * reading `R.string.note_empty_title`, which needs the Android framework.
 */
class NoteDisplayTitleTest {

    companion object {
        private const val EMPTY = "(nota vacía)"
    }

    @Test
    fun displayTitle_emptyContent_returnsFallback() {
        assertEquals(EMPTY, displayTitle("", EMPTY))
    }

    @Test
    fun displayTitle_allBlankContent_returnsFallback() {
        assertEquals(EMPTY, displayTitle("   \n\t\n  ", EMPTY))
        assertEquals("", displayRemainder("   \n\t\n  "))
    }

    @Test
    fun displayTitle_leadingBlankLines_skipsToFirstContent() {
        assertEquals("Hola", displayTitle("\n\n  Hola\nmundo", EMPTY))
    }

    @Test
    fun displayTitle_surroundingWhitespace_isTrimmed() {
        assertEquals("Hola mundo", displayTitle("   Hola mundo  ", EMPTY))
    }

    @Test
    fun displayTitle_markdownHeading_shownVerbatim() {
        assertEquals("# Título", displayTitle("  # Título", EMPTY))
    }

    @Test
    fun displayTitle_longLine_returnedUntouched() {
        val long = "x".repeat(500)
        assertEquals(long, displayTitle(long, EMPTY))
        assertEquals(500, displayTitle(long, EMPTY).length)
    }

    @Test
    fun displayRemainder_titleOnly_isEmpty() {
        assertEquals("", displayRemainder("Solo título"))
        // Trailing blank lines keep the remainder blank (hidden by isBlank
        // at the call site) without needing a trim in data.
        assertTrue(displayRemainder("\n  Solo título  \n   \n").isBlank())
    }

    @Test
    fun displayRemainder_withBody_skipsTitleLineOnly() {
        assertEquals("segunda\ntercera", displayRemainder("primera\nsegunda\ntercera"))
        assertEquals("cuerpo", displayRemainder("\n\n  título  \ncuerpo"))
        assertTrue(displayRemainder("título\n   \n").isBlank())
    }
}
