use mlm_meta::providers::RomanceIo;

fn resolve_plan_file(rel: &str) -> std::io::Result<std::path::PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join(rel);
        if candidate.exists() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("could not find {}", rel),
    ))
}

#[test]
fn parse_book_html_extracts_categories_and_tags() {
    let p = resolve_plan_file("plan/romanceio/book.html").expect("plan file");
    let contents = std::fs::read_to_string(p).expect("read plan file");

    let prov = RomanceIo::new();
    let meta = prov.parse_book_html(&contents).expect("parse book html");

    // basic metadata
    assert!(meta.title.to_lowercase().contains("of ink and alchemy"));
    assert!(
        meta.authors
            .iter()
            .any(|a| a.to_lowercase().contains("sloane"))
    );

    // categories should include contemporary and dark romance (derived from topics)
    assert!(meta.categories.iter().any(|c| c == "contemporary"));
    assert!(meta.categories.iter().any(|c| c == "dark romance"));

    // tags should include some of the romance-specific tropes
    let tags = meta.tags.join(",");
    assert!(tags.contains("age difference") || tags.contains("age gap"));
    assert!(tags.contains("friends to lovers") || tags.contains("friends to lovers"));
}
