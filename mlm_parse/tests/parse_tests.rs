use mlm_parse::{clean_name, clean_value, normalize_title};

#[test]
fn test_clean_value_decodes_entities() {
    let s = "Tom &amp; Jerry &quot;Fun&quot;";
    let cleaned = clean_value(s).unwrap();
    assert_eq!(cleaned, "Tom & Jerry \"Fun\"");
}

#[test]
fn test_normalize_title_variants() {
    let s = "The Amazing & Strange Vol. 2 (light novel)";
    let n = normalize_title(s);
    // Expect articles removed, ampersand -> and, volume token removed and lowercased
    assert!(n.starts_with("amazing and strange"));
}

#[test]
fn test_clean_name_initials_and_case() {
    let mut name = "JRR TOLKIEN".to_string();
    clean_name(&mut name).unwrap();
    // JRR should remain as-is (algorithm doesn't split 3-letter initials); TOLKIEN should become Title case
    assert!(name.starts_with("JRR"));
    assert!(name.contains("Tolkien"));

    let mut name2 = "john doe".to_string();
    clean_name(&mut name2).unwrap();
    // short lowercase words should be capitalized at start
    assert!(name2.contains("John"));
}
