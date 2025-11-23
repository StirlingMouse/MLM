use std::path::PathBuf;

use anyhow::{Result, ensure};
use reqwest::Url;
use time::UtcDateTime;
use tracing::error;

use crate::{
    config::{GoodreadsList, Library, LibraryLinkMethod, LibraryTagFilters, TorrentFilter},
    data::{Language, MediaType, OldCategory, Size, Torrent},
    mam::{enums::Flags, search::MaMTorrent, serde::DATE_TIME_FORMAT},
};

impl TorrentFilter {
    pub fn matches(&self, torrent: &MaMTorrent) -> bool {
        if !self.categories.matches(torrent.category) {
            return false;
        }

        if !self.languages.is_empty() {
            if let Some(language) = Language::from_id(torrent.language) {
                if !self.languages.contains(&language) {
                    return false;
                }
            } else {
                error!(
                    "Failed parsing language \"{}\" for torrent \"{}\"",
                    torrent.language, torrent.title
                );
                return false;
            }
        }

        let torrent_flags = Flags::from_bitfield(torrent.browseflags);
        if !self.flags.matches(&torrent_flags) {
            return false;
        }

        if self.min_size.bytes() > 0 || self.max_size.bytes() > 0 {
            match Size::try_from(torrent.size.clone()) {
                Ok(size) => {
                    if self.min_size.bytes() > 0 && size < self.min_size {
                        return false;
                    }
                    if self.max_size.bytes() > 0 && size > self.max_size {
                        return false;
                    }
                }
                Err(_) => {
                    error!(
                        "Failed parsing size \"{}\" for torrent \"{}\"",
                        torrent.size, torrent.title
                    );
                    return false;
                }
            };
        }

        if self.exclude_uploader.contains(&torrent.owner_name) {
            return false;
        }

        if self.uploaded_after.is_some() || self.uploaded_before.is_some() {
            match UtcDateTime::parse(&torrent.added, &DATE_TIME_FORMAT) {
                Ok(added) => {
                    if let Some(uploaded_after) = self.uploaded_after {
                        if added.date() < uploaded_after {
                            return false;
                        }
                    }
                    if let Some(uploaded_before) = self.uploaded_before {
                        if added.date() > uploaded_before {
                            return false;
                        }
                    }
                }
                Err(_) => {
                    error!(
                        "Failed parsing added \"{}\" for torrent \"{}\"",
                        torrent.added, torrent.title
                    );
                    return false;
                }
            }
        }

        if let Some(min_seeders) = self.min_seeders {
            if torrent.seeders < min_seeders {
                return false;
            }
        }
        if let Some(max_seeders) = self.max_seeders {
            if torrent.seeders > max_seeders {
                return false;
            }
        }
        if let Some(min_leechers) = self.min_leechers {
            if torrent.leechers < min_leechers {
                return false;
            }
        }
        if let Some(max_leechers) = self.max_leechers {
            if torrent.leechers > max_leechers {
                return false;
            }
        }
        if let Some(min_snatched) = self.min_snatched {
            if torrent.times_completed < min_snatched {
                return false;
            }
        }
        if let Some(max_snatched) = self.max_snatched {
            if torrent.times_completed > max_snatched {
                return false;
            }
        }

        true
    }

    pub(crate) fn matches_lib(&self, torrent: &Torrent) -> Result<bool, anyhow::Error> {
        if let Some(language) = &torrent.meta.language {
            if !self.languages.is_empty() && !self.languages.contains(language) {
                return Ok(false);
            }
        } else {
            ensure!(
                self.languages.is_empty(),
                "has language selection and no stored language"
            );
        }
        if let Some(cat) = &torrent.meta.cat {
            match cat {
                OldCategory::Audio(category) => {
                    if self
                        .categories
                        .audio
                        .as_ref()
                        .is_some_and(|cats| !cats.contains(category))
                    {
                        return Ok(false);
                    }
                }
                OldCategory::Ebook(category) => {
                    if self
                        .categories
                        .ebook
                        .as_ref()
                        .is_some_and(|cats| !cats.contains(category))
                    {
                        return Ok(false);
                    }
                }
            }
        } else {
            if self.categories.audio.as_ref().is_some_and(|c| c.is_empty())
                && torrent.meta.media_type == MediaType::Audiobook
            {
                return Ok(false);
            }
            if self.categories.ebook.as_ref().is_some_and(|c| c.is_empty())
                && torrent.meta.media_type == MediaType::Ebook
            {
                return Ok(false);
            }
            ensure!(
                self.categories.audio.as_ref().is_none_or(|c| c.is_empty()),
                "has advanced audio selection and no stored category"
            );
            ensure!(
                self.categories.ebook.as_ref().is_none_or(|c| c.is_empty()),
                "has advanced ebook selection and no stored category"
            );
        }
        if let Some(flags) = torrent.meta.flags {
            let flags: Flags = flags.into();
            if !self.flags.matches(&flags) {
                return Ok(false);
            }
        } else {
            ensure!(
                self.flags.as_bitfield() == 0,
                "has flags selection and no stored flags"
            );
        }

        ensure!(self.min_size.bytes() == 0, "has min_size");
        ensure!(self.max_size.bytes() == 0, "has max_size");
        ensure!(self.exclude_uploader.is_empty(), "has exclude_uploader");
        ensure!(self.uploaded_after.is_none(), "has uploaded_after");
        ensure!(self.uploaded_before.is_none(), "has uploaded_before");
        ensure!(self.min_seeders.is_none(), "has min_seeders");
        ensure!(self.max_seeders.is_none(), "has max_seeders");
        ensure!(self.min_leechers.is_none(), "has min_leechers");
        ensure!(self.max_leechers.is_none(), "has max_leechers");
        ensure!(self.min_snatched.is_none(), "has min_snatched");
        ensure!(self.max_snatched.is_none(), "has max_snatched");

        Ok(true)
    }
}

impl GoodreadsList {
    pub fn list_id(&self) -> Result<String, anyhow::Error> {
        let link: Url = self.url.parse()?;
        let user_id = link
            .path_segments()
            .iter_mut()
            .flatten()
            .next_back()
            .ok_or(anyhow::Error::msg("Failed to get goodreads user id"))?;
        let (_, shelf) = link
            .query_pairs()
            .find(|(name, _)| name == "shelf")
            .unwrap_or_default();
        let list_id = format!("{user_id}:{shelf}");
        Ok(list_id)
    }

    pub fn allow_audio(&self) -> bool {
        self.grab.iter().any(|g| {
            g.filter
                .categories
                .audio
                .as_ref()
                .is_none_or(|c| !c.is_empty())
        })
    }

    pub fn allow_ebook(&self) -> bool {
        self.grab.iter().any(|g| {
            g.filter
                .categories
                .ebook
                .as_ref()
                .is_none_or(|c| !c.is_empty())
        })
    }
}

impl Library {
    pub fn method(&self) -> LibraryLinkMethod {
        match self {
            Library::ByDir(l) => l.tag_filters.method,
            Library::ByCategory(l) => l.tag_filters.method,
        }
    }

    pub fn library_dir(&self) -> &PathBuf {
        match self {
            Library::ByDir(l) => &l.library_dir,
            Library::ByCategory(l) => &l.library_dir,
        }
    }

    pub fn tag_filters(&self) -> &LibraryTagFilters {
        match self {
            Library::ByDir(l) => &l.tag_filters,
            Library::ByCategory(l) => &l.tag_filters,
        }
    }
}

#[cfg(test)]
mod tests {
    use time::macros::date;

    use crate::{
        data::{AudiobookCategory, FlagBits, Timestamp, TorrentMeta},
        mam_enums::Categories,
    };

    use super::*;

    #[test]
    fn test_uploaded_after() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = TorrentFilter {
            uploaded_after: Some(date!(2025 - 07 - 05)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
        let filter = TorrentFilter {
            uploaded_after: Some(date!(2025 - 07 - 07)),
            ..Default::default()
        };
        assert!(!filter.matches(&torrent));
    }

    #[test]
    fn test_uploaded_after_should_be_inclusive() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = TorrentFilter {
            uploaded_after: Some(date!(2025 - 07 - 06)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
    }

    #[test]
    fn test_uploaded_before() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = TorrentFilter {
            uploaded_before: Some(date!(2025 - 07 - 07)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
        let filter = TorrentFilter {
            uploaded_before: Some(date!(2025 - 07 - 05)),
            ..Default::default()
        };
        assert!(!filter.matches(&torrent));
    }

    #[test]
    fn test_uploaded_before_should_be_inclusive() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = TorrentFilter {
            uploaded_before: Some(date!(2025 - 07 - 06)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
    }

    mod filter_matches {
        use super::*;

        fn create_default_torrent() -> MaMTorrent {
            MaMTorrent {
                id: 1,
                added: "2023-01-20 10:00:00".to_string(),
                browseflags: Flags {
                    violence: Some(true),
                    ..Default::default()
                }
                .as_bitfield(),
                category: AudiobookCategory::GeneralFiction.to_id() as u64,
                language: Language::English.to_id(),
                leechers: 5,
                owner_name: "TestUploader".to_string(),
                seeders: 50,
                size: "5 GiB".to_string(),
                times_completed: 10,
                title: "Test Torrent".to_string(),
                ..Default::default()
            }
        }

        #[test]
        fn test_default_filter_matches_all() {
            let filter = TorrentFilter::default();
            let torrent = create_default_torrent();
            assert!(
                filter.matches(&torrent),
                "A default filter should match a default torrent."
            );
        }

        // --- Category Filtering ---
        #[test]
        fn test_category_match() {
            let filter = TorrentFilter {
                categories: Categories {
                    audio: Some(vec![AudiobookCategory::GeneralFiction]),
                    ebook: Some(vec![]),
                },
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(filter.matches(&torrent), "Should match category");
        }

        #[test]
        fn test_category_no_match() {
            let filter = TorrentFilter {
                categories: Categories {
                    audio: Some(vec![AudiobookCategory::GeneralNonFic]),
                    ebook: Some(vec![]),
                },
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(!filter.matches(&torrent), "Should not match category");
        }

        // --- Language Filtering ---
        #[test]
        fn test_language_match() {
            let filter = TorrentFilter {
                languages: vec![Language::English, Language::German],
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(filter.matches(&torrent), "Should match English language.");
        }

        #[test]
        fn test_language_no_match() {
            let filter = TorrentFilter {
                languages: vec![Language::French, Language::German],
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                !filter.matches(&torrent),
                "Should not match, filter only contains French/German."
            );
        }

        #[test]
        fn test_language_parse_fail() {
            let filter = TorrentFilter {
                languages: vec![Language::French],
                ..TorrentFilter::default()
            };
            let mut torrent = create_default_torrent();
            torrent.language = 99; // Invalid Language
            assert!(
                !filter.matches(&torrent),
                "Should not match if language ID cannot be mapped when a language filter is active."
            );
        }

        // --- Flags Filtering ---
        #[test]
        fn test_flags_match() {
            let filter = TorrentFilter {
                flags: Flags {
                    violence: Some(true),
                    ..Default::default()
                },
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                filter.matches(&torrent),
                "Should match if torrent has the required flag."
            );
        }

        #[test]
        fn test_flags_no_match() {
            let filter = TorrentFilter {
                flags: Flags {
                    explicit: Some(true),
                    ..Default::default()
                },
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                !filter.matches(&torrent),
                "Should not match if torrent is missing a required flag."
            );
        }

        // --- Size Filtering ---
        #[test]
        fn test_min_size_match() {
            let filter = TorrentFilter {
                min_size: Size::from_bytes(1_000_000_000),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                filter.matches(&torrent),
                "Torrent size 5GB should be >= 1GB min size."
            );
        }

        #[test]
        fn test_min_size_no_match() {
            let filter = TorrentFilter {
                min_size: Size::from_bytes(10_000_000_000),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                !filter.matches(&torrent),
                "Torrent size 5GB should be < 10GB min size."
            );
        }

        #[test]
        fn test_max_size_match() {
            let filter = TorrentFilter {
                max_size: Size::from_bytes(10_000_000_000),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                filter.matches(&torrent),
                "Torrent size 5GB should be <= 10GB max size."
            );
        }

        #[test]
        fn test_max_size_no_match() {
            let filter = TorrentFilter {
                max_size: Size::from_bytes(1_000_000_000),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                !filter.matches(&torrent),
                "Torrent size 5GB should be > 1GB max size."
            );
        }

        #[test]
        fn test_size_parsing_failure() {
            let filter = TorrentFilter {
                min_size: Size::from_bytes(1),
                ..TorrentFilter::default()
            };
            let mut torrent = create_default_torrent();
            torrent.size = "INVALID_SIZE".to_string();
            assert!(
                !filter.matches(&torrent),
                "Should fail if torrent size cannot be parsed when size filter is active."
            );
        }

        // --- Uploader Exclusion ---
        #[test]
        fn test_uploader_excluded() {
            let filter = TorrentFilter {
                exclude_uploader: vec!["BadUploader".to_string(), "TestUploader".to_string()],
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                !filter.matches(&torrent),
                "Should fail if uploader is in the exclusion list."
            );
        }

        #[test]
        fn test_uploader_not_excluded() {
            let filter = TorrentFilter {
                exclude_uploader: vec!["BadUploader".to_string()],
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                filter.matches(&torrent),
                "Should pass if uploader is not in the exclusion list."
            );
        }

        // --- Date Filtering ---
        #[test]
        fn test_uploaded_after_match() {
            let filter = TorrentFilter {
                uploaded_after: Some(date!(2023 - 01 - 15)),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                filter.matches(&torrent),
                "2023-01-20 should be >= 2023-01-15"
            );
        }

        #[test]
        fn test_uploaded_after_no_match() {
            let filter = TorrentFilter {
                uploaded_after: Some(date!(2023 - 01 - 25)),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                !filter.matches(&torrent),
                "2023-01-20 should be < 2023-01-25"
            );
        }

        #[test]
        fn test_uploaded_before_match() {
            let filter = TorrentFilter {
                uploaded_before: Some(date!(2023 - 01 - 25)),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                filter.matches(&torrent),
                "2023-01-20 should be <= 2023-01-25"
            );
        }

        #[test]
        fn test_uploaded_before_no_match() {
            let filter = TorrentFilter {
                uploaded_before: Some(date!(2023 - 01 - 15)),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                !filter.matches(&torrent),
                "2023-01-20 should be > 2023-01-15"
            );
        }

        #[test]
        fn test_date_parsing_failure() {
            let filter = TorrentFilter {
                uploaded_after: Some(date!(2023 - 01 - 25)),
                ..TorrentFilter::default()
            };
            let mut torrent = create_default_torrent();
            torrent.added = "INVALID_DATE".to_string();
            assert!(
                !filter.matches(&torrent),
                "Should fail if the torrent's 'added' date cannot be parsed when date filter is active."
            );
        }

        // --- Seeder, Leecher, Snatched Filtering (Stats) ---
        // Torrent defaults: seeders: 50, leechers: 5, times_completed: 10
        #[test]
        fn test_min_seeders_match() {
            let filter = TorrentFilter {
                min_seeders: Some(40),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent(); // seeders: 50
            assert!(filter.matches(&torrent), "50 seeders should be >= 40.");
        }

        #[test]
        fn test_min_seeders_no_match() {
            let filter = TorrentFilter {
                min_seeders: Some(60),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent(); // seeders: 50
            assert!(!filter.matches(&torrent), "50 seeders should be < 60.");
        }

        #[test]
        fn test_max_seeders_match() {
            let filter = TorrentFilter {
                max_seeders: Some(60),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent(); // seeders: 50
            assert!(filter.matches(&torrent), "50 seeders should be <= 60.");
        }

        #[test]
        fn test_max_seeders_no_match() {
            let filter = TorrentFilter {
                max_seeders: Some(40),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent(); // seeders: 50
            assert!(!filter.matches(&torrent), "50 seeders should be > 40.");
        }

        #[test]
        fn test_min_leechers_match() {
            let filter = TorrentFilter {
                min_leechers: Some(5),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent(); // leechers: 5
            assert!(filter.matches(&torrent), "5 leechers should be >= 5.");
        }

        #[test]
        fn test_max_leechers_no_match() {
            let filter = TorrentFilter {
                max_leechers: Some(4),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent(); // leechers: 5
            assert!(!filter.matches(&torrent), "5 leechers should be > 4.");
        }

        #[test]
        fn test_min_snatched_match() {
            let filter = TorrentFilter {
                min_snatched: Some(10),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent(); // times_completed: 10
            assert!(filter.matches(&torrent), "10 completions should be >= 10.");
        }

        #[test]
        fn test_max_snatched_no_match() {
            let filter = TorrentFilter {
                max_snatched: Some(9),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent(); // times_completed: 10
            assert!(!filter.matches(&torrent), "10 completions should be > 9.");
        }

        // --- Combined Tests ---
        #[test]
        fn test_combined_success() {
            let filter = TorrentFilter {
                categories: Categories {
                    audio: Some(vec![AudiobookCategory::GeneralFiction]),
                    ebook: Some(vec![]),
                },
                languages: vec![Language::English],
                flags: Flags {
                    violence: Some(true),
                    ..Default::default()
                },
                min_size: Size::from_bytes(1_000_000_000),
                max_size: Size::from_bytes(10_000_000_000),
                exclude_uploader: vec!["OtherUploader".to_string()],
                uploaded_after: Some(date!(2023 - 01 - 15)),
                min_seeders: Some(40),
                max_leechers: Some(10),
                ..TorrentFilter::default()
            };
            let torrent = create_default_torrent();
            assert!(
                filter.matches(&torrent),
                "Should pass when all filters are met."
            );
        }
    }

    mod filter_matches_lib {
        use crate::data::{MainCat, MediaType, MetadataSource, OldCategory};

        use super::*;

        fn create_torrent_with_meta(meta: TorrentMeta) -> Torrent {
            Torrent {
                meta,

                hash: "".to_string(),
                mam_id: 0,
                abs_id: None,
                goodreads_id: None,
                library_path: None,
                library_files: vec![],
                linker: None,
                category: None,
                selected_audio_format: None,
                selected_ebook_format: None,
                title_search: "".to_string(),
                created_at: Timestamp::now(),
                replaced_with: None,
                request_matadata_update: false,
                library_mismatch: None,
                client_status: None,
            }
        }

        fn default_meta() -> TorrentMeta {
            TorrentMeta {
                mam_id: 0,
                vip_status: None,
                media_type: MediaType::Audiobook,
                main_cat: Some(MainCat::Fiction),
                categories: vec![],
                cat: None,
                language: None,
                flags: None,
                filetypes: vec![],
                size: Size::from_bytes(0),
                title: "".to_string(),
                authors: vec![],
                narrators: vec![],
                series: vec![],
                source: MetadataSource::Mam,
            }
        }

        fn create_filter_with_audio_cats(cats: Option<Vec<AudiobookCategory>>) -> TorrentFilter {
            TorrentFilter {
                categories: Categories {
                    audio: cats,
                    ..Default::default()
                },
                ..Default::default()
            }
        }

        // --- Language Filtering Tests ---
        #[test]
        fn test_lang_match_ok_true() {
            let filter = TorrentFilter {
                languages: vec![Language::English, Language::French],
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(TorrentMeta {
                language: Some(Language::English),
                ..default_meta()
            });
            assert!(filter.matches_lib(&torrent).unwrap());
        }

        #[test]
        fn test_lang_mismatch_ok_false() {
            let filter = TorrentFilter {
                languages: vec![Language::French],
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(TorrentMeta {
                language: Some(Language::English),
                ..default_meta()
            });
            assert!(!filter.matches_lib(&torrent).unwrap());
        }

        #[test]
        fn test_lang_filter_active_torrent_none_err() {
            let filter = TorrentFilter {
                languages: vec![Language::English],
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(TorrentMeta {
                language: None,
                ..default_meta()
            });
            // Should return Err due to `ensure!` failing
            assert!(filter.matches_lib(&torrent).is_err());
            assert!(
                filter
                    .matches_lib(&torrent)
                    .unwrap_err()
                    .to_string()
                    .contains("has language selection and no stored language")
            );
        }

        #[test]
        fn test_lang_filter_inactive_torrent_none_ok_true() {
            let filter = TorrentFilter {
                languages: vec![],
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(TorrentMeta {
                language: None,
                ..default_meta()
            });
            assert!(filter.matches_lib(&torrent).unwrap());
        }

        // --- Category Filtering Tests ---
        #[test]
        fn test_audio_cat_match_ok_true() {
            let filter =
                create_filter_with_audio_cats(Some(vec![AudiobookCategory::GeneralFiction]));
            let torrent = create_torrent_with_meta(TorrentMeta {
                cat: Some(OldCategory::Audio(AudiobookCategory::GeneralFiction)),
                ..default_meta()
            });
            assert!(filter.matches_lib(&torrent).unwrap());
        }

        #[test]
        fn test_audio_cat_mismatch_ok_false() {
            let filter =
                create_filter_with_audio_cats(Some(vec![AudiobookCategory::GeneralNonFic]));
            let torrent = create_torrent_with_meta(TorrentMeta {
                cat: Some(OldCategory::Audio(AudiobookCategory::GeneralFiction)),
                ..default_meta()
            });
            assert!(!filter.matches_lib(&torrent).unwrap());
        }

        #[test]
        fn test_cat_filter_inactive_torrent_has_cat_ok_true() {
            let filter = TorrentFilter::default();
            let torrent = create_torrent_with_meta(TorrentMeta {
                cat: Some(OldCategory::Audio(AudiobookCategory::GeneralNonFic)),
                ..default_meta()
            });
            assert!(filter.matches_lib(&torrent).unwrap());
        }

        #[test]
        fn test_no_cat_and_filter_empty_set_audio_ok_false() {
            let filter = create_filter_with_audio_cats(Some(vec![]));
            let torrent = create_torrent_with_meta(TorrentMeta {
                cat: None,
                media_type: MediaType::Audiobook, // Main category matches
                ..default_meta()
            });
            assert!(
                !filter.matches_lib(&torrent).unwrap(),
                "Should return false for Audio main-cat with no sub-cat, when filter has an empty set of audio categories."
            );
        }

        #[test]
        fn test_no_cat_and_filter_active_err() {
            let filter =
                create_filter_with_audio_cats(Some(vec![AudiobookCategory::GeneralFiction]));
            let torrent = create_torrent_with_meta(TorrentMeta {
                cat: None,
                media_type: MediaType::Audiobook,
                ..default_meta()
            });
            assert!(filter.matches_lib(&torrent).is_err());
            assert!(
                filter
                    .matches_lib(&torrent)
                    .unwrap_err()
                    .to_string()
                    .contains("has advanced audio selection and no stored category")
            );
        }

        // --- Flags Filtering Tests ---
        #[test]
        fn test_flags_match_ok_true() {
            let filter = TorrentFilter {
                flags: Flags {
                    violence: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(TorrentMeta {
                flags: Some(FlagBits::new(
                    Flags {
                        violence: Some(true),
                        explicit: Some(true),
                        ..Default::default()
                    }
                    .as_bitfield(),
                )),
                ..default_meta()
            });
            assert!(filter.matches_lib(&torrent).unwrap());
        }

        #[test]
        fn test_flags_mismatch_ok_false() {
            let filter = TorrentFilter {
                flags: Flags {
                    explicit: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(TorrentMeta {
                flags: Some(FlagBits::new(
                    Flags {
                        violence: Some(true),
                        ..Default::default()
                    }
                    .as_bitfield(),
                )),
                ..default_meta()
            });
            assert!(!filter.matches_lib(&torrent).unwrap());
        }

        #[test]
        fn test_flags_filter_active_torrent_none_err() {
            let filter = TorrentFilter {
                flags: Flags {
                    violence: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(TorrentMeta {
                flags: None,
                ..default_meta()
            });
            assert!(filter.matches_lib(&torrent).is_err());
            assert!(
                filter
                    .matches_lib(&torrent)
                    .unwrap_err()
                    .to_string()
                    .contains("has flags selection and no stored flags")
            );
        }

        // --- Disallowed Filter Checks (Ensure) ---
        #[test]
        fn test_disallowed_min_size_err() {
            let filter = TorrentFilter {
                min_size: Size::from_bytes(1),
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(default_meta());
            assert!(filter.matches_lib(&torrent).is_err());
            assert!(
                filter
                    .matches_lib(&torrent)
                    .unwrap_err()
                    .to_string()
                    .contains("has min_size")
            );
        }

        #[test]
        fn test_disallowed_exclude_uploader_err() {
            let filter = TorrentFilter {
                exclude_uploader: vec!["test".to_string()],
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(default_meta());
            assert!(filter.matches_lib(&torrent).is_err());
            assert!(
                filter
                    .matches_lib(&torrent)
                    .unwrap_err()
                    .to_string()
                    .contains("has exclude_uploader")
            );
        }

        #[test]
        fn test_disallowed_uploaded_after_err() {
            let filter = TorrentFilter {
                uploaded_after: Some(date!(2023 - 01 - 15)),
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(default_meta());
            assert!(filter.matches_lib(&torrent).is_err());
            assert!(
                filter
                    .matches_lib(&torrent)
                    .unwrap_err()
                    .to_string()
                    .contains("has uploaded_after")
            );
        }

        #[test]
        fn test_disallowed_max_seeders_err() {
            let filter = TorrentFilter {
                max_seeders: Some(100),
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(default_meta());
            assert!(filter.matches_lib(&torrent).is_err());
            assert!(
                filter
                    .matches_lib(&torrent)
                    .unwrap_err()
                    .to_string()
                    .contains("has max_seeders")
            );
        }

        // --- Full Success Case ---
        #[test]
        fn test_full_success_match() {
            let filter = TorrentFilter {
                languages: vec![Language::English],
                categories: Categories {
                    audio: Some(vec![AudiobookCategory::GeneralFiction]),
                    ebook: None,
                },
                flags: Flags {
                    crude_language: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            };
            let torrent = create_torrent_with_meta(TorrentMeta {
                language: Some(Language::English),
                cat: Some(OldCategory::Audio(AudiobookCategory::GeneralFiction)),
                media_type: MediaType::Audiobook,
                flags: Some(FlagBits::new(
                    Flags {
                        crude_language: Some(true),
                        explicit: Some(true),
                        ..Default::default()
                    }
                    .as_bitfield(),
                )),
                ..default_meta()
            });
            assert!(
                filter.matches_lib(&torrent).unwrap(),
                "Torrent should pass all checks when all allowed filter criteria match."
            );
        }
    }
}
