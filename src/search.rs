use crate::note::Note;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note_id: String,
    pub score: i64,
}

pub fn search_notes(notes: &[Note], query: &str) -> Vec<String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return notes.iter().map(|n| n.id.clone()).collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored_results: Vec<SearchResult> = Vec::new();

    for note in notes {
        let mut best_score: Option<i64> = None;

        // 1. Check title match (highest priority multiplier)
        if let Some(s) = matcher.fuzzy_match(&note.title, &q) {
            best_score = Some(s * 3);
        } else if note.title.to_lowercase().contains(&q) {
            best_score = Some(500);
        }

        // 2. Check tags match
        for tag in &note.tags {
            if tag.to_lowercase().contains(&q) {
                let tag_score = 400;
                best_score = Some(best_score.map_or(tag_score, |s| s.max(tag_score)));
            }
        }

        // 3. Check content match
        if let Some(s) = matcher.fuzzy_match(&note.content, &q) {
            best_score = Some(best_score.map_or(s, |prev| prev.max(s)));
        } else if note.content.to_lowercase().contains(&q) {
            let content_score = 200;
            best_score = Some(best_score.map_or(content_score, |prev| prev.max(content_score)));
        }

        if let Some(score) = best_score {
            scored_results.push(SearchResult {
                note_id: note.id.clone(),
                score,
            });
        }
    }

    // Sort descending by score
    scored_results.sort_by(|a, b| b.score.cmp(&a.score));

    scored_results.into_iter().map(|r| r.note_id).collect()
}
