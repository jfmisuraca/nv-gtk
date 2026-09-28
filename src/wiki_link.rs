//! Desktop view of wiki-links. The logic lives in `nv-core` (`nv_core::wiki`)
//! so Android runs the same code through FFI; this module only adapts the
//! core hits to the desktop `WikiLink` (offsets as `usize` for GTK indexing).
use nv_core::wiki;

#[derive(Debug, Clone, PartialEq)]
pub struct WikiLink {
    pub target: String,
    pub start: usize,
    pub end: usize,
}

pub fn extract_wiki_links(text: &str) -> Vec<WikiLink> {
    wiki::extract_wiki_links(text)
        .into_iter()
        .map(|hit| WikiLink {
            target: hit.target,
            start: hit.start,
            end: hit.end,
        })
        .collect()
}
