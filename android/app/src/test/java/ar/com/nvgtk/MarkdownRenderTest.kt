package ar.com.nvgtk

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM tests for [renderMarkdownAnnotated] (desktop parity: headings,
 * bold, italic, code, lists, links as `text (url)`). Colors pass in as
 * constants — no Android framework needed.
 */
class MarkdownRenderTest {

    companion object {
        private val LINK = Color(0xFF0000FF)
        private val CODE_BG = Color(0xFFEEEEEE)
    }

    @Test
    fun renderMarkdown_basicSubset_rendersTextWithoutMarkers() {
        val out = renderMarkdownAnnotated(
            "# Título\n\nHola **negrita** y *itálica* con `código`.\n\n- uno\n- dos\n",
            LINK,
            CODE_BG
        ).text
        assertTrue(out.contains("Título"))
        assertTrue(out.contains("negrita"))
        assertTrue(!out.contains("**"))
        assertTrue(!out.contains("#"))
        assertTrue(out.contains("• uno"))
        assertTrue(out.contains("• dos"))
    }

    @Test
    fun renderMarkdown_link_showsUrlInParens() {
        val out = renderMarkdownAnnotated("[enlace](https://ejemplo.com)", LINK, CODE_BG).text
        assertEquals("enlace (https://ejemplo.com)", out)
    }

    @Test
    fun renderMarkdown_orderedListAndCodeBlock() {
        val out = renderMarkdownAnnotated(
            "1. primero\n2. segundo\n\n```\nlet x = 1;\n```\n",
            LINK,
            CODE_BG
        ).text
        assertTrue(out.contains("1. primero"))
        assertTrue(out.contains("2. segundo"))
        assertTrue(out.contains("let x = 1;"))
        assertTrue(!out.contains("```"))
    }

    @Test
    fun renderMarkdown_ruleAndQuote() {
        val out = renderMarkdownAnnotated("---\n\n> cita\n", LINK, CODE_BG).text
        assertTrue(out.contains("───"))
        assertTrue(out.contains("> cita"))
    }

    @Test
    fun renderMarkdown_emptyInput_staysEmpty() {
        assertEquals("", renderMarkdownAnnotated("", LINK, CODE_BG).text)
    }
}
