use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub struct WikiLink {
    pub target: String,
    pub start: usize,
    pub end: usize,
}

pub fn extract_wiki_links(text: &str) -> Vec<WikiLink> {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let mut links = Vec::new();

    for cap in re.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let target = cap[1].trim().to_string();
            links.push(WikiLink {
                target,
                start: m.start(),
                end: m.end(),
            });
        }
    }

    links
}
