//! Wiki-link logic shared by desktop and Android (parity slice 3).
//!
//! Moved verbatim-semantics from `src/wiki_link.rs` (extraction) and
//! `src/wiki_autocomplete.rs` (fuzzy matching, display title, preview,
//! ranking) so both platforms run the same code: desktop calls these
//! functions directly, Android reaches them through `ffi.rs` (`NvStorage`
//! methods + free functions). No storage behavior lives here — callers
//! supply the notes to rank, and `ffi.rs` adapts the results to UniFFI
//! records.
//!
//! Matching rules (frozen from the desktop implementation):
//! - A link is `[[<one or more non-']' chars>]]`; the target is trimmed.
//! - A candidate matches when every query char appears in the display title
//!   in order (subsequence, case-insensitive); fewer gaps rank first.
//! - The display title is the first non-blank trimmed content line, with
//!   `#`/markdown kept verbatim.

use std::sync::OnceLock;

use regex::Regex;

/// Closed `[[target]]` hit with byte offsets over the source text, exactly
/// like the desktop `WikiLink` (GTK converts them to char offsets itself).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLinkHit {
    pub target: String,
    pub start: usize,
    pub end: usize,
}

/// Ranked autocomplete candidate: note identity plus what the UI shows.
/// `tags` stays a plain list; each platform formats it (`[a b]`, `#a`, …).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiCandidate {
    pub id: String,
    pub display_title: String,
    pub tags: Vec<String>,
    pub preview: String,
}

/// Minimal read-only view of a note needed for ranking. Both the desktop
/// `Note` and the FFI snapshot map into this, so there is a single ranking
/// implementation.
#[derive(Debug, Clone)]
pub struct WikiSourceNote {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
}

/// Fallback display title when a note has no non-blank line. Shared by both
/// platforms (desktop hardcodes it, Android `R.string.note_empty_title` is
/// the same text); kept here so candidates never carry an empty title.
pub const EMPTY_DISPLAY_TITLE: &str = "(nota vacía)";

/// How many candidates `suggest_wiki_candidates` returns at most (desktop
/// panel shows 8 rows).
pub const MAX_WIKI_SUGGESTIONS: usize = 8;

/// How many leading content lines form a candidate preview (desktop panel).
pub const WIKI_PREVIEW_LINES: usize = 4;

fn wiki_link_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("wiki-link regex"))
}

/// Extract every closed `[[target]]` from `text`, in document order.
/// Byte offsets, target trimmed — identical to the old desktop extractor,
/// including keeping empty-after-trim targets (`[[ ]]`): callers that
/// navigate ignore blank targets themselves.
pub fn extract_wiki_links(text: &str) -> Vec<WikiLinkHit> {
    let re = wiki_link_regex();
    let mut links = Vec::new();

    for cap in re.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let target = cap[1].trim().to_string();
            links.push(WikiLinkHit {
                target,
                start: m.start(),
                end: m.end(),
            });
        }
    }

    links
}

/// Trigger rule for the `[[query` autocomplete panel: given the current line
/// up to the cursor, return the pending query when the line contains an
/// unclosed `[[` (the last one wins), `None` when there is none or it is
/// already closed by `]]`. Mirrors the desktop line scan exactly.
pub fn open_wiki_query(line_before_cursor: &str) -> Option<String> {
    line_before_cursor.rfind("[[").and_then(|idx| {
        let after = &line_before_cursor[idx + 2..];
        if after.contains("]]") {
            None
        } else {
            Some(after.to_string())
        }
    })
}

/// Closed link under a cursor given in *chars* (not bytes), for platforms
/// without GTK text iters. Returns `None` for out-of-range cursors.
/// Byte offsets in the hit stay compatible with `extract_wiki_links`.
pub fn link_at_cursor(text: &str, cursor_chars: usize) -> Option<WikiLinkHit> {
    let total_chars = text.chars().count();
    if cursor_chars > total_chars {
        return None;
    }
    let total_bytes = text.len();
    let start_char_of = |byte: usize| text[..byte.min(total_bytes)].chars().count();
    // Edges count as "on" the link, like the desktop `>= start && <= end`
    // check after its byte→char conversion.
    extract_wiki_links(text)
        .into_iter()
        .find(|link| {
            cursor_chars >= start_char_of(link.start) && cursor_chars <= start_char_of(link.end)
        })
}

/// First non-blank trimmed content line (`None` when there is none).
/// Desktop and Android both showed this as the note title in wiki UI.
pub fn display_title_for(content: &str) -> Option<String> {
    content
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

/// Same as `display_title_for` with the shared `(nota vacía)` fallback, so
/// FFI candidates always carry a visible title.
pub fn display_title_or_fallback(content: &str) -> String {
    display_title_for(content).unwrap_or_else(|| EMPTY_DISPLAY_TITLE.to_string())
}

/// Preview shown next to a candidate: the first 4 content lines joined.
pub fn preview_for(content: &str) -> String {
    content
        .lines()
        .take(WIKI_PREVIEW_LINES)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Fuzzy subsequence score: every char of `query` must appear in `target`
/// in order (case-insensitive). `None` when it does not match; lower
/// `Some(score)` ranks first (gap + late-start penalties). Moved unchanged
/// from the desktop autocomplete.
pub fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();

    let mut score: i32 = 0;
    let mut last_match: Option<usize> = None;
    let mut q_chars = query_lower.chars().peekable();

    for (ti, tc) in target_lower.chars().enumerate() {
        if let Some(&qc) = q_chars.peek() {
            if tc == qc {
                match last_match {
                    Some(last) => score += (ti - last - 1) as i32,
                    None => score += ti as i32,
                }
                last_match = Some(ti);
                q_chars.next();
            }
        } else {
            break;
        }
    }

    if q_chars.peek().is_some() {
        None
    } else {
        Some(score)
    }
}

/// Rank notes against `query` for the autocomplete panel: fuzzy-match on
/// the display title, sort by `(score, display_title)`, keep the first 8.
/// Empty queries match everything (score 0), so the panel lists notes
/// alphabetically — same as desktop.
pub fn suggest_wiki_candidates(query: &str, notes: &[WikiSourceNote]) -> Vec<WikiCandidate> {
    let mut scored: Vec<(WikiCandidate, i32)> = notes
        .iter()
        .filter_map(|note| {
            let display_title = display_title_or_fallback(&note.content);
            fuzzy_score(query, &display_title).map(|score| {
                (
                    WikiCandidate {
                        id: note.id.clone(),
                        display_title,
                        tags: note.tags.clone(),
                        preview: preview_for(&note.content),
                    },
                    score,
                )
            })
        })
        .collect();

    scored.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| a.0.display_title.cmp(&b.0.display_title))
    });
    scored.truncate(MAX_WIKI_SUGGESTIONS);
    scored.into_iter().map(|(candidate, _)| candidate).collect()
}

/// Resolve a `[[target]]` to the note id whose display title equals the
/// trimmed target (case-insensitive Unicode lowercase on both sides).
/// First hit in slice order wins; storage lists newest-first, so the most
/// recently modified note wins ties. `None` for blank targets or no match —
/// the caller then creates a note with the target as content (desktop flow).
pub fn resolve_wiki_target(target: &str, notes: &[WikiSourceNote]) -> Option<String> {
    let want = target.trim();
    if want.is_empty() {
        return None;
    }
    let want_lower = want.to_lowercase();
    notes.iter().find_map(|note| {
        display_title_for(&note.content).and_then(|title| {
            if title.to_lowercase() == want_lower {
                Some(note.id.clone())
            } else {
                None
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, content: &str) -> WikiSourceNote {
        WikiSourceNote {
            id: id.to_string(),
            content: content.to_string(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn extract_finds_closed_links_with_byte_offsets() {
        let text = "ver [[nota uno]] y [[dos]]";
        let links = extract_wiki_links(text);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "nota uno");
        assert_eq!(&text[links[0].start..links[0].end], "[[nota uno]]");
        assert_eq!(links[1].target, "dos");
        assert_eq!(&text[links[1].start..links[1].end], "[[dos]]");
    }

    #[test]
    fn extract_ignores_unclosed_and_trims_target() {
        assert!(extract_wiki_links("pendiente [[sin cerrar").is_empty());
        let links = extract_wiki_links("[[  con espacios  ]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "con espacios");
    }

    #[test]
    fn extract_edge_cases_match_desktop_regex() {
        // `]]` inside breaks the match like the old regex: no link at all.
        assert!(extract_wiki_links("[[a]b]]").is_empty());
        // Empty-after-trim targets are kept (callers filter blanks).
        let links = extract_wiki_links("[[ ]]");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "");
        // Multiline text: links on any line are found.
        let links = extract_wiki_links("línea uno\n[[meta]]\nfin");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "meta");
    }

    #[test]
    fn extract_unicode_targets_keep_byte_offsets() {
        let text = "áé [[título ñ]] fin";
        let links = extract_wiki_links(text);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "título ñ");
        assert_eq!(&text[links[0].start..links[0].end], "[[título ñ]]");
    }

    #[test]
    fn open_query_detects_unclosed_trigger_only() {
        assert_eq!(
            open_wiki_query("una nota [[proy"),
            Some("proy".to_string())
        );
        assert_eq!(open_wiki_query("una nota [[ "), Some(" ".to_string()));
        assert_eq!(open_wiki_query("recién [[abierto"), Some("abierto".to_string()));
        // Closed links never trigger.
        assert_eq!(open_wiki_query("ver [[nota]]"), None);
        assert_eq!(open_wiki_query("ver [[nota]] y texto"), None);
        // No trigger without brackets.
        assert_eq!(open_wiki_query("texto plano"), None);
        // Last unclosed `[[` wins; an earlier closed one is ignored.
        assert_eq!(
            open_wiki_query("[[cerrado]] y [[nue"),
            Some("nue".to_string())
        );
    }

    #[test]
    fn link_at_cursor_finds_link_under_char_cursor() {
        let text = "ver [[nota]] fin";
        let link_start_chars = text[..text.find("[[nota]]").unwrap()].chars().count();
        // Cursor on the brackets and inside counts as "on" the link.
        assert_eq!(
            link_at_cursor(text, link_start_chars).map(|l| l.target),
            Some("nota".to_string())
        );
        assert_eq!(
            link_at_cursor(text, link_start_chars + 3).map(|l| l.target),
            Some("nota".to_string())
        );
        assert_eq!(
            link_at_cursor(text, link_start_chars + 8).map(|l| l.target),
            Some("nota".to_string())
        );
        // Outside: nothing.
        assert_eq!(link_at_cursor(text, 0), None);
        assert_eq!(link_at_cursor(text, text.chars().count()), None);
        // Out of range: nothing, never panics.
        assert_eq!(link_at_cursor(text, text.chars().count() + 5), None);
        assert_eq!(link_at_cursor("", 0), None);
    }

    #[test]
    fn link_at_cursor_handles_unicode_before_link() {
        let text = "áéí [[meta]]";
        let byte_start = text.find("[[meta]]").unwrap();
        let char_start = text[..byte_start].chars().count();
        // Byte and char offsets differ here (áéí are 2 bytes each); the
        // char-based cursor must still land on the link.
        assert!(byte_start != char_start);
        assert_eq!(
            link_at_cursor(text, char_start + 2).map(|l| l.target),
            Some("meta".to_string())
        );
    }

    #[test]
    fn fuzzy_score_matches_desktop_semantics() {
        assert_eq!(fuzzy_score("", "cualquiera"), Some(0));
        // Contiguous early match scores better than gappy/late one.
        let exact = fuzzy_score("abc", "abc").unwrap();
        let gappy = fuzzy_score("abc", "xaxbxc").unwrap();
        assert!(exact < gappy);
        // Case-insensitive subsequence.
        assert!(fuzzy_score("PY", "proyecto").is_some());
        // Missing char: no match.
        assert_eq!(fuzzy_score("xyz", "abc"), None);
    }

    #[test]
    fn suggest_ranks_sorts_and_truncates_like_desktop() {
        let notes = vec![
            source("a", "proyecto alfa"),
            source("b", "beta proyecto"),
            source("c", "sin relación"),
        ];
        let got = suggest_wiki_candidates("proy", &notes);
        assert_eq!(got.len(), 2);
        // "proyecto alfa" starts at 0, "beta proyecto" later: same order.
        assert_eq!(got[0].id, "a");
        assert_eq!(got[1].id, "b");
        assert_eq!(got[0].display_title, "proyecto alfa");
        assert_eq!(got[0].preview, "proyecto alfa");

        // Empty query lists everything alphabetically by display title.
        let got = suggest_wiki_candidates("", &notes);
        assert_eq!(got.len(), 3);
        let titles: Vec<_> = got.iter().map(|c| c.display_title.as_str()).collect();
        let mut sorted = titles.clone();
        sorted.sort();
        assert_eq!(titles, sorted);
    }

    #[test]
    fn suggest_caps_at_eight_with_preview_and_tags() {
        let notes: Vec<_> = (0..12)
            .map(|i| WikiSourceNote {
                id: format!("n{i}"),
                content: format!("nota {i}\nlínea dos\nlínea tres\nlínea cuatro\nlínea cinco"),
                tags: vec!["t".to_string()],
            })
            .collect();
        let got = suggest_wiki_candidates("", &notes);
        assert_eq!(got.len(), MAX_WIKI_SUGGESTIONS);
        assert_eq!(got[0].preview, "nota 0\nlínea dos\nlínea tres\nlínea cuatro");
        assert_eq!(got[0].tags, vec!["t".to_string()]);
    }

    #[test]
    fn display_title_skips_blanks_and_falls_back() {
        assert_eq!(
            display_title_for("\n  \nHola\nmundo"),
            Some("Hola".to_string())
        );
        assert_eq!(display_title_for("  \n\t\n "), None);
        // Markdown kept verbatim, like both UIs show it.
        assert_eq!(
            display_title_for("  # Título"),
            Some("# Título".to_string())
        );
        assert_eq!(display_title_or_fallback(""), EMPTY_DISPLAY_TITLE);
    }

    #[test]
    fn resolve_matches_display_title_case_insensitively() {
        let notes = vec![
            source("old", "Proyecto Alfa\ncuerpo"),
            source("new", "beta"),
        ];
        assert_eq!(
            resolve_wiki_target("  proyecto alfa ", &notes),
            Some("old".to_string())
        );
        assert_eq!(
            resolve_wiki_target("BETA", &notes),
            Some("new".to_string())
        );
        assert_eq!(resolve_wiki_target("inexistente", &notes), None);
        assert_eq!(resolve_wiki_target("   ", &notes), None);
        // Notes without content never resolve (fallback is UI-only).
        assert_eq!(
            resolve_wiki_target("(nota vacía)", &[source("e", "")]),
            None
        );
    }
}
