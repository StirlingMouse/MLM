use maplit::hashset;
use once_cell::sync::Lazy;

pub static AMMONIA: Lazy<ammonia::Builder<'static>> = Lazy::new(|| {
    let mut builder = ammonia::Builder::default();
    builder
        .url_schemes(hashset!["http", "https"])
        .add_generic_attributes(hashset!["style"])
        .filter_style_properties(hashset![
            "font-style",
            "font-weight",
            "text-align",
            "text-decoration"
        ])
        .attribute_filter(|element, attribute, value| match (element, attribute) {
            ("a", "href") => None,
            ("img", "src") => value.starts_with("http").then_some(value.into()),
            _ => Some(value.into()),
        });
    builder
});
