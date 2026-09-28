package ar.com.nvgtk

import androidx.navigation.navOptions
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Regression tests for wiki navigation (parity slice 3, T7 reopened).
 *
 * The first T7 attempt tested only string helpers (`editorRoute`,
 * `cleanWikiTarget`) while the device still flattened lista→A→B to
 * B→atrás→lista. These tests lock what the REAL `navigate()` calls use:
 * the routing decision (`editorFollowRoute`, applied verbatim by
 * `followWikiLink`) and the exact `NavOptions` passed to every editor
 * `navigate()` (`::applyEditorNavOptions` — the same function reference
 * the call sites pass, not a copy).
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

    @Test
    fun editorFollowRoute_pushesDistinctRouteForAnotherNote() {
        // lista→A→(link)→B must push editor/B on top of editor/A so that
        // back walks B→A→lista.
        assertEquals("editor/B", editorFollowRoute("A", "B"))
    }

    @Test
    fun editorFollowRoute_selfLinkStaysInPlace() {
        assertNull(editorFollowRoute("A", "A"))
    }

    @Test
    fun editorFollowRoute_noCurrentEntryPushes() {
        assertEquals("editor/B", editorFollowRoute(null, "B"))
    }

    @Test
    fun editorNavOptions_pushWithoutFlattening() {
        // The exact options every editor navigate() call passes: a plain
        // push (no single-top reuse, no popUpTo dropping A, no restore).
        val options = navOptions(::applyEditorNavOptions)
        assertFalse(options.shouldLaunchSingleTop())
        assertFalse(options.shouldRestoreState())
        assertEquals(-1, options.popUpToId)
        assertFalse(options.isPopUpToInclusive())
    }
}
