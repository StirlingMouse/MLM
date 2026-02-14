use mlm_parse::{clean_name, normalize_title};

pub use anyhow;
pub use tracing::{Level, debug, enabled, trace};

/// Search query with optional author. Providers can decide how to use these fields.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub title: String,
    pub author: Option<String>,
}

impl SearchQuery {
    pub fn new(title: String, author: Option<String>) -> Self {
        Self { title, author }
    }

    /// Build a combined search string for providers that use a single query string.
    pub fn to_combined_string(&self) -> String {
        match &self.author {
            Some(author) if !self.title.is_empty() && !author.is_empty() => {
                format!("{} {}", self.title, author)
            }
            _ if !self.title.is_empty() => self.title.clone(),
            _ => String::new(),
        }
    }
}

/// Build SearchQuery with author included
pub fn query_with_author(title: &str, authors: &[String]) -> SearchQuery {
    let author = authors.first().cloned();
    SearchQuery::new(title.to_string(), author)
}

/// Build SearchQuery without author (title-only search)
pub fn query_title_only(title: &str) -> SearchQuery {
    SearchQuery::new(title.to_string(), None)
}

/// Normalized string similarity 0.0..1.0
pub fn token_similarity(a: &str, b: &str) -> f64 {
    strsim::normalized_levenshtein(a, b)
}

/// Normalize author names (clean and lowercase)
pub fn normalize_authors(auths: &[String]) -> Vec<String> {
    auths
        .iter()
        .map(|a| {
            let mut s = a.clone();
            let _ = clean_name(&mut s);
            s.to_lowercase()
        })
        .collect()
}

/// Score a candidate by title and author similarity. Candidate title and
/// candidate authors are provided directly as strings (the caller extracts
/// them from JSON). The query title/authors are the original query values.
pub fn score_candidate(
    cand_title: Option<&str>,
    cand_auths: &[String],
    q_title: &Option<String>,
    q_auths: &[String],
) -> f64 {
    let q_title_norm = q_title.as_ref().map(|t| normalize_title(t));

    let mut title_score = 0.0f64;
    let mut title_exact = false;
    if let Some(qt_norm) = q_title_norm.as_ref()
        && let Some(ct) = cand_title
    {
        let cand = normalize_title(ct);
        if cand == *qt_norm {
            title_score = 1.0;
            title_exact = true;
        } else if cand.contains(qt_norm.as_str()) || qt_norm.contains(cand.as_str()) {
            title_score = 0.9;
        } else {
            title_score = token_similarity(&cand, qt_norm);
        }
    }

    let mut author_score = 0.0f64;
    let mut authors_match = false;
    if !q_auths.is_empty() {
        let q_auths_norm = normalize_authors(q_auths);
        let mut best = 0.0f64;
        for a in cand_auths {
            let mut n = a.clone();
            let _ = clean_name(&mut n);
            let n = n.to_lowercase();
            for qa in &q_auths_norm {
                if n.contains(qa) || qa.contains(&n) {
                    best = best.max(1.0);
                    authors_match = true;
                } else {
                    best = best.max(token_similarity(&n, qa));
                }
            }
        }
        author_score = best;
    }

    // Penalize heavily if title is not exact AND no author match
    // This prevents "Not the Boss of the Year" from matching "Boss of the Year"
    // when authors don't match
    if !title_exact && !authors_match && q_title_norm.is_some() && !q_auths.is_empty() {
        return 0.0;
    }

    if q_title_norm.is_some() && !q_auths.is_empty() {
        0.7 * title_score + 0.3 * author_score
    } else if q_title_norm.is_some() {
        title_score
    } else {
        author_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlm_parse::normalize_title;

    #[test]
    fn test_token_similarity() {
        assert!(token_similarity("great adventure", "great adventure") > 0.999);
        assert!(token_similarity("great adventure", "great adventures") > 0.8);
        assert!(token_similarity("great adventure", "completely different") < 0.3);
    }

    #[test]
    fn test_score_candidate_title_pref() {
        let q_title = Some(normalize_title("The Great Adventure"));
        let q_auths: Vec<String> = vec![];

        let cand_exact_title = Some("The Great Adventure");
        let cand_sim_title = Some("Great Adventure");
        let cand_auths_exact: Vec<String> = vec!["Alice".to_string()];
        let cand_auths_sim: Vec<String> = vec!["Bob Smith".to_string()];

        let s_exact = score_candidate(cand_exact_title, &cand_auths_exact, &q_title, &q_auths);
        let s_sim = score_candidate(cand_sim_title, &cand_auths_sim, &q_title, &q_auths);
        assert!(s_exact >= s_sim, "expected exact title to score >= similar");
    }

    #[test]
    fn test_score_candidate_author_influence() {
        let q_title = Some(normalize_title("Great Adventure"));
        let q_auths: Vec<String> = vec!["bob smith".to_string()];

        let cand_title_only = Some("Great Adventure");
        let cand_both = Some("Great Adventur");
        let cand_auths_title_only: Vec<String> = vec!["Alice".to_string()];
        let cand_auths_both: Vec<String> = vec!["Bob Smith".to_string()];

        let s_title_only =
            score_candidate(cand_title_only, &cand_auths_title_only, &q_title, &q_auths);
        let s_both = score_candidate(cand_both, &cand_auths_both, &q_title, &q_auths);
        assert!(
            s_both > s_title_only,
            "expected candidate with matching author to score higher"
        );
    }
}
