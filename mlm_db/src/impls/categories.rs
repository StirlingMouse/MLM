use std::str::FromStr;

use crate::{Category, MainCat, MediaType, OldCategory, OldMainCat};

impl MediaType {
    pub fn from_id(id: u8) -> Option<MediaType> {
        match id {
            1 => Some(MediaType::Audiobook),
            2 => Some(MediaType::Ebook),
            3 => Some(MediaType::Musicology),
            4 => Some(MediaType::Radio),
            5 => Some(MediaType::Manga),
            6 => Some(MediaType::ComicBook),
            7 => Some(MediaType::PeriodicalEbook),
            8 => Some(MediaType::PeriodicalAudiobook),
            _ => None,
        }
    }

    pub fn from_main_cat_id(main_cat: u8) -> Option<MediaType> {
        match main_cat {
            13 => Some(MediaType::Audiobook),
            14 => Some(MediaType::Ebook),
            15 => Some(MediaType::Musicology),
            16 => Some(MediaType::Radio),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Audiobook => "Audiobook",
            MediaType::Ebook => "Ebook",
            MediaType::Musicology => "Musicology",
            MediaType::Radio => "Radio",
            MediaType::Manga => "Manga",
            MediaType::ComicBook => "Comic Book / Graphic Novel",
            MediaType::PeriodicalEbook => "Periodical Ebook",
            MediaType::PeriodicalAudiobook => "Periodical Audiobook",
        }
    }

    #[allow(dead_code)]
    pub fn as_id(&self) -> u8 {
        match self {
            MediaType::Audiobook => 1,
            MediaType::Ebook => 2,
            MediaType::Musicology => 3,
            MediaType::Radio => 4,
            MediaType::Manga => 5,
            MediaType::ComicBook => 6,
            MediaType::PeriodicalEbook => 7,
            MediaType::PeriodicalAudiobook => 8,
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for MediaType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "audiobook" => Ok(MediaType::Audiobook),
            "ebook" => Ok(MediaType::Ebook),
            "musicology" => Ok(MediaType::Musicology),
            "radio" => Ok(MediaType::Radio),
            "manga" => Ok(MediaType::Manga),
            "comic book / graphic novel" => Ok(MediaType::ComicBook),
            "comic book" => Ok(MediaType::ComicBook),
            "comics" => Ok(MediaType::ComicBook),
            "graphic novel" => Ok(MediaType::ComicBook),
            "periodical_ebook" => Ok(MediaType::PeriodicalEbook),
            "periodical ebook" => Ok(MediaType::PeriodicalEbook),
            "periodical_audiobook" => Ok(MediaType::PeriodicalAudiobook),
            "periodical audiobook" => Ok(MediaType::PeriodicalAudiobook),
            _ => Err(format!("Unknown media type: {}", value)),
        }
    }
}

impl From<OldMainCat> for MediaType {
    fn from(value: OldMainCat) -> Self {
        match value {
            OldMainCat::Audio => MediaType::Audiobook,
            OldMainCat::Ebook => MediaType::Ebook,
            OldMainCat::Musicology => MediaType::Musicology,
            OldMainCat::Radio => MediaType::Radio,
        }
    }
}

impl MainCat {
    pub fn from_id(id: u8) -> Option<MainCat> {
        match id {
            1 => Some(MainCat::Fiction),
            2 => Some(MainCat::Nonfiction),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MainCat::Fiction => "Fiction",
            MainCat::Nonfiction => "Nonfiction",
        }
    }

    #[allow(dead_code)]
    pub fn as_id(&self) -> u8 {
        match self {
            MainCat::Fiction => 1,
            MainCat::Nonfiction => 2,
        }
    }
}

impl std::fmt::Display for MainCat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Category {
    pub fn from_old_category(category: OldCategory) -> Vec<Category> {
        match category {
            OldCategory::Audio(cat) => {
                let mut mapped = vec![Category::Audiobook];
                mapped.extend(match cat {
                    crate::AudiobookCategory::ActionAdventure => vec![Category::ActionAdventure],
                    crate::AudiobookCategory::Art => vec![Category::ArtPhotography],
                    crate::AudiobookCategory::Biographical => vec![Category::Biography],
                    crate::AudiobookCategory::Business => vec![Category::Business],
                    crate::AudiobookCategory::ComputerInternet => vec![Category::ComputerScience],
                    crate::AudiobookCategory::Crafts => vec![Category::CraftsDiy],
                    crate::AudiobookCategory::CrimeThriller => {
                        vec![Category::Crime, Category::Thriller]
                    }
                    crate::AudiobookCategory::Fantasy => vec![Category::Fantasy],
                    crate::AudiobookCategory::Food => vec![Category::CookingFood],
                    crate::AudiobookCategory::GeneralNonFic => vec![],
                    crate::AudiobookCategory::HistoricalFiction => vec![Category::Historical],
                    crate::AudiobookCategory::History => vec![Category::History],
                    crate::AudiobookCategory::HomeGarden => vec![Category::HomeGarden],
                    crate::AudiobookCategory::Horror => vec![Category::Horror],
                    crate::AudiobookCategory::Humor => vec![Category::Funny, Category::Humor],
                    crate::AudiobookCategory::Instructional => vec![Category::GuideManual],
                    crate::AudiobookCategory::Juvenile => vec![Category::Children],
                    crate::AudiobookCategory::Language => vec![Category::LanguageLinguistics],
                    crate::AudiobookCategory::MathScienceTech => {
                        vec![Category::Science, Category::Technology]
                    }
                    crate::AudiobookCategory::Medical => vec![Category::Medicine],
                    crate::AudiobookCategory::Mystery => vec![Category::Mystery],
                    crate::AudiobookCategory::Nature => vec![Category::NatureEnvironment],
                    crate::AudiobookCategory::Philosophy => vec![Category::Philosophy],
                    crate::AudiobookCategory::PolSocRelig => {
                        vec![Category::PoliticsSociety, Category::ReligionSpirituality]
                    }
                    crate::AudiobookCategory::Recreation => vec![Category::SportsOutdoors],
                    crate::AudiobookCategory::Romance => vec![Category::Romance],
                    crate::AudiobookCategory::ScienceFiction => vec![Category::ScienceFiction],
                    crate::AudiobookCategory::SelfHelp => vec![Category::SelfHelp],
                    crate::AudiobookCategory::TravelAdventure => vec![Category::Travel],
                    crate::AudiobookCategory::TrueCrime => vec![Category::TrueCrime],
                    crate::AudiobookCategory::UrbanFantasy => {
                        vec![Category::Fantasy, Category::UrbanFantasy]
                    }
                    crate::AudiobookCategory::Western => vec![Category::Western],
                    crate::AudiobookCategory::YoungAdult => vec![Category::YoungAdult],
                    _ => vec![],
                });
                mapped
            }
            OldCategory::Ebook(cat) => {
                let mut mapped = vec![Category::Ebook];
                mapped.extend(match cat {
                    crate::EbookCategory::ActionAdventure => vec![Category::ActionAdventure],
                    crate::EbookCategory::Art => vec![Category::ArtPhotography],
                    crate::EbookCategory::Biographical => vec![Category::Biography],
                    crate::EbookCategory::Business => vec![Category::Business],
                    crate::EbookCategory::ComicsGraphicnovels => {
                        vec![Category::GraphicNovelsComics]
                    }
                    crate::EbookCategory::ComputerInternet => vec![Category::ComputerScience],
                    crate::EbookCategory::Crafts => vec![Category::CraftsDiy],
                    crate::EbookCategory::CrimeThriller => {
                        vec![Category::Crime, Category::Thriller]
                    }
                    crate::EbookCategory::Fantasy => vec![Category::Fantasy],
                    crate::EbookCategory::Food => vec![Category::CookingFood],
                    crate::EbookCategory::GeneralNonFiction => vec![],
                    crate::EbookCategory::HistoricalFiction => vec![Category::Historical],
                    crate::EbookCategory::History => vec![Category::History],
                    crate::EbookCategory::HomeGarden => vec![Category::HomeGarden],
                    crate::EbookCategory::Horror => vec![Category::Horror],
                    crate::EbookCategory::Humor => vec![Category::Funny, Category::Humor],
                    crate::EbookCategory::IllusionMagic => vec![Category::MythologyFolklore],
                    crate::EbookCategory::Instructional => vec![Category::GuideManual],
                    crate::EbookCategory::Juvenile => vec![Category::Children],
                    crate::EbookCategory::Language => vec![Category::LanguageLinguistics],
                    crate::EbookCategory::MagazinesNewspapers => vec![],
                    crate::EbookCategory::MathScienceTech => {
                        vec![Category::Science, Category::Technology]
                    }
                    crate::EbookCategory::Medical => vec![Category::Medicine],
                    crate::EbookCategory::MixedCollections => vec![Category::Anthology],
                    crate::EbookCategory::Mystery => vec![Category::Mystery],
                    crate::EbookCategory::Nature => vec![Category::NatureEnvironment],
                    crate::EbookCategory::Philosophy => vec![Category::Philosophy],
                    crate::EbookCategory::PolSocRelig => {
                        vec![Category::PoliticsSociety, Category::ReligionSpirituality]
                    }
                    crate::EbookCategory::Recreation => vec![Category::SportsOutdoors],
                    crate::EbookCategory::Romance => vec![Category::Romance],
                    crate::EbookCategory::ScienceFiction => vec![Category::ScienceFiction],
                    crate::EbookCategory::SelfHelp => vec![Category::SelfHelp],
                    crate::EbookCategory::TravelAdventure => vec![Category::Travel],
                    crate::EbookCategory::TrueCrime => vec![Category::TrueCrime],
                    crate::EbookCategory::UrbanFantasy => {
                        vec![Category::Fantasy, Category::UrbanFantasy]
                    }
                    crate::EbookCategory::Western => vec![Category::Western],
                    crate::EbookCategory::YoungAdult => vec![Category::YoungAdult],
                    _ => vec![],
                });
                mapped
            }
            OldCategory::Musicology(cat) => match cat {
                crate::MusicologyCategory::GuitarBassTabs
                | crate::MusicologyCategory::IndividualSheet
                | crate::MusicologyCategory::IndividualSheetMP3
                | crate::MusicologyCategory::SheetCollection
                | crate::MusicologyCategory::SheetCollectionMP3 => {
                    vec![Category::Music, Category::SheetMusicScores]
                }
                crate::MusicologyCategory::MusicCompleteEditions
                | crate::MusicologyCategory::MusicBook
                | crate::MusicologyCategory::MusicBookMP3 => vec![Category::Music],
                crate::MusicologyCategory::InstructionalBookWithVideo
                | crate::MusicologyCategory::InstructionalMediaMusic
                | crate::MusicologyCategory::LickLibraryLTPJamWith
                | crate::MusicologyCategory::LickLibraryTechniquesQL => {
                    vec![Category::Music, Category::GuideManual]
                }
            },
            OldCategory::Radio(cat) => match cat {
                crate::RadioCategory::Comedy => vec![Category::Funny],
                crate::RadioCategory::Drama => vec![Category::DramaPlays],
                _ => vec![],
            },
        }
    }

    pub fn from_legacy_v15_id(id: u8) -> Option<(Vec<Category>, Vec<String>)> {
        let category = match id {
            1 => crate::v15::Category::Action,
            2 => crate::v15::Category::Art,
            3 => crate::v15::Category::Biographical,
            4 => crate::v15::Category::Business,
            5 => crate::v15::Category::Comedy,
            6 => crate::v15::Category::CompleteEditionsMusic,
            7 => crate::v15::Category::Computer,
            8 => crate::v15::Category::Crafts,
            9 => crate::v15::Category::Crime,
            10 => crate::v15::Category::Dramatization,
            11 => crate::v15::Category::Education,
            12 => crate::v15::Category::FactualNews,
            13 => crate::v15::Category::Fantasy,
            14 => crate::v15::Category::Food,
            15 => crate::v15::Category::Guitar,
            16 => crate::v15::Category::Health,
            17 => crate::v15::Category::Historical,
            18 => crate::v15::Category::Home,
            19 => crate::v15::Category::Horror,
            20 => crate::v15::Category::Humor,
            21 => crate::v15::Category::IndividualSheet,
            22 => crate::v15::Category::Instructional,
            23 => crate::v15::Category::Juvenile,
            24 => crate::v15::Category::Language,
            25 => crate::v15::Category::Lgbt,
            26 => crate::v15::Category::LickLibraryLTP,
            27 => crate::v15::Category::LickLibraryTechniques,
            28 => crate::v15::Category::LiteraryClassics,
            29 => crate::v15::Category::LitRPG,
            30 => crate::v15::Category::Math,
            31 => crate::v15::Category::Medicine,
            32 => crate::v15::Category::Music,
            33 => crate::v15::Category::MusicBook,
            34 => crate::v15::Category::Mystery,
            35 => crate::v15::Category::Nature,
            36 => crate::v15::Category::Paranormal,
            37 => crate::v15::Category::Philosophy,
            38 => crate::v15::Category::Poetry,
            39 => crate::v15::Category::Politics,
            40 => crate::v15::Category::Reference,
            41 => crate::v15::Category::Religion,
            42 => crate::v15::Category::Romance,
            43 => crate::v15::Category::Rpg,
            44 => crate::v15::Category::Science,
            45 => crate::v15::Category::ScienceFiction,
            46 => crate::v15::Category::SelfHelp,
            47 => crate::v15::Category::SheetCollection,
            48 => crate::v15::Category::SheetCollectionMP3,
            49 => crate::v15::Category::Sports,
            50 => crate::v15::Category::Technology,
            51 => crate::v15::Category::Thriller,
            52 => crate::v15::Category::Travel,
            53 => crate::v15::Category::UrbanFantasy,
            54 => crate::v15::Category::Western,
            55 => crate::v15::Category::YoungAdult,
            56 => crate::v15::Category::Superheroes,
            57 => crate::v15::Category::LiteraryFiction,
            58 => crate::v15::Category::ProgressionFantasy,
            59 => crate::v15::Category::ContemporaryFiction,
            60 => crate::v15::Category::DramaPlays,
            61 => crate::v15::Category::Unknown(61),
            62 => crate::v15::Category::Unknown(62),
            _ => return None,
        };
        Some(Self::from_legacy_v15_category(category, &[], &[]))
    }

    pub fn from_legacy_v15_category(
        category: crate::v15::Category,
        legacy_categories: &[crate::v15::Category],
        existing_categories: &[Category],
    ) -> (Vec<Category>, Vec<String>) {
        let mut mapped = Vec::new();
        let mut tags = Vec::new();

        match category {
            crate::v15::Category::Action => mapped.extend([Category::ActionAdventure]),
            crate::v15::Category::Art => mapped.extend([Category::ArtPhotography]),
            crate::v15::Category::Biographical => mapped.extend([Category::Biography]),
            crate::v15::Category::Business => mapped.extend([Category::Business]),
            crate::v15::Category::Comedy | crate::v15::Category::Humor => {
                mapped.extend([Category::Funny, Category::Humor]);
            }
            crate::v15::Category::CompleteEditionsMusic
            | crate::v15::Category::Music
            | crate::v15::Category::MusicBook => mapped.extend([Category::Music]),
            crate::v15::Category::Computer => mapped.extend([Category::ComputerScience]),
            crate::v15::Category::Crafts => mapped.extend([Category::CraftsDiy]),
            crate::v15::Category::Crime => mapped.extend([Category::Crime]),
            crate::v15::Category::Dramatization => {
                mapped.extend([Category::DramatizedAdaptation, Category::FullCast]);
            }
            crate::v15::Category::Education => mapped.extend([Category::Textbook]),
            crate::v15::Category::FactualNews => mapped.extend([Category::PoliticsSociety]),
            crate::v15::Category::Fantasy => mapped.extend([Category::Fantasy]),
            crate::v15::Category::Food => mapped.extend([Category::CookingFood]),
            crate::v15::Category::Guitar
            | crate::v15::Category::IndividualSheet
            | crate::v15::Category::SheetCollection
            | crate::v15::Category::SheetCollectionMP3 => {
                mapped.extend([Category::SheetMusicScores]);
            }
            crate::v15::Category::Health => mapped.extend([Category::HealthWellness]),
            crate::v15::Category::Historical => mapped.extend([Category::Historical]),
            crate::v15::Category::Home => mapped.extend([Category::HomeGarden]),
            crate::v15::Category::Horror => mapped.extend([Category::Horror]),
            crate::v15::Category::Lgbt => mapped.extend([Category::Lgbtqia]),
            crate::v15::Category::Instructional
            | crate::v15::Category::LickLibraryLTP
            | crate::v15::Category::LickLibraryTechniques => {
                mapped.extend([Category::GuideManual]);
            }
            crate::v15::Category::Juvenile => mapped.extend([Category::Children]),
            crate::v15::Category::Language => mapped.extend([Category::LanguageLinguistics]),
            crate::v15::Category::LiteraryClassics => {
                tags.push(Self::legacy_v15_label(category));
            }
            crate::v15::Category::LiteraryFiction => {
                mapped.extend([Category::CharacterDriven]);
                tags.push(Self::legacy_v15_label(category));
            }
            crate::v15::Category::LitRPG | crate::v15::Category::ProgressionFantasy => {
                mapped.extend([Category::Fantasy, Category::ProgressionFantasy]);
            }
            crate::v15::Category::Math => mapped.extend([Category::Mathematics]),
            crate::v15::Category::Medicine => {
                mapped.extend([Category::Medicine, Category::Psychology]);
            }
            crate::v15::Category::Mystery => mapped.extend([Category::Mystery]),
            crate::v15::Category::Nature => mapped.extend([Category::NatureEnvironment]),
            crate::v15::Category::Philosophy => mapped.extend([Category::Philosophy]),
            crate::v15::Category::Poetry => mapped.extend([Category::Poetry]),
            crate::v15::Category::Politics => mapped.extend([Category::PoliticsSociety]),
            crate::v15::Category::Reference => mapped.extend([Category::Reference]),
            crate::v15::Category::Religion => {
                mapped.extend([Category::ReligionSpirituality]);
            }
            crate::v15::Category::Romance => {
                mapped.extend([Category::Romance]);
                if legacy_categories.contains(&crate::v15::Category::Thriller) {
                    mapped.extend([Category::RomanticSuspense]);
                }
                if legacy_categories.contains(&crate::v15::Category::Humor) {
                    mapped.extend([Category::RomanticComedy]);
                }
            }
            crate::v15::Category::Rpg => mapped.extend([Category::Fantasy]),
            crate::v15::Category::Science => mapped.extend([Category::Science]),
            crate::v15::Category::ScienceFiction => mapped.extend([Category::ScienceFiction]),
            crate::v15::Category::SelfHelp => mapped.extend([Category::SelfHelp]),
            crate::v15::Category::Sports => mapped.extend([Category::SportsOutdoors]),
            crate::v15::Category::Technology => mapped.extend([Category::Technology]),
            crate::v15::Category::Thriller => mapped.extend([Category::Thriller]),
            crate::v15::Category::Travel => mapped.extend([Category::Travel]),
            crate::v15::Category::UrbanFantasy => {
                mapped.extend([Category::Fantasy, Category::UrbanFantasy]);
            }
            crate::v15::Category::Western => mapped.extend([Category::Western]),
            crate::v15::Category::YoungAdult => mapped.extend([Category::YoungAdult]),
            crate::v15::Category::ContemporaryFiction => {
                mapped.extend([Category::ContemporaryRealist]);
            }
            crate::v15::Category::DramaPlays => mapped.extend([Category::DramaPlays]),
            crate::v15::Category::Paranormal => {
                if !existing_categories.contains(&Category::ParanormalRomance)
                    && !existing_categories.contains(&Category::ParanormalHorror)
                {
                    if legacy_categories.contains(&crate::v15::Category::Romance) {
                        mapped.extend([Category::ParanormalRomance]);
                    } else if legacy_categories.contains(&crate::v15::Category::Horror) {
                        mapped.extend([Category::ParanormalHorror]);
                    } else {
                        tags.push("Paranormal".to_string());
                    }
                }
            }
            crate::v15::Category::Unknown(61) => {
                mapped.extend([Category::OccultEsotericism]);
            }
            crate::v15::Category::Unknown(62) => {
                mapped.extend([Category::ContemporaryRealist, Category::CharacterDriven]);
            }
            _ => tags.push(Self::legacy_v15_label(category)),
        }

        (mapped, tags)
    }

    pub fn from_id(id: u8) -> Option<Category> {
        if let Some(legacy) = Category::legacy_label_by_id(id)
            && let Some(mapped) = Category::from_legacy_label(legacy)
        {
            return Some(mapped);
        }

        match OldCategory::from_one_id(id as u64) {
            Some(OldCategory::Audio(_)) => Some(Category::Audiobook),
            Some(OldCategory::Ebook(_)) => Some(Category::Ebook),
            Some(OldCategory::Musicology(_)) => Some(Category::Music),
            Some(OldCategory::Radio(_)) => Some(Category::DramatizedAdaptation),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Fantasy => "Fantasy",
            Category::ScienceFiction => "Science Fiction",
            Category::Romance => "Romance",
            Category::Historical => "Historical",
            Category::ContemporaryRealist => "Contemporary Realist",
            Category::Mystery => "Mystery",
            Category::Thriller => "Thriller",
            Category::Crime => "Crime",
            Category::Horror => "Horror",
            Category::ActionAdventure => "Action & Adventure",
            Category::Dystopian => "Dystopian",
            Category::PostApocalyptic => "Post-Apocalyptic",
            Category::MagicalRealism => "Magical Realism",
            Category::Western => "Western",
            Category::Military => "Military",
            Category::EpicFantasy => "Epic Fantasy",
            Category::UrbanFantasy => "Urban Fantasy",
            Category::SwordAndSorcery => "Sword & Sorcery",
            Category::HardSciFi => "Hard Sci-Fi",
            Category::SpaceOpera => "Space Opera",
            Category::Cyberpunk => "Cyberpunk",
            Category::TimeTravel => "Time Travel",
            Category::AlternateHistory => "Alternate History",
            Category::ProgressionFantasy => "Progression Fantasy",
            Category::RomanticComedy => "Romantic Comedy",
            Category::RomanticSuspense => "Romantic Suspense",
            Category::ParanormalRomance => "Paranormal Romance",
            Category::DarkRomance => "Dark Romance",
            Category::WhyChoose => "Why Choose",
            Category::Erotica => "Erotica",
            Category::Detective => "Detective",
            Category::Noir => "Noir",
            Category::LegalThriller => "Legal Thriller",
            Category::PsychologicalThriller => "Psychological Thriller",
            Category::CozyMystery => "Cozy Mystery",
            Category::BodyHorror => "Body Horror",
            Category::GothicHorror => "Gothic Horror",
            Category::CosmicHorror => "Cosmic Horror",
            Category::ParanormalHorror => "Paranormal Horror",
            Category::Lgbtqia => "LGBTQIA+",
            Category::TransRepresentation => "Trans Representation",
            Category::DisabilityRepresentation => "Disability Representation",
            Category::NeurodivergentRepresentation => "Neurodivergent Representation",
            Category::PocRepresentation => "POC Representation",
            Category::ComingOfAge => "Coming of Age",
            Category::FoundFamily => "Found Family",
            Category::EnemiesToLovers => "Enemies to Lovers",
            Category::FriendsToLovers => "Friends to Lovers",
            Category::FakeDating => "Fake Dating",
            Category::SecondChance => "Second Chance",
            Category::SlowBurn => "Slow Burn",
            Category::PoliticalIntrigue => "Political Intrigue",
            Category::Revenge => "Revenge",
            Category::Redemption => "Redemption",
            Category::Survival => "Survival",
            Category::Retelling => "Retelling",
            Category::Ancient => "Ancient",
            Category::Medieval => "Medieval",
            Category::EarlyModern => "Early Modern",
            Category::NineteenthCentury => "19th Century",
            Category::TwentiethCentury => "20th Century",
            Category::Contemporary => "Contemporary",
            Category::Future => "Future",
            Category::AlternateTimeline => "Alternate Timeline",
            Category::AlternateUniverse => "Alternate Universe",
            Category::SmallTown => "Small Town",
            Category::Urban => "Urban",
            Category::Rural => "Rural",
            Category::AcademySchool => "Academy / School",
            Category::Space => "Space",
            Category::Africa => "Africa",
            Category::EastAsia => "East Asia",
            Category::SouthAsia => "South Asia",
            Category::SoutheastAsia => "Southeast Asia",
            Category::MiddleEast => "Middle East",
            Category::Europe => "Europe",
            Category::NorthAmerica => "North America",
            Category::LatinAmerica => "Latin America",
            Category::Caribbean => "Caribbean",
            Category::Oceania => "Oceania",
            Category::Cozy => "Cozy",
            Category::Dark => "Dark",
            Category::Gritty => "Gritty",
            Category::Wholesome => "Wholesome",
            Category::Funny => "Funny",
            Category::Satire => "Satire",
            Category::Emotional => "Emotional",
            Category::CharacterDriven => "Character-Driven",
            Category::Children => "Children",
            Category::MiddleGrade => "Middle Grade",
            Category::YoungAdult => "Young Adult",
            Category::NewAdult => "New Adult",
            Category::Adult => "Adult",
            Category::Audiobook => "Audiobook",
            Category::Ebook => "Ebook",
            Category::GraphicNovelsComics => "Graphic Novels & Comics",
            Category::Manga => "Manga",
            Category::Novella => "Novella",
            Category::LightNovel => "Light Novel",
            Category::ShortStories => "Short Stories",
            Category::Anthology => "Anthology",
            Category::Poetry => "Poetry",
            Category::Essays => "Essays",
            Category::Epistolary => "Epistolary",
            Category::DramaPlays => "Drama / Plays",
            Category::FullCast => "Full Cast",
            Category::DualNarration => "Dual Narration",
            Category::DuetNarration => "Duet Narration",
            Category::DramatizedAdaptation => "Dramatized Adaptation",
            Category::AuthorNarrated => "Author Narrated",
            Category::Abridged => "Abridged",
            Category::Biography => "Biography",
            Category::Memoir => "Memoir",
            Category::History => "History",
            Category::TrueCrime => "True Crime",
            Category::Philosophy => "Philosophy",
            Category::ReligionSpirituality => "Religion & Spirituality",
            Category::MythologyFolklore => "Mythology & Folklore",
            Category::OccultEsotericism => "Occult & Esotericism",
            Category::PoliticsSociety => "Politics & Society",
            Category::Business => "Business",
            Category::PersonalFinance => "Personal Finance",
            Category::ParentingFamily => "Parenting & Family",
            Category::SelfHelp => "Self-Help",
            Category::Psychology => "Psychology",
            Category::HealthWellness => "Health & Wellness",
            Category::Science => "Science",
            Category::Technology => "Technology",
            Category::Travel => "Travel",
            Category::Mathematics => "Mathematics",
            Category::ComputerScience => "Computer Science",
            Category::DataAi => "Data & AI",
            Category::Medicine => "Medicine",
            Category::NatureEnvironment => "Nature & Environment",
            Category::Engineering => "Engineering",
            Category::ArtPhotography => "Art & Photography",
            Category::Music => "Music",
            Category::SheetMusicScores => "Sheet Music & Scores",
            Category::FilmTelevision => "Film & Television",
            Category::PopCulture => "Pop Culture",
            Category::Humor => "Humor",
            Category::LiteraryCriticism => "Literary Criticism",
            Category::CookingFood => "Cooking & Food",
            Category::HomeGarden => "Home & Garden",
            Category::CraftsDiy => "Crafts & DIY",
            Category::SportsOutdoors => "Sports & Outdoors",
            Category::Textbook => "Textbook",
            Category::Reference => "Reference",
            Category::Workbook => "Workbook",
            Category::GuideManual => "Guide / Manual",
            Category::LanguageLinguistics => "Language & Linguistics",
        }
    }

    fn normalize(value: &str) -> String {
        value
            .trim()
            .to_ascii_lowercase()
            .replace('&', " and ")
            .replace(['/', '+', '-'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn from_legacy_label(value: &str) -> Option<Category> {
        match Self::normalize(value).as_str() {
            "action" | "action adventure" => Some(Category::ActionAdventure),
            "crime" | "true crime" => Some(Category::Crime),
            "crime thriller" => Some(Category::Crime),
            "thriller" | "thriller suspense" => Some(Category::Thriller),
            "fantasy" => Some(Category::Fantasy),
            "science fiction" | "sf" => Some(Category::ScienceFiction),
            "historical" | "historical fiction" => Some(Category::Historical),
            "mystery" => Some(Category::Mystery),
            "horror" => Some(Category::Horror),
            "romance" => Some(Category::Romance),
            "urban fantasy" => Some(Category::UrbanFantasy),
            "western" => Some(Category::Western),
            "progression fantasy" | "litrpg" => Some(Category::ProgressionFantasy),
            "young adult" | "ya" => Some(Category::YoungAdult),
            "juvenile" => Some(Category::Children),
            "lgbtqia" => Some(Category::Lgbtqia),
            "comedy" => Some(Category::Funny),
            "humor" => Some(Category::Humor),
            "contemporary" | "contemporary fiction" => Some(Category::ContemporaryRealist),
            "general fiction" | "literary fiction" => Some(Category::CharacterDriven),
            "superheroes" => Some(Category::ActionAdventure),
            "art photography" => Some(Category::ArtPhotography),
            "biographical" => Some(Category::Biography),
            "business money" => Some(Category::Business),
            "complete editions music" | "music" | "music book" => Some(Category::Music),
            "computer internet" => Some(Category::ComputerScience),
            "crafts" => Some(Category::CraftsDiy),
            "dramatization full cast" => Some(Category::DramatizedAdaptation),
            "education textbook" => Some(Category::Textbook),
            "factual news current events" => Some(Category::PoliticsSociety),
            "food wine" => Some(Category::CookingFood),
            "guitar bass tabs"
            | "individual sheet"
            | "sheet collection"
            | "sheet collection mp3" => Some(Category::SheetMusicScores),
            "health fitness diet" => Some(Category::HealthWellness),
            "home garden" => Some(Category::HomeGarden),
            "instructional" | "lick library ltp jam with" | "lick library techniques ql" => {
                Some(Category::GuideManual)
            }
            "language" => Some(Category::LanguageLinguistics),
            "literary classics" => Some(Category::CharacterDriven),
            "math" => Some(Category::Mathematics),
            "medicine psychology" => Some(Category::Psychology),
            "nature" => Some(Category::NatureEnvironment),
            "paranormal" => Some(Category::ParanormalHorror),
            "philosophy" => Some(Category::Philosophy),
            "poetry" => Some(Category::Poetry),
            "politics sociology" => Some(Category::PoliticsSociety),
            "reference" => Some(Category::Reference),
            "religion spirituality" => Some(Category::ReligionSpirituality),
            "rpg" => Some(Category::Fantasy),
            "science" => Some(Category::Science),
            "self help" => Some(Category::SelfHelp),
            "sports hobbies" => Some(Category::SportsOutdoors),
            "technology" => Some(Category::Technology),
            "travel" => Some(Category::Travel),
            "drama plays" => Some(Category::DramaPlays),
            "occult metaphysical practices" => Some(Category::OccultEsotericism),
            "slice of life" => Some(Category::CharacterDriven),
            _ => None,
        }
    }

    fn legacy_label_by_id(id: u8) -> Option<&'static str> {
        match id {
            1 => Some("Action/Adventure"),
            2 => Some("Art/Photography"),
            3 => Some("Biographical"),
            4 => Some("Business/Money"),
            5 => Some("Comedy"),
            6 => Some("Complete Editions - Music"),
            7 => Some("Computer/Internet"),
            8 => Some("Crafts"),
            9 => Some("Crime"),
            10 => Some("Dramatization/Full Cast"),
            11 => Some("Education/Textbook"),
            12 => Some("Factual News/Current Events"),
            13 => Some("Fantasy"),
            14 => Some("Food/Wine"),
            15 => Some("Guitar/Bass Tabs"),
            16 => Some("Health/Fitness/Diet"),
            17 => Some("Historical"),
            18 => Some("Home/Garden"),
            19 => Some("Horror"),
            20 => Some("Humor"),
            21 => Some("Individual Sheet"),
            22 => Some("Instructional"),
            23 => Some("Juvenile"),
            24 => Some("Language"),
            25 => Some("LGBTQIA+"),
            26 => Some("Lick Library - LTP/Jam With"),
            27 => Some("Lick Library - Techniques/QL"),
            28 => Some("Literary Classics"),
            29 => Some("LitRPG"),
            30 => Some("Math"),
            31 => Some("Medicine/Psychology"),
            32 => Some("Music"),
            33 => Some("Music Book"),
            34 => Some("Mystery"),
            35 => Some("Nature"),
            36 => Some("Paranormal"),
            37 => Some("Philosophy"),
            38 => Some("Poetry"),
            39 => Some("Politics/Sociology"),
            40 => Some("Reference"),
            41 => Some("Religion/Spirituality"),
            42 => Some("Romance"),
            43 => Some("RPG"),
            44 => Some("Science"),
            45 => Some("Science Fiction"),
            46 => Some("Self-Help"),
            47 => Some("Sheet Collection"),
            48 => Some("Sheet Collection MP3"),
            49 => Some("Sports/Hobbies"),
            50 => Some("Technology"),
            51 => Some("Thriller/Suspense"),
            52 => Some("Travel"),
            53 => Some("Urban Fantasy"),
            54 => Some("Western"),
            55 => Some("Young Adult"),
            56 => Some("Superheroes"),
            57 => Some("Literary Fiction"),
            58 => Some("Progression Fantasy"),
            59 => Some("Contemporary Fiction"),
            60 => Some("Drama/Plays"),
            61 => Some("Occult / Metaphysical Practices"),
            62 => Some("Slice of Life"),
            _ => None,
        }
    }

    fn legacy_v15_label(category: crate::v15::Category) -> String {
        match category {
            crate::v15::Category::Action => "Action/Adventure".to_string(),
            crate::v15::Category::Art => "Art/Photography".to_string(),
            crate::v15::Category::Biographical => "Biographical".to_string(),
            crate::v15::Category::Business => "Business/Money".to_string(),
            crate::v15::Category::Comedy => "Comedy".to_string(),
            crate::v15::Category::CompleteEditionsMusic => "Complete Editions - Music".to_string(),
            crate::v15::Category::Computer => "Computer/Internet".to_string(),
            crate::v15::Category::Crafts => "Crafts".to_string(),
            crate::v15::Category::Crime => "Crime".to_string(),
            crate::v15::Category::Dramatization => "Dramatization/Full Cast".to_string(),
            crate::v15::Category::Education => "Education/Textbook".to_string(),
            crate::v15::Category::FactualNews => "Factual News/Current Events".to_string(),
            crate::v15::Category::Fantasy => "Fantasy".to_string(),
            crate::v15::Category::Food => "Food/Wine".to_string(),
            crate::v15::Category::Guitar => "Guitar/Bass Tabs".to_string(),
            crate::v15::Category::Health => "Health/Fitness/Diet".to_string(),
            crate::v15::Category::Historical => "Historical".to_string(),
            crate::v15::Category::Home => "Home/Garden".to_string(),
            crate::v15::Category::Horror => "Horror".to_string(),
            crate::v15::Category::Humor => "Humor".to_string(),
            crate::v15::Category::IndividualSheet => "Individual Sheet".to_string(),
            crate::v15::Category::Instructional => "Instructional".to_string(),
            crate::v15::Category::Juvenile => "Juvenile".to_string(),
            crate::v15::Category::Language => "Language".to_string(),
            crate::v15::Category::Lgbt => "LGBTQIA+".to_string(),
            crate::v15::Category::LickLibraryLTP => "Lick Library - LTP/Jam With".to_string(),
            crate::v15::Category::LickLibraryTechniques => {
                "Lick Library - Techniques/QL".to_string()
            }
            crate::v15::Category::LiteraryClassics => "Literary Classics".to_string(),
            crate::v15::Category::LitRPG => "LitRPG".to_string(),
            crate::v15::Category::Math => "Math".to_string(),
            crate::v15::Category::Medicine => "Medicine/Psychology".to_string(),
            crate::v15::Category::Music => "Music".to_string(),
            crate::v15::Category::MusicBook => "Music Book".to_string(),
            crate::v15::Category::Mystery => "Mystery".to_string(),
            crate::v15::Category::Nature => "Nature".to_string(),
            crate::v15::Category::Paranormal => "Paranormal".to_string(),
            crate::v15::Category::Philosophy => "Philosophy".to_string(),
            crate::v15::Category::Poetry => "Poetry".to_string(),
            crate::v15::Category::Politics => "Politics/Sociology".to_string(),
            crate::v15::Category::Reference => "Reference".to_string(),
            crate::v15::Category::Religion => "Religion/Spirituality".to_string(),
            crate::v15::Category::Romance => "Romance".to_string(),
            crate::v15::Category::Rpg => "RPG".to_string(),
            crate::v15::Category::Science => "Science".to_string(),
            crate::v15::Category::ScienceFiction => "Science Fiction".to_string(),
            crate::v15::Category::SelfHelp => "Self-Help".to_string(),
            crate::v15::Category::SheetCollection => "Sheet Collection".to_string(),
            crate::v15::Category::SheetCollectionMP3 => "Sheet Collection MP3".to_string(),
            crate::v15::Category::Sports => "Sports/Hobbies".to_string(),
            crate::v15::Category::Technology => "Technology".to_string(),
            crate::v15::Category::Thriller => "Thriller/Suspense".to_string(),
            crate::v15::Category::Travel => "Travel".to_string(),
            crate::v15::Category::UrbanFantasy => "Urban Fantasy".to_string(),
            crate::v15::Category::Western => "Western".to_string(),
            crate::v15::Category::YoungAdult => "Young Adult".to_string(),
            crate::v15::Category::Superheroes => "Superheroes".to_string(),
            crate::v15::Category::LiteraryFiction => "Literary Fiction".to_string(),
            crate::v15::Category::ProgressionFantasy => "Progression Fantasy".to_string(),
            crate::v15::Category::ContemporaryFiction => "Contemporary Fiction".to_string(),
            crate::v15::Category::DramaPlays => "Drama/Plays".to_string(),
            crate::v15::Category::Unknown(61) => "Occult / Metaphysical Practices".to_string(),
            crate::v15::Category::Unknown(62) => "Slice of Life".to_string(),
            crate::v15::Category::Unknown(id) => format!("Unknown Category (id: {id})"),
        }
    }
}

impl FromStr for Category {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let key = Category::normalize(value);

        for category in [
            Category::Fantasy,
            Category::ScienceFiction,
            Category::Romance,
            Category::Historical,
            Category::ContemporaryRealist,
            Category::Mystery,
            Category::Thriller,
            Category::Crime,
            Category::Horror,
            Category::ActionAdventure,
            Category::Dystopian,
            Category::PostApocalyptic,
            Category::MagicalRealism,
            Category::Western,
            Category::Military,
            Category::EpicFantasy,
            Category::UrbanFantasy,
            Category::SwordAndSorcery,
            Category::HardSciFi,
            Category::SpaceOpera,
            Category::Cyberpunk,
            Category::TimeTravel,
            Category::AlternateHistory,
            Category::ProgressionFantasy,
            Category::RomanticComedy,
            Category::RomanticSuspense,
            Category::ParanormalRomance,
            Category::DarkRomance,
            Category::WhyChoose,
            Category::Erotica,
            Category::Detective,
            Category::Noir,
            Category::LegalThriller,
            Category::PsychologicalThriller,
            Category::CozyMystery,
            Category::BodyHorror,
            Category::GothicHorror,
            Category::CosmicHorror,
            Category::ParanormalHorror,
            Category::Lgbtqia,
            Category::TransRepresentation,
            Category::DisabilityRepresentation,
            Category::NeurodivergentRepresentation,
            Category::PocRepresentation,
            Category::ComingOfAge,
            Category::FoundFamily,
            Category::EnemiesToLovers,
            Category::FriendsToLovers,
            Category::FakeDating,
            Category::SecondChance,
            Category::SlowBurn,
            Category::PoliticalIntrigue,
            Category::Revenge,
            Category::Redemption,
            Category::Survival,
            Category::Retelling,
            Category::Ancient,
            Category::Medieval,
            Category::EarlyModern,
            Category::NineteenthCentury,
            Category::TwentiethCentury,
            Category::Contemporary,
            Category::Future,
            Category::AlternateTimeline,
            Category::AlternateUniverse,
            Category::SmallTown,
            Category::Urban,
            Category::Rural,
            Category::AcademySchool,
            Category::Space,
            Category::Africa,
            Category::EastAsia,
            Category::SouthAsia,
            Category::SoutheastAsia,
            Category::MiddleEast,
            Category::Europe,
            Category::NorthAmerica,
            Category::LatinAmerica,
            Category::Caribbean,
            Category::Oceania,
            Category::Cozy,
            Category::Dark,
            Category::Gritty,
            Category::Wholesome,
            Category::Funny,
            Category::Satire,
            Category::Emotional,
            Category::CharacterDriven,
            Category::Children,
            Category::MiddleGrade,
            Category::YoungAdult,
            Category::NewAdult,
            Category::Adult,
            Category::Audiobook,
            Category::Ebook,
            Category::GraphicNovelsComics,
            Category::Manga,
            Category::Novella,
            Category::LightNovel,
            Category::ShortStories,
            Category::Anthology,
            Category::Poetry,
            Category::Essays,
            Category::Epistolary,
            Category::DramaPlays,
            Category::FullCast,
            Category::DualNarration,
            Category::DuetNarration,
            Category::DramatizedAdaptation,
            Category::AuthorNarrated,
            Category::Abridged,
            Category::Biography,
            Category::Memoir,
            Category::History,
            Category::TrueCrime,
            Category::Philosophy,
            Category::ReligionSpirituality,
            Category::MythologyFolklore,
            Category::OccultEsotericism,
            Category::PoliticsSociety,
            Category::Business,
            Category::PersonalFinance,
            Category::ParentingFamily,
            Category::SelfHelp,
            Category::Psychology,
            Category::HealthWellness,
            Category::Science,
            Category::Technology,
            Category::Travel,
            Category::Mathematics,
            Category::ComputerScience,
            Category::DataAi,
            Category::Medicine,
            Category::NatureEnvironment,
            Category::Engineering,
            Category::ArtPhotography,
            Category::Music,
            Category::SheetMusicScores,
            Category::FilmTelevision,
            Category::PopCulture,
            Category::Humor,
            Category::LiteraryCriticism,
            Category::CookingFood,
            Category::HomeGarden,
            Category::CraftsDiy,
            Category::SportsOutdoors,
            Category::Textbook,
            Category::Reference,
            Category::Workbook,
            Category::GuideManual,
            Category::LanguageLinguistics,
        ] {
            if Category::normalize(category.as_str()) == key {
                return Ok(category);
            }
        }

        Category::from_legacy_label(value).ok_or_else(|| format!("Unknown category: {value}"))
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
