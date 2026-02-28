use mlm_meta::providers::romanceio::RomanceIo;

// Lightweight test ensuring parse_book_html can be called (keeps tests simple
// without requiring a full mocked HTTP fetcher). More thorough scoring tests
// should be added with a fetcher mock if needed.

#[test]
fn parse_book_html_smoke() {
    let provider = RomanceIo::new();
    let html = include_str!("../../plan/romanceio/book.html");
    let meta = provider.parse_book_html(html).unwrap();
    assert!(!meta.title.is_empty());
    assert!(!meta.authors.is_empty());
}
