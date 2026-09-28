package ar.com.nvgtk

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pure-JVM tests for the wiki navigation helpers (parity slice 3, T7).
 * Distinct note ids produce distinct `editor/{noteId}` routes, so the
 * NavHost keeps a real back stack (lista→A→B→A→lista) instead of
 * flattening every follow-link onto one single-top entry.
 */
class NavigationTest {

    @Test
    fun editorRoute_embedsNoteId() {
        assertEquals("editor/abc", editorRoute("abc"))
    }

    @Test
    fun editorRoute_distinctIdsGiveDistinctRoutes() {
        assertEquals("editor/A", editorRoute("A"))
        assertEquals("editor/B", editorRoute("B"))
        assert(editorRoute("A") != editorRoute("B"))
    }

    @Test
    fun cleanWikiTarget_trimsTarget() {
        assertEquals("Proyecto", cleanWikiTarget("  Proyecto  "))
    }

    @Test
    fun cleanWikiTarget_blankIsNothingToFollow() {
        assertNull(cleanWikiTarget(""))
        assertNull(cleanWikiTarget("   "))
    }
}
