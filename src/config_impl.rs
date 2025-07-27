use std::path::PathBuf;

use anyhow::ensure;
use time::UtcDateTime;
use tracing::error;

use crate::{
    config::{Cost, Filter, GoodreadsList, Library, LibraryLinkMethod, LibraryTagFilters},
    data::{Language, MainCat, Size, Torrent},
    mam::{DATE_TIME_FORMAT, MaMTorrent},
    mam_enums::Flags,
};

impl Filter {
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
        }
        if self.categories.audio.as_ref().is_some_and(|c| c.is_empty())
            && torrent.meta.main_cat == MainCat::Audio
        {
            return Ok(false);
        }
        if self.categories.ebook.as_ref().is_some_and(|c| c.is_empty())
            && torrent.meta.main_cat == MainCat::Ebook
        {
            return Ok(false);
        }

        ensure!(self.flags.as_bitfield() == 0, "has flags");
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

        ensure!(
            self.categories.audio.as_ref().is_none_or(|c| c.is_empty()),
            "has advanced audio selection"
        );
        ensure!(
            self.categories.ebook.as_ref().is_none_or(|c| c.is_empty()),
            "has advanced ebook selection"
        );

        Ok(true)
    }
}

impl Cost {
    pub fn wedge(self) -> bool {
        match self {
            Cost::Wedge => true,
            Cost::TryWedge => true,
            _ => false,
        }
    }
}

impl GoodreadsList {
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

    use crate::mam_enums::AudiobookCategory;

    use super::*;

    #[test]
    fn test_uploaded_after() {
        let torrent = MaMTorrent {
            category: AudiobookCategory::ActionAdventure.to_id() as u64,
            added: "2025-07-06 05:40:54".to_owned(),
            ..Default::default()
        };
        let filter = Filter {
            uploaded_after: Some(date!(2025 - 07 - 05)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
        let filter = Filter {
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
        let filter = Filter {
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
        let filter = Filter {
            uploaded_before: Some(date!(2025 - 07 - 07)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
        let filter = Filter {
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
        let filter = Filter {
            uploaded_before: Some(date!(2025 - 07 - 06)),
            ..Default::default()
        };
        assert!(filter.matches(&torrent));
    }
}
