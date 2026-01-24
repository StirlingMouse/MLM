use anyhow::{Error, Result};
use mlm_db::{MediaType, OldCategory, TorrentMeta};
use mlm_parse::{SERIES_CLEANUP, TITLE_CLEANUP, clean_name, clean_value, parse_edition};

#[derive(thiserror::Error, Debug)]
pub enum MetaError {
    #[error("{0}")]
    UnknownMediaType(String),
    #[error("Unknown category: {0}")]
    UnknownCat(u8),
    #[error("Unknown old category: {0} ({1})")]
    UnknownOldCat(String, u64),
    #[error("Unknown language id {0}, code: {1}")]
    UnknownLanguage(u8, String),
    #[error("{0}")]
    InvalidSize(String),
    #[error("{0}")]
    InvalidSeries(&'static str),
    #[error("Invalid added date: {0}")]
    InvalidAdded(String),
    #[error("Invalid vip_expiry: {0}")]
    InvalidVipExpiry(u64),
    #[error("Unknown error: {0}")]
    Other(#[from] Error),
}

pub fn clean_meta(mut meta: TorrentMeta, tags: &str) -> Result<TorrentMeta> {
    // A large amount of audiobook torrents have been incorrectly set to ebook
    if meta.media_type == MediaType::Ebook
        && let Some(OldCategory::Audio(_)) = meta.cat
    {
        meta.media_type = MediaType::Audiobook;
    }
    for author in &mut meta.authors {
        clean_name(author)?;
    }
    for narrator in &mut meta.narrators {
        clean_name(narrator)?;
    }
    for series in &mut meta.series {
        series.name = SERIES_CLEANUP
            .replace_all(&clean_value(&series.name)?, "")
            .to_string();
    }

    let (title, edition) = parse_edition(&meta.title, tags);
    meta.title = title;
    meta.edition = edition;

    // Apparently authors is getting removed from periodicals
    if meta.media_type != MediaType::PeriodicalEbook
        && meta.media_type != MediaType::PeriodicalAudiobook
        && meta.authors.len() == 1
        && let Some(author) = meta.authors.first()
    {
        if let Some(title) = meta.title.strip_suffix(author) {
            if let Some(title) = title
                .strip_suffix(" by ")
                .or_else(|| title.strip_suffix(" - "))
            {
                meta.title = title.trim().to_string();
            }
        } else if let Some(title) = meta.title.strip_prefix(author)
            && let Some(title) = title.strip_prefix(" - ")
        {
            meta.title = title.trim().to_string();
        }
    }

    meta.title = TITLE_CLEANUP
        .replace_all(&meta.title, "")
        .trim()
        .to_string();

    Ok(meta)
}

// #[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// pub enum Category {
//     Action,
//     Art,
//     Biographical,
//     Business,
//     Comedy,
//     CompleteEditionsMusic,
//     Computer,
//     Crafts,
//     Crime,
//     Dramatization,
//     Education,
//     FactualNews,
//     Fantasy,
//     Food,
//     Guitar,
//     Health,
//     Historical,
//     Home,
//     Horror,
//     Humor,
//     IndividualSheet,
//     Instructional,
//     Juvenile,
//     Language,
//     Lgbt,
//     LickLibraryLTP,
//     LickLibraryTechniques,
//     LiteraryClassics,
//     LitRPG,
//     Math,
//     Medicine,
//     Music,
//     MusicBook,
//     Mystery,
//     Nature,
//     Paranormal,
//     Philosophy,
//     Poetry,
//     Politics,
//     Reference,
//     Religion,
//     Romance,
//     Rpg,
//     Science,
//     ScienceFiction,
//     SelfHelp,
//     SheetCollection,
//     SheetCollectionMP3,
//     Sports,
//     Technology,
//     Thriller,
//     Travel,
//     UrbanFantasy,
//     Western,
//     YoungAdult,
//     Superheroes,
//     LiteraryFiction,
//     ProgressionFantasy,
//     ContemporaryFiction,
//     DramaPlays,
//     OccultMetaphysicalPractices,
//     SliceOfLife,
//     Unknown(u8),
// }
