use std::fmt;

use itertools::Itertools as _;

use crate::{
    Flags, MediaType, MetadataSource, OldCategory, TorrentMeta, TorrentMetaDiff, TorrentMetaField,
    VipStatus, impls::format_serie,
};

impl TorrentMeta {
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
        let mut diff = vec![];
        if self.mam_id != other.mam_id {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MamId,
                from: self.mam_id.to_string(),
                to: other.mam_id.to_string(),
            });
        }
        if self.vip_status != other.vip_status
        // If we go from exired temp vip to not vip, do not write a diff
            && !(self
                .vip_status
                .as_ref()
                .is_some_and(|s| !s.is_vip())
                && other.vip_status == Some(VipStatus::NotVip))
        {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Vip,
                from: self
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
        if self.cat != other.cat {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Cat,
                from: self
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
        if self.media_type != other.media_type {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MediaType,
                from: self.media_type.to_string(),
                to: other.media_type.to_string(),
            });
        }
        if self.main_cat != other.main_cat {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::MainCat,
                from: self.main_cat.map(|c| c.to_string()).unwrap_or_default(),
                to: other.main_cat.map(|c| c.to_string()).unwrap_or_default(),
            });
        }
        if self.categories != other.categories {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Categories,
                from: self
                    .categories
                    .iter()
                    .map(|cat| cat.as_raw_str().to_string())
                    .join(", "),
                to: other
                    .categories
                    .iter()
                    .map(|cat| cat.as_raw_str().to_string())
                    .join(", "),
            });
        }
        if self.language != other.language {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Language,
                from: self
                    .language
                    .map(|language| language.to_str().to_string())
                    .unwrap_or_default(),
                to: other
                    .language
                    .map(|language| language.to_str().to_string())
                    .unwrap_or_default(),
            });
        }
        if self.flags != other.flags {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Flags,
                from: self
                    .flags
                    .map(|flags| format!("{}", Flags::from(flags)))
                    .unwrap_or_default(),
                to: other
                    .flags
                    .map(|flags| format!("{}", Flags::from(flags)))
                    .unwrap_or_default(),
            });
        }
        if self.filetypes != other.filetypes {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Filetypes,
                from: self.filetypes.join(", ").to_string(),
                to: other.filetypes.join(", ").to_string(),
            });
        }
        if self.size != other.size {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Size,
                from: self.size.to_string(),
                to: other.size.to_string(),
            });
        }
        if self.title != other.title {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Title,
                from: self.title.to_string(),
                to: other.title.to_string(),
            });
        }
        if self.edition != other.edition {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Edition,
                from: self
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
        if self.authors != other.authors {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Authors,
                from: self.authors.join(", ").to_string(),
                to: other.authors.join(", ").to_string(),
            });
        }
        if self.narrators != other.narrators {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Narrators,
                from: self.narrators.join(", ").to_string(),
                to: other.narrators.join(", ").to_string(),
            });
        }
        if self.series != other.series {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Series,
                from: self.series.iter().map(format_serie).join(", ").to_string(),
                to: other.series.iter().map(format_serie).join(", ").to_string(),
            });
        }
        if self.source != other.source {
            diff.push(TorrentMetaDiff {
                field: TorrentMetaField::Source,
                from: self.source.to_string(),
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
            TorrentMetaField::MamId => write!(f, "mam_id"),
            TorrentMetaField::Vip => write!(f, "vip"),
            TorrentMetaField::Cat => write!(f, "cat"),
            TorrentMetaField::MediaType => write!(f, "media_type"),
            TorrentMetaField::MainCat => write!(f, "main_cat"),
            TorrentMetaField::Categories => write!(f, "categories"),
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
        }
    }
}
