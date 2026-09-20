use crate::note::Note;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note_id: String,
    pub score: i64,
}

/// Densidad máxima admisible de un match fuzzy sobre contenido: el tramo que
/// ocupan las letras coincide (#chars) no puede superar `query.len() * MAX_SPAN_MULT`.
/// Sin esto, un query como "hoy" matchea h…o…y dispersos a 60+ caracteres de
/// distancia (score bajo, span enorme) y la nota aparece SIN contener el texto.
/// Calibración en el módulo `calibration` (comandos que quedan como evidencia).
const MAX_SPAN_MULT: i64 = 3;

pub fn search_notes(notes: &[Note], q_raw: &str) -> Vec<String> {
    let q = q_raw.trim().to_lowercase();
    if q.is_empty() {
        return notes.iter().map(|n| n.id.clone()).collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored_results: Vec<SearchResult> = Vec::new();

    for note in notes {
        let mut best_score: Option<i64> = None;

        // 1. Title: fuzzy denso (títulos cortos) o substring
        if let Some(s) = matcher.fuzzy_match(&note.title, &q) {
            best_score = Some(s * 3);
        } else if note.title.to_lowercase().contains(&q) {
            best_score = Some(500);
        }

        // 2. Tags
        for tag in &note.tags {
            if tag.to_lowercase().contains(&q) {
                let tag_score = 400;
                best_score = Some(best_score.map_or(tag_score, |p| p.max(tag_score)));
            }
        }

        // 3. Content: solo coincidencias DENSA — las letras del query deben
        //    caer en un tramo compacto. Un match disperso (span > 3×query) es
        //    ruido: la nota no contiene el texto real.
        if let Some((s, indices)) = matcher.fuzzy_indices(&note.content, &q) {
            let span = (indices.last().unwrap() - indices.first().unwrap() + 1) as i64;
            if span <= q.len() as i64 * MAX_SPAN_MULT {
                best_score = Some(best_score.map_or(s, |p| p.max(s)));
            } else if note.content.to_lowercase().contains(&q) {
                let content_score = 200;
                best_score = Some(best_score.map_or(content_score, |p| p.max(content_score)));
            }
        } else if note.content.to_lowercase().contains(&q) {
            let content_score = 200;
            best_score = Some(best_score.map_or(content_score, |p| p.max(content_score)));
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

#[cfg(test)]
mod calibration {
    use fuzzy_matcher::FuzzyMatcher;
    use fuzzy_matcher::skim::SkimMatcherV2;

    // Calibración retenida DE PROPÓSITO: documenta el rango de scores/spans
    // para fijar MAX_SPAN_MULT. No es una prueba de correctitud; ver con
    // `cargo test calibration -- --nocapture`.
    #[test]
    fn print_reference_spans() {
        let m = SkimMatcherV2::default();
        let cases: &[(&str, &str)] = &[
            // Densos (deben pasar)
            ("sumar", "quiero sumar dos números en el panel sumador"),
            ("esc", "escape cancela el overlay"),
            ("proyecto", "Este es mi proyecto de notas en nv-gtk"),
            // Dispersos (el bug del usuario: no deben pasar)
            ("hoy", "hay una nota antigua que menciona ojo y yo"),
            ("nvgtk", "nuevas ventanas giran tras el teclado"),
        ];
        for (q, content) in cases {
            let s = m.fuzzy_match(content, q);
            let idx = m.fuzzy_indices(content, q);
            let span = idx.as_ref().map(|(_, v)| {
                v.last().map(|l| *l as usize - v[0] as usize + 1)
            });
            println!("q={q:<8} span={span:?} score={s:?} content={content:<55}");
        }
    }
}
