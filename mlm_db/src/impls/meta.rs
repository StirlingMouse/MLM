use std::fmt;

use itertools::Itertools as _;

use crate::{
    Flags, MediaType, MetadataSource, OldCategory, TorrentMeta, TorrentMetaDiff, TorrentMetaField,
    VipStatus, ids, impls::format_serie,
};

impl TorrentMeta {
    pub fn canonicalize(&mut self) {
        self.categories.sort_unstable();
        self.categories.dedup();
    }

    pub fn mam_id(&self) -> Option<u64> {
        self.ids.get(ids::MAM).and_then(|id| id.parse().ok())
    }

    pub fn matches(&self, other: &TorrentMeta) -> bool {
        self.media_type.matches(other.media_type)
            && self.language == other.language
            && (self.edition.is_none() == other.edition.is_none()
                || self.edition.as_ref().is_some_and(|this| {
                    other
                        .edition
                        .as_ref()
                        .is_some_and(|that| this.1 == that.1 && (this.1 != 0 || this.0 == that.0))
                }))
            && self.authors.iter().any(|a| other.authors.contains(a))
            && ((self.narrators.is_empty() && other.narrators.is_empty())
                || self.narrators.iter().any(|a| other.narrators.contains(a)))
    }

    pub fn cat_name(&self) -> &str {
        match self.cat {
            Some(OldCategory::Audio(cat)) => cat.to_str(),
            Some(OldCategory::Ebook(cat)) => cat.to_str(),
            Some(OldCategory::Musicology(cat)) => cat.to_str(),
            Some(OldCategory::Radio(cat)) => cat.to_str(),
            None => "N/A",
        }
    }

    pub fn diff(&self, other: &TorrentMeta) -> Vec<TorrentMetaDiff> {
        let mut this = self.clone();
        this.canonicalize();
        let mut other = other.clone();
        other.canonicalize();

        let mut diff = vec![];
        if this.ids != other.ids {
            let format_ids = |ids: &std::collections::BTreeMap<String, String>| {
                ids.iter()
                    .map(|(key, value)| format!("{key}: {value}"))
                    .join("\n")
            };
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Ids,
                from: format_ids(&this.ids),
                to: format_ids(&other.ids),
            });
        }
        if this.vip_status != other.vip_status
        // If we go from exired temp vip to not vip, do not write a diff
            && !(this
                .vip_status
                .as_ref()
                .is_some_and(|s| !s.is_vip())
                && other.vip_status == Some(VipStatus::NotVip))
        {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Vip,
                from: this
                    .vip_status
                    .as_ref()
                    .map(|vip_status| vip_status.to_string())
                    .unwrap_or_default(),
                to: other
                    .vip_status
                    .as_ref()
                    .map(|vip_status| vip_status.to_string())
                    .unwrap_or_default(),
            });
        }
        if this.cat != other.cat {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Cat,
                from: this
                    .cat
                    .as_ref()
                    .map(|cat| cat.to_string())
                    .unwrap_or_default(),
                to: other
                    .cat
                    .as_ref()
                    .map(|cat| cat.to_string())
                    .unwrap_or_default(),
            });
        }
        if this.media_type != other.media_type {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MediaType,
                from: this.media_type.to_string(),
                to: other.media_type.to_string(),
            });
        }
        if this.main_cat != other.main_cat {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MainCat,
                from: this.main_cat.map(|c| c.to_string()).unwrap_or_default(),
                to: other.main_cat.map(|c| c.to_string()).unwrap_or_default(),
            });
        }
        if this.categories != other.categories {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Categories,
                from: this.categories.iter().map(ToString::to_string).join(", "),
                to: other.categories.iter().map(ToString::to_string).join(", "),
            });
        }
        if this.tags != other.tags {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Tags,
                from: this.tags.join(", "),
                to: other.tags.join(", "),
            });
        }
        if this.language != other.language {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Language,
                from: this
                    .language
                    .map(|language| language.to_str().to_string())
                    .unwrap_or_default(),
                to: other
                    .language
                    .map(|language| language.to_str().to_string())
                    .unwrap_or_default(),
            });
        }
        if this.flags != other.flags {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Flags,
                from: this
                    .flags
                    .map(|flags| format!("{}", Flags::from(flags)))
                    .unwrap_or_default(),
                to: other
                    .flags
                    .map(|flags| format!("{}", Flags::from(flags)))
                    .unwrap_or_default(),
            });
        }
        if this.filetypes != other.filetypes {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Filetypes,
                from: this.filetypes.join(", ").to_string(),
                to: other.filetypes.join(", ").to_string(),
            });
        }
        if this.size != other.size {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Size,
                from: this.size.to_string(),
                to: other.size.to_string(),
            });
        }
        if this.title != other.title {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Title,
                from: this.title.to_string(),
                to: other.title.to_string(),
            });
        }
        if this.edition != other.edition {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Edition,
                from: this
                    .edition
                    .as_ref()
                    .map(|e| e.0.to_string())
                    .unwrap_or_default(),
                to: other
                    .edition
                    .as_ref()
                    .map(|e| e.0.to_string())
                    .unwrap_or_default(),
            });
        }
        if this.authors != other.authors {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Authors,
                from: this.authors.join(", ").to_string(),
                to: other.authors.join(", ").to_string(),
            });
        }
        if this.narrators != other.narrators {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Narrators,
                from: this.narrators.join(", ").to_string(),
                to: other.narrators.join(", ").to_string(),
            });
        }
        if this.series != other.series {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Series,
                from: this.series.iter().map(format_serie).join(", ").to_string(),
                to: other.series.iter().map(format_serie).join(", ").to_string(),
            });
        }
        if this.source != other.source {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Source,
                from: this.source.to_string(),
                to: other.source.to_string(),
            });
        }
        diff
    }
}

impl MediaType {
    pub fn matches(&self, other: MediaType) -> bool {
        match (*self, other) {
            (a, b) if a == b => true,
            // Due to filetype restrictions, torrents for the same book will end up in different
            // media types
            (MediaType::Ebook, MediaType::ComicBook) => true,
            (MediaType::ComicBook, MediaType::Ebook) => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for TorrentMetaField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TorrentMetaField::Ids => write!(f, "ids"),
            TorrentMetaField::Vip => write!(f, "vip"),
            TorrentMetaField::Cat => write!(f, "cat"),
            TorrentMetaField::MediaType => write!(f, "media_type"),
            TorrentMetaField::MainCat => write!(f, "main_cat"),
            TorrentMetaField::Categories => write!(f, "categories"),
            TorrentMetaField::Tags => write!(f, "tags"),
            TorrentMetaField::Language => write!(f, "language"),
            TorrentMetaField::Flags => write!(f, "flags"),
            TorrentMetaField::Filetypes => write!(f, "filetypes"),
            TorrentMetaField::Size => write!(f, "size"),
            TorrentMetaField::Title => write!(f, "title"),
            TorrentMetaField::Edition => write!(f, "edition"),
            TorrentMetaField::Authors => write!(f, "authors"),
            TorrentMetaField::Narrators => write!(f, "narrators"),
            TorrentMetaField::Series => write!(f, "series"),
            TorrentMetaField::Source => write!(f, "source"),
        }
    }
}

impl fmt::Display for MetadataSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetadataSource::Mam => write!(f, "MaM"),
            MetadataSource::Manual => write!(f, "Manual"),
            MetadataSource::File => write!(f, "File"),
            MetadataSource::Match => write!(f, "Match"),
        }
    }
}
