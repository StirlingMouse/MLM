pub fn flag_icon(flag: &str) -> Option<(&'static str, &'static str)> {
    match flag {
        "language" => Some(("/assets/icons/language.png", "Crude Language")),
        "violence" => Some(("/assets/icons/hand.png", "Violence")),
        "some_explicit" => Some((
            "/assets/icons/lipssmall.png",
            "Some Sexually Explicit Content",
        )),
        "explicit" => Some(("/assets/icons/flames.png", "Sexually Explicit Content")),
        "abridged" => Some(("/assets/icons/abridged.png", "Abridged")),
        "lgbt" => Some(("/assets/icons/lgbt.png", "LGBT")),
        _ => None,
    }
}
