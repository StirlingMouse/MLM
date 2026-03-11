use mlm_db::Category;

fn normalize_tag(tag: &str) -> String {
    let trimmed = tag.trim();
    let value = match trimmed.split_once(':') {
        Some((prefix, rest)) => {
            let p = prefix.trim().to_ascii_lowercase();
            if matches!(
                p.as_str(),
                "genre" | "enre" | "mood" | "tag" | "pace" | "content warning" | "general"
            ) {
                rest.trim()
            } else {
                trimmed
            }
        }
        None => trimmed,
    };

    value
        .to_ascii_lowercase()
        .replace('&', " and ")
        .replace(['/', '-', '|'], " ")
        .replace('\'', "")
        .replace([',', '.', '(', ')'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Map external string tags into the internal category taxonomy.
///
/// Returns an empty list for broad/ambiguous/noisy tags that do not map cleanly.
pub fn map_tag_to_category(tag: &str) -> Vec<Category> {
    let key = normalize_tag(tag);

    // Explicit multi-category mappings for compound tags.
    match key.as_str() {
        "contemporary romance" => return vec![Category::Contemporary, Category::Romance],
        "historical romance" => return vec![Category::Historical, Category::Romance],
        "fantasy romance" => return vec![Category::Fantasy, Category::Romance],
        "science fiction and fantasy" | "science fiction fantasy" => {
            return vec![Category::ScienceFiction, Category::Fantasy];
        }
        _ => {}
    }

    if key.contains("programming language") {
        return vec![Category::ComputerScience];
    }
    let mapped: &[Category] = match key.as_str() {
        // Kept mappings
        "fantasy" | "magic" | "fairies" | "fantasy games" => &[Category::Fantasy],
        "young adult" | "young adult fiction" | "adolescence" => &[Category::YoungAdult],
        "adventure" | "adventurous" => &[Category::ActionAdventure],
        "science fiction"
        | "aliens"
        | "extraterrestrial beings"
        | "life on other planets"
        | "human alien encounters" => &[Category::ScienceFiction],
        "strong character development" | "character driven" | "literary" => {
            &[Category::CharacterDriven]
        }
        "comics" | "graphic novels" | "comics and graphic novels" => {
            &[Category::GraphicNovelsComics]
        }
        "history" | "histoire" | "civilization" | "holocaust" | "world war ii" | "1914 1918" => {
            &[Category::History]
        }
        "emotional" | "sad" | "heartfelt" | "introspective" | "depressing" | "grief" => {
            &[Category::Emotional]
        }
        "lgbtq" | "lgbtqia" | "lgbtqia+" => &[Category::Lgbtqia],
        "dark" => &[Category::Dark],
        "romance" | "love stories" | "romance fiction" | "romantic" | "love" | "marriage"
        | "arranged marriage" | "regency romance" | "romantic suspense" => &[Category::Romance],
        "war" | "world war" | "1939 1945" | "imaginary wars and battles" => &[Category::Military],
        "mysterious" => &[Category::Mystery],
        "juvenile fiction"
        | "children"
        | "childrens stories"
        | "childrens literature"
        | "board books"
        | "picture book"
        | "boys" => &[Category::Children],
        "tense" | "suspense" | "mystery thriller" => &[Category::Thriller],
        "reflective" | "thought provoking" => &[Category::CharacterDriven],
        "funny" | "exciting" => &[Category::Funny],
        "biography" | "biography and autobiography" | "autobiography" => &[Category::Biography],
        "lighthearted" | "hopeful" | "inspiring" => &[Category::Wholesome],
        "mystery" | "detective and mystery stories" => &[Category::Mystery],
        "dystopian" => &[Category::Dystopian],
        "religion" | "spirituality" => &[Category::ReligionSpirituality],
        "juvenile nonfiction" | "education" => &[Category::Textbook],
        "space" | "astronauts" => &[Category::Space],
        "business and economics" | "business" => &[Category::Business],
        "philosophy" => &[Category::Philosophy],
        "science" | "physics" | "cosmology" | "genetic engineering" => &[Category::Science],
        "thriller" | "thriller and suspense" | "suspenseful" => &[Category::Thriller],
        "computers" | "programming" => &[Category::ComputerScience],
        "psychology" => &[Category::Psychology],
        "poetry" | "childrens poetry" | "english poetry" => &[Category::Poetry],
        "relaxing" => &[Category::Cozy],
        "humor"
        | "comedy"
        | "humorous"
        | "humorous stories"
        | "humour"
        | "american wit and humor"
        | "witty"
        | "comedians" => &[Category::Humor],
        "politics"
        | "social science"
        | "political science"
        | "politique"
        | "feminism"
        | "capitalism"
        | "communism"
        | "leadership"
        | "presidents"
        | "spies and politics"
        | "sociologie"
        | "political"
        | "sociology"
        | "anarchism"
        | "arab israeli conflict" => &[Category::PoliticsSociety],
        "travel" | "air pilots" => &[Category::Travel],
        "mathematics" | "algebra" | "calculus" => &[Category::Mathematics],
        "cooking" | "food" => &[Category::CookingFood],
        "murder" | "police" => &[Category::Crime],
        "art" | "painters" | "architects" | "drawing" | "beauty" => &[Category::ArtPhotography],
        "self help" => &[Category::SelfHelp],
        "short stories" => &[Category::ShortStories],
        "literary criticism" => &[Category::LiteraryCriticism],
        "body" | "mind and spirit" => &[Category::HealthWellness],
        "health and fitness"
        | "health"
        | "cancer"
        | "self actualization psychology"
        | "happiness"
        | "emotions"
        | "aging" => &[Category::HealthWellness],
        "literary collections" => &[Category::Anthology],
        "historical fiction" => &[Category::Historical],
        "contemporary"
        | "english fiction"
        | "domestic fiction"
        | "slice of life"
        | "genre fiction"
        | "literature and fiction"
        | "literary fiction"
        | "classique"
        | "realistic fiction"
        | "french fiction"
        | "afrikaans fiction" => &[Category::ContemporaryRealist],
        "language arts and disciplines"
        | "foreign language study"
        | "spanish"
        | "spanish language"
        | "english"
        | "french"
        | "french language"
        | "german language"
        | "turkish"
        | "italian"
        | "speech"
        | "communication" => &[Category::LanguageLinguistics],
        "nature" | "animals" | "bears" | "birds" | "dinosaurs" => &[Category::NatureEnvironment],
        "folklore" | "fairy tales" | "mythology" => &[Category::MythologyFolklore],
        "sports and recreation" | "sports" | "soccer" | "horses" | "hiking" | "baseball" => {
            &[Category::SportsOutdoors]
        }
        "fast paced" => &[Category::ActionAdventure],
        "medical" => &[Category::Medicine],
        "performing arts" | "drama" | "english drama" | "verse novel" | "plays" => {
            &[Category::DramaPlays]
        }
        "manga" => &[Category::Manga],
        "cyberpunk" => &[Category::Cyberpunk],
        "crime" | "true crime" => &[Category::TrueCrime],
        "music" => &[Category::Music],
        "technology and engineering" | "aeronautics" | "automobiles" => &[Category::Engineering],
        "horror" | "horror tales" | "scary" | "horreur" => &[Category::Horror],
        "architecture" | "design" | "photography" => &[Category::ArtPhotography],
        "fantasy fiction" => &[Category::Fantasy],
        "crafts and hobbies" => &[Category::CraftsDiy],
        "adventure stories"
        | "action and adventure"
        | "adventure and adventurers"
        | "action"
        | "aventure" => &[Category::ActionAdventure],
        "reference" => &[Category::Reference],
        "urban fantasy" | "paranormal and urban" => &[Category::UrbanFantasy],
        "games and activities" | "games" | "roleplaying games" => &[Category::SportsOutdoors],
        "audiobook" | "audio book" | "audiobooks" | "kinder hörbücher" | "childrens audiobooks" => {
            &[Category::Audiobook]
        }
        "electronic books" => &[Category::Ebook],
        "holiday" | "christmas" => &[Category::Wholesome],
        "great britain" | "england" | "europe" | "british" | "germany" | "london england"
        | "ireland" | "greece" | "italy" | "scotland" | "rome" | "portugal" | "poland"
        | "berlin germany" | "soviet union" | "russia" => &[Category::Europe],
        "gardening" | "house and home" => &[Category::HomeGarden],
        "memoir" => &[Category::Memoir],
        "bible" | "bibles" | "christian life" | "christian fiction" => {
            &[Category::ReligionSpirituality]
        }
        "epic fantasy" => &[Category::EpicFantasy],
        "military" => &[Category::Military],
        "boys love" | "bl" | "yaoi" | "gay men" => &[Category::Lgbtqia],
        "young adult nonfiction" | "teen and young adult" | "jeune adulte" => {
            &[Category::YoungAdult]
        }
        "american" | "native americans" | "american fiction" | "americans" => {
            &[Category::NorthAmerica]
        }
        "authors" => &[Category::LiteraryCriticism],
        "english language" | "fiction in english" => &[Category::LanguageLinguistics],
        "historical" => &[Category::Historical],
        "middle grade" => &[Category::MiddleGrade],
        "australia" | "australian fiction" | "australian" => &[Category::Oceania],
        "american poetry" => &[Category::Poetry],
        "france" => &[Category::Europe],
        "china" | "chinese" => &[Category::EastAsia],
        "japan" | "japanese" => &[Category::EastAsia],
        "india" => &[Category::SouthAsia],
        "egypt" | "arabic fiction" => &[Category::MiddleEast],
        "africa" => &[Category::Africa],
        "united states" | "california" | "canada" | "canadian" | "colorado" | "new york n y"
        | "arizona" | "alaska" | "america" => &[Category::NorthAmerica],
        "brazil" => &[Category::LatinAmerica],
        "christmas stories" => &[Category::Wholesome],
        "occult" => &[Category::OccultEsotericism],
        "demonology" => &[Category::OccultEsotericism],
        "erotic stories" => &[Category::Erotica],
        "erotica" => &[Category::Erotica],
        "romantasy" => &[Category::Fantasy, Category::Romance],
        "dragons" => &[Category::Fantasy],
        "conduct of life" => &[Category::SelfHelp],
        "modern" => &[Category::Contemporary],
        "transportation" => &[Category::Travel],
        "space opera" | "first contact" => &[Category::SpaceOpera],
        "assassins" | "mafia" | "missing persons" | "kidnapping" | "abduction" => {
            &[Category::Crime]
        }
        "dark romance" | "dark romance kink" => &[Category::DarkRomance],
        "study aids" => &[Category::Workbook],
        "adult" | "adulte" => &[Category::Adult],
        "paranormal romance" | "omegaverse" | "amish romance" => &[Category::ParanormalRomance],
        "monster romance" => &[Category::ParanormalRomance],
        "espionage" => &[Category::PoliticalIntrigue],
        "conspiracies" => &[Category::PoliticalIntrigue],
        "artists" => &[Category::ArtPhotography],
        "actors"
        | "actresses"
        | "motion picture actors and actresses"
        | "motion pictures"
        | "motion picture producers and directors" => &[Category::FilmTelevision],
        "paranormal" | "paranormal fiction" | "supernatural" | "vampires" | "ghost stories"
        | "ghosts" | "angels" => &[Category::ParanormalHorror],
        "slow paced" => &[Category::Cozy],
        "time travel" => &[Category::TimeTravel],
        "magical realism" => &[Category::MagicalRealism],
        "dystopias" => &[Category::Dystopian],
        "criminals" | "crime fiction" => &[Category::Crime],
        "thrillers" | "suspense fiction" | "crime thrillers" => &[Category::Thriller],
        "philosophie" => &[Category::Philosophy],
        "found family" => &[Category::FoundFamily],
        "chick lit" => &[Category::RomanticComedy],
        "women sleuths" => &[Category::Detective],
        "city and town life" => &[Category::Urban],
        "college students" => &[Category::NewAdult],
        "caricatures and cartoons" | "fantasy comic books" | "pictorial" => {
            &[Category::GraphicNovelsComics]
        }
        "artificial intelligence" => &[Category::DataAi],
        "businessmen" | "economics" | "business enterprises" | "businesswomen" => {
            &[Category::Business]
        }
        "country life" | "frontier and pioneer life" => &[Category::Rural],
        "coming of age" | "bildungsromans" => &[Category::ComingOfAge],
        "high fantasy" => &[Category::EpicFantasy],
        "psychological" | "amnesia" | "psychological thriller" => {
            &[Category::PsychologicalThriller]
        }
        "books and reading" | "authorship" => &[Category::LiteraryCriticism],
        "anthology" => &[Category::Anthology],
        "essays" => &[Category::Essays],
        "novella" => &[Category::Novella],
        "java" | "javascript" | "c++" | "python" | "application software" => {
            &[Category::ComputerScience]
        }
        "sapphic" | "queer" | "lesbians" | "shounen ai" => &[Category::Lgbtqia],
        "rock musicians" => &[Category::Music],
        "christianity" | "buddhism" | "amish" => &[Category::ReligionSpirituality],
        "criminal investigation"
        | "private investigators"
        | "cold cases criminal investigation"
        | "mystery and detective"
        | "detective" => &[Category::Detective],
        "hard science fiction"
        | "speculative fiction"
        | "sci fi"
        | "doctor who fictitious character" => &[Category::ScienceFiction],
        "imaginary places" => &[Category::Fantasy],
        "astronomy" | "interplanetary voyages" => &[Category::Space],
        "dark fantasy" => &[Category::Fantasy],
        "litrpg" => &[Category::ProgressionFantasy],
        "mental health" | "brain" | "ability" => &[Category::HealthWellness],
        "cowboys" | "american western romance" => &[Category::Western],
        "cookbooks" => &[Category::CookingFood],
        "dreams" => &[Category::Psychology],
        "blessing and cursing" => &[Category::ReligionSpirituality],
        "high school students" => &[Category::AcademySchool],
        "mythical"
        | "dragons and mythical creatures"
        | "curiosities and wonders"
        | "gods"
        | "arthurian romances" => &[Category::MythologyFolklore],
        "african american women" => &[Category::PocRepresentation],
        "indians of north america" => &[Category::NorthAmerica],
        "audio theater" | "hörspiel" => &[Category::DramatizedAdaptation],
        "cozy" | "bed and breakfast accommodations" | "birthdays" => &[Category::Cozy],
        "war and military" | "battle of" | "soldiers" | "guerre" => &[Category::Military],
        "chicago ill" | "boston mass" => &[Category::NorthAmerica],
        "literature and fiction science fiction and fantasy" => {
            &[Category::ScienceFiction, Category::Fantasy]
        }
        "batman fictitious character" | "superheroes" | "science fiction comic books" => {
            &[Category::GraphicNovelsComics]
        }
        "german" | "greek" | "russian" => &[Category::LanguageLinguistics],
        "jewish 1939 1945" => &[Category::History],
        "german fiction" | "japanese fiction" | "chinese fiction" => {
            &[Category::ContemporaryRealist]
        }
        "light novel" => &[Category::LightNovel],
        "computer networks" | "technology" | "computer games" | "computer adventure games" => {
            &[Category::Technology]
        }
        "diaries" => &[Category::Memoir],
        "retellings" | "retelling" => &[Category::Retelling],
        "gothic" => &[Category::GothicHorror],
        "artistic" | "aesthetics" => &[Category::ArtPhotography],
        "ethics" => &[Category::Philosophy],
        "series" | "anthologies" | "anthologies and short stories" => &[Category::Anthology],
        "islam" | "spiritual life" => &[Category::ReligionSpirituality],
        "urban" | "cities and towns" => &[Category::Urban],
        "ancient" => &[Category::Ancient],
        "medieval" | "castles" => &[Category::Medieval],
        "child rearing" => &[Category::ParentingFamily],
        "psychological fiction" => &[Category::PsychologicalThriller],
        "traditional detectives" | "amateur sleuths" | "police procedural" => {
            &[Category::Detective]
        }
        "cozy mystery" => &[Category::CozyMystery],
        "heroes" => &[Category::ActionAdventure],
        "alphabet"
        | "bedtime"
        | "picture books for children"
        | "readers"
        | "girls"
        | "babysitters" => &[Category::Children],
        "intelligence officers" => &[Category::PoliticalIntrigue],
        "cults" => &[Category::OccultEsotericism],
        "dungeons and dragons game" => &[Category::ProgressionFantasy],
        "sexy" => &[Category::Erotica],
        "climatic changes" | "agriculture" | "farm life" | "dwellings" => {
            &[Category::NatureEnvironment]
        }
        "romantic comedy" => &[Category::RomanticComedy],
        "post apocalyptic" | "end of the world" => &[Category::PostApocalyptic],
        "satire" => &[Category::Satire],
        "democracy" => &[Category::PoliticsSociety],
        "monsters" => &[Category::Horror],
        "biology" => &[Category::Science],
        "clothing and dress" => &[Category::CraftsDiy],
        "adult fiction" => &[Category::Adult],
        "western" => &[Category::Western],
        "jews" => &[Category::ReligionSpirituality],
        "businesspeople" | "entrepreneurship" => &[Category::Business],
        "new zealand" => &[Category::Oceania],
        "anxiety" => &[Category::Psychology],
        "literature and fiction mystery" => &[Category::Mystery],
        "bandes dessinées" => &[Category::GraphicNovelsComics],
        "afghanistan" | "iran" => &[Category::MiddleEast],
        "technothrillers" => &[Category::Thriller],
        "gothic horror" => &[Category::GothicHorror],
        "computer science" => &[Category::ComputerScience],
        "slow burn" => &[Category::SlowBurn],
        "blind" => &[Category::DisabilityRepresentation],
        "novelists" | "journalism" | "poets" | "college teachers" | "composers" => {
            &[Category::LiteraryCriticism]
        }
        "pirates" => &[Category::ActionAdventure],
        "christian biography" => &[Category::Biography],
        "courtship" => &[Category::Romance],
        "mexico" => &[Category::LatinAmerica],
        "alternative histories fiction" => &[Category::AlternateHistory],
        "historical fantasy" => &[Category::Historical],
        "magical" => &[Category::Fantasy],
        "antiques and collectibles" => &[Category::CraftsDiy],

        _ => &[],
    };

    mapped.to_vec()
}

#[cfg(test)]
mod tests {
    use super::map_tag_to_category;
    use mlm_db::Category;

    #[test]
    fn maps_selected_tags() {
        assert_eq!(map_tag_to_category("Fantasy"), vec![Category::Fantasy]);
        assert_eq!(map_tag_to_category("  Fantasy "), vec![Category::Fantasy]);
        assert_eq!(
            map_tag_to_category("Character driven"),
            vec![Category::CharacterDriven]
        );
        assert_eq!(
            map_tag_to_category("Comics & Graphic Novels"),
            vec![Category::GraphicNovelsComics]
        );
        assert_eq!(
            map_tag_to_category("Business & Economics"),
            vec![Category::Business]
        );
        assert_eq!(map_tag_to_category("LGBTQ"), vec![Category::Lgbtqia]);
        assert_eq!(map_tag_to_category("Boy's Love"), vec![Category::Lgbtqia]);
        assert_eq!(map_tag_to_category("Manga"), vec![Category::Manga]);
        assert_eq!(
            map_tag_to_category("Technology & Engineering"),
            vec![Category::Engineering]
        );
        assert_eq!(map_tag_to_category("Audio book"), vec![Category::Audiobook]);
        assert_eq!(
            map_tag_to_category("English language"),
            vec![Category::LanguageLinguistics]
        );
        assert_eq!(
            map_tag_to_category("Paranormal Romance"),
            vec![Category::ParanormalRomance]
        );
        assert_eq!(
            map_tag_to_category("Genre: Rust (Programming Language)"),
            vec![Category::ComputerScience]
        );
        assert_eq!(
            map_tag_to_category("Genre: C# (Programming Language)"),
            vec![Category::ComputerScience]
        );
        assert_eq!(
            map_tag_to_category("Genre: Light Novel"),
            vec![Category::LightNovel]
        );
        assert_eq!(map_tag_to_category("enre: Ireland"), vec![Category::Europe]);
    }

    #[test]
    fn maps_compound_tags_to_multiple_categories() {
        assert_eq!(
            map_tag_to_category("Contemporary Romance"),
            vec![Category::Contemporary, Category::Romance]
        );
        assert_eq!(
            map_tag_to_category("Historical Romance"),
            vec![Category::Historical, Category::Romance]
        );
        assert_eq!(
            map_tag_to_category("Fantasy Romance"),
            vec![Category::Fantasy, Category::Romance]
        );
    }

    #[test]
    fn drops_ambiguous_tags() {
        assert_eq!(map_tag_to_category("Fiction"), Vec::<Category>::new());
        assert_eq!(map_tag_to_category("Nonfiction"), Vec::<Category>::new());
        assert_eq!(map_tag_to_category("medium"), Vec::<Category>::new());
        assert_eq!(map_tag_to_category("A mix driven"), Vec::<Category>::new());
        assert_eq!(map_tag_to_category("etc"), Vec::<Category>::new());
        assert_eq!(map_tag_to_category("Rape"), Vec::<Category>::new());
        assert_eq!(
            map_tag_to_category("Sexual violence"),
            Vec::<Category>::new()
        );
        assert_eq!(map_tag_to_category("Finance"), Vec::<Category>::new());
        assert_eq!(map_tag_to_category("Law"), Vec::<Category>::new());
        assert_eq!(map_tag_to_category("Asia"), Vec::<Category>::new());
    }
}
