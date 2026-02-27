use dioxus::prelude::*;

pub fn media_icon_src(mediatype: u8, main_cat: u8) -> Option<&'static str> {
    match (mediatype, main_cat) {
        (1, 1) => Some("/assets/icons/mediatypes/AudiobookF.png"),
        (1, 2) => Some("/assets/icons/mediatypes/AudiobookNF.png"),
        (1, _) => Some("/assets/icons/mediatypes/Audiobook.png"),
        (2, 1) => Some("/assets/icons/mediatypes/EBookF.png"),
        (2, 2) => Some("/assets/icons/mediatypes/EBookNF.png"),
        (2, _) => Some("/assets/icons/mediatypes/EBook.png"),
        (3, 1) => Some("/assets/icons/mediatypes/MusicF.png"),
        (3, 2) => Some("/assets/icons/mediatypes/MusicNF.png"),
        (3, _) => Some("/assets/icons/mediatypes/Music.png"),
        (4, 1) => Some("/assets/icons/mediatypes/RadioF.png"),
        (4, 2) => Some("/assets/icons/mediatypes/RadioNF.png"),
        (4, _) => Some("/assets/icons/mediatypes/Radio.png"),
        (5, 1) => Some("/assets/icons/mediatypes/MangaF.png"),
        (5, 2) => Some("/assets/icons/mediatypes/MangaNF.png"),
        (5, _) => Some("/assets/icons/mediatypes/Manga.png"),
        (6, 1) => Some("/assets/icons/mediatypes/ComicsF.png"),
        (6, 2) => Some("/assets/icons/mediatypes/ComicsNF.png"),
        (6, _) => Some("/assets/icons/mediatypes/Comics.png"),
        (7, 1) => Some("/assets/icons/mediatypes/PeriodicalsF.png"),
        (7, 2) => Some("/assets/icons/mediatypes/PeriodicalsNF.png"),
        (7, _) => Some("/assets/icons/mediatypes/Periodicals.png"),
        (8, 1) => Some("/assets/icons/mediatypes/PeriodicalsAudioF.png"),
        (8, 2) => Some("/assets/icons/mediatypes/PeriodicalsAudioNF.png"),
        (8, _) => Some("/assets/icons/mediatypes/PeriodicalsAudio.png"),
        _ => None,
    }
}

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

#[component]
pub fn CategoryPills(categories: Vec<String>, old_category: Option<String>) -> Element {
    if categories.is_empty() && old_category.is_none() {
        return rsx! {};
    }

    rsx! {
        div { class: "CategoryPills",
            if let Some(ref old_category) = old_category {
                span { class: "CategoryPill old", "{old_category}" }
            }
            for category in &categories {
                if old_category.as_ref() != Some(category) {
                    span { class: "CategoryPill", "{category}" }
                }
            }
        }
    }
}

#[component]
pub fn TorrentIcons(
    vip: bool,
    personal_freeleech: bool,
    free: bool,
    flags: Vec<String>,
) -> Element {
    rsx! {
        div { class: "TorrentIcons", grid_area: "icons",
            if vip {
                img { src: "/assets/icons/vip.png", alt: "VIP", title: "VIP" }
            } else if personal_freeleech {
                img {
                    src: "/assets/icons/freedownload.png",
                    alt: "Personal Freeleech",
                    title: "Personal Freeleech",
                    style: "filter:hue-rotate(180deg)",
                }
            } else if free {
                img {
                    src: "/assets/icons/freedownload.png",
                    alt: "Freeleech",
                    title: "Freeleech",
                }
            }
            for flag in &flags {
                if let Some((src, title)) = flag_icon(flag) {
                    img {
                        class: "flag",
                        src: "{src}",
                        alt: "{title}",
                        title: "{title}",
                    }
                }
            }
        }
    }
}
