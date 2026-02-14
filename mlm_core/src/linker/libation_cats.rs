use std::collections::BTreeSet;

use mlm_db::Category;

use super::folder::CategoryLadder;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CategoryMapping {
    pub categories: Vec<Category>,
    pub freeform_tags: Vec<String>,
}

impl CategoryMapping {
    pub fn is_empty(&self) -> bool {
        self.categories.is_empty() && self.freeform_tags.is_empty()
    }

    fn push_category(&mut self, category: Category) {
        if !self.categories.contains(&category) {
            self.categories.push(category);
        }
    }

    fn push_tag(&mut self, tag: &str) {
        if !self.freeform_tags.iter().any(|existing| existing == tag) {
            self.freeform_tags.push(tag.to_string());
        }
    }

    #[cfg(test)]
    fn extend(&mut self, other: CategoryMapping) {
        for category in other.categories {
            self.push_category(category);
        }
        for tag in other.freeform_tags {
            self.push_tag(&tag);
        }
    }
}

fn mapped(categories: &[Category], tags: &[&str]) -> CategoryMapping {
    let mut out = CategoryMapping::default();
    for category in categories {
        out.push_category(*category);
    }
    for tag in tags {
        out.push_tag(tag);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MappingDepth {
    ExactFullPath,
    FallbackTwoLevel,
    FallbackTopLevel,
    Unmapped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderMatch {
    pub original_path: Vec<String>,
    pub matched_path: Vec<String>,
    pub depth: MappingDepth,
    pub mapping: CategoryMapping,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AggregateCategoryResult {
    pub categories: Vec<Category>,
    pub freeform_tags: Vec<String>,
    pub ladder_matches: Vec<LadderMatch>,
    pub unmapped_paths: Vec<Vec<String>>,
}

pub fn map_audible_es_path_exact(path: &[&str]) -> CategoryMapping {
    use Category::*;

    match path {
        [
            "Literatura y ficción",
            "Literatura de género",
            "Coming of age",
        ] => mapped(&[ComingOfAge], &["Genre Fiction"]),
        _ => CategoryMapping::default(),
    }
}

pub fn map_audible_es_path(path: &[&str]) -> CategoryMapping {
    use Category::*;

    match path {
        [
            "Literatura y ficción",
            "Literatura de género",
            "Coming of age",
        ] => mapped(&[ComingOfAge], &["Genre Fiction"]),
        ["Literatura y ficción", "Clásicos"] => mapped(&[], &["Classics"]),
        ["Policíaca, negra y suspense", "Novela negra"] => mapped(&[Crime, Noir], &[]),

        ["Adolescentes"] => mapped(&[YoungAdult], &[]),
        ["Adolescentes", leaf] => {
            let mut out = mapped(&[YoungAdult], &[]);
            match *leaf {
                "Biografías" => out.push_category(Biography),
                "Ciencia ficción y fantasía" => out.push_tag("Science Fiction & Fantasy"),
                "Deportes y aire libre" => out.push_category(SportsOutdoors),
                "LGBTQ+" => out.push_category(Lgbtqia),
                "Literatura y ficción" => out.push_tag("Literature & Fiction"),
                "Policíaca, negra y suspense" => out.push_tag("Crime, Noir & Suspense"),
                "Romántica" => out.push_category(Romance),
                "Salud, estilo de vida y relaciones" => {
                    out.push_category(HealthWellness);
                    out.push_tag("Lifestyle & Relationships");
                }
                _ => {}
            }
            out
        }

        ["Arte y entretenimiento"] => mapped(&[], &["Arts & Entertainment"]),
        ["Arte y entretenimiento", leaf] => match *leaf {
            "Arte" => mapped(&[ArtPhotography], &[]),
            "Audiciones y dramatizaciones" => mapped(&[], &["Auditions & Dramatizations"]),
            "Entretenimiento y artes escénicas" => {
                mapped(&[], &["Entertainment & Performing Arts"])
            }
            "Música" => mapped(&[Music], &[]),
            _ => CategoryMapping::default(),
        },

        ["Audiolibros infantiles"] => mapped(&[Children, Audiobook], &[]),
        ["Audiolibros infantiles", leaf] => {
            let mut out = mapped(&[Children, Audiobook], &[]);
            match *leaf {
                "Acción y aventura" => out.push_category(ActionAdventure),
                "Actividades y aficiones" => out.push_tag("Activities & Hobbies"),
                "Animales y naturaleza" => out.push_category(NatureEnvironment),
                "Biografías" => out.push_category(Biography),
                "Ciencia ficción y fantasía" => out.push_tag("Science Fiction & Fantasy"),
                "Ciencia y tecnología" => out.push_tag("Science & Technology"),
                "Crecer y cosas de la vida" => out.push_tag("Growing Up & Life"),
                "Cuentos y leyendas" => out.push_tag("Stories & Legends"),
                "Deportes y aire libre" => out.push_category(SportsOutdoors),
                "Educación y formación" => out.push_tag("Education & Learning"),
                "Fiestas y celebraciones" => out.push_tag("Holidays & Celebrations"),
                "Geografía y culturas" => out.push_tag("Geography & Cultures"),
                "Historia" => out.push_category(History),
                "Humor" => out.push_category(Funny),
                "Literatura y ficción" => out.push_tag("Literature & Fiction"),
                "Misterio y suspense" => {
                    out.push_category(Mystery);
                    out.push_tag("Mystery & Suspense");
                }
                "Música y artes escénicas" => {
                    out.push_category(Music);
                    out.push_tag("Performing Arts");
                }
                "Religión" => out.push_category(ReligionSpirituality),
                "Vehículos y transporte" => out.push_tag("Vehicles & Transportation"),
                _ => {}
            }
            out
        }

        ["Biografías y memorias"] => mapped(&[Biography, Memoir], &[]),
        ["Biografías y memorias", leaf] => {
            let mut out = mapped(&[Biography, Memoir], &[]);
            match *leaf {
                "Arte y literatura" => out.push_tag("Art & Literature"),
                "Aventureros, exploradores y supervivencia" => {
                    out.push_tag("Adventurers, Explorers & Survival");
                }
                "Celebridades del entretenimiento" => {
                    out.push_tag("Entertainment Celebrities");
                }
                "Crímenes reales" => out.push_category(TrueCrime),
                "Culturales y regionales" => out.push_tag("Cultural & Regional"),
                "Deportes" => out.push_category(SportsOutdoors),
                "Ejército y guerra" => out.push_category(Military),
                "Histórico" => out.push_category(History),
                "LGBT" => out.push_category(Lgbtqia),
                "Mujeres" => out.push_tag("Women"),
                "Política y activismo" => {
                    out.push_category(PoliticsSociety);
                    out.push_tag("Activism");
                }
                "Profesionales y académicos" => out.push_tag("Professionals & Academics"),
                "Religiones" => out.push_category(ReligionSpirituality),
                _ => {}
            }
            out
        }

        ["Ciencia e ingeniería"] => mapped(&[], &["Science & Engineering"]),
        ["Ciencia e ingeniería", "Ciencia"] => mapped(&[Science], &[]),
        ["Ciencia e ingeniería", "Ingeniería"] => mapped(&[Engineering], &[]),

        ["Ciencia ficción y fantasía"] => mapped(&[], &["Science Fiction & Fantasy"]),
        ["Ciencia ficción y fantasía", "Ciencia ficción"] => mapped(&[ScienceFiction], &[]),
        ["Ciencia ficción y fantasía", "Fantasía"] => mapped(&[Fantasy], &[]),

        ["Comedia y humor"] => mapped(&[], &["Comedy & Humor"]),
        ["Comedia y humor", leaf] => match *leaf {
            "Artes escénicas" => mapped(&[Humor], &["Performing Arts Comedy"]),
            "Literatura y ficción" => mapped(&[Funny], &["Comedy & Humor"]),
            _ => CategoryMapping::default(),
        },

        ["Deportes y aire libre"] => mapped(&[SportsOutdoors], &[]),
        ["Deportes y aire libre", leaf] => {
            let mut out = mapped(&[SportsOutdoors], &[]);
            match *leaf {
                "Aire libre y naturaleza" => out.push_category(NatureEnvironment),
                "Aventureros, exploradores y supervivencia" => {
                    out.push_tag("Adventurers, Explorers & Survival");
                }
                "Baloncesto" => out.push_tag("Basketball"),
                "Biografías y memorias" => {
                    out.push_category(Biography);
                    out.push_category(Memoir);
                }
                "Béisbol y softbol" => out.push_tag("Baseball & Softball"),
                "Culturismo y entrenamiento muscular" => {
                    out.push_category(HealthWellness);
                    out.push_tag("Bodybuilding & Strength Training");
                }
                "Ensayos y comentarios" => out.push_category(Essays),
                "Entrenamiento" => {
                    out.push_category(GuideManual);
                    out.push_category(HealthWellness);
                }
                "Fútbol" => out.push_tag("Soccer"),
                "Fútbol americano" => out.push_tag("American Football"),
                "Historia del deporte" => out.push_category(History),
                "Juegos Olímpicos y Paralímpicos" => out.push_tag("Olympics & Paralympics"),
                _ => {}
            }
            out
        }

        ["Dinero y finanzas"] => mapped(&[], &["Money & Finance"]),
        ["Dinero y finanzas", leaf] => match *leaf {
            "Comercio electrónico" => mapped(&[Business, Technology], &["E-Commerce"]),
            "Economía" => mapped(&[Business], &["Economics"]),
            "Finanzas personales" => mapped(&[PersonalFinance], &[]),
            "Internacional" => mapped(&[], &["International Finance"]),
            "Inversiones y valores" => mapped(&[PersonalFinance], &["Investing & Securities"]),
            _ => CategoryMapping::default(),
        },

        ["Educación y formación"] => mapped(&[], &["Education & Learning"]),
        ["Educación y formación", leaf] => match *leaf {
            "Aprendizaje de idiomas" => mapped(&[LanguageLinguistics], &[]),
            "Educación" => mapped(&[], &["Education"]),
            "Lengua y gramática" => mapped(&[LanguageLinguistics], &[]),
            "Redacción y publicación" => mapped(
                &[LanguageLinguistics, GuideManual],
                &["Writing & Publishing"],
            ),
            _ => CategoryMapping::default(),
        },

        ["Erótica"] => mapped(&[Erotica], &[]),
        ["Erótica", leaf] => match *leaf {
            "Educación sexual" => mapped(&[HealthWellness], &["Sex Education"]),
            "Literatura y ficción" => mapped(&[Romance, Erotica], &[]),
            _ => CategoryMapping::default(),
        },

        ["Historia"] => mapped(&[History], &[]),
        ["Historia", leaf] => {
            let mut out = mapped(&[History], &[]);
            match *leaf {
                "América" => out.push_tag("Americas"),
                "Antigua" => out.push_category(Ancient),
                "Asia" => out.push_tag("Asia"),
                "Era moderna" => out.push_category(EarlyModern),
                "Europa" => out.push_category(Europe),
                "LGBTQ+" => out.push_category(Lgbtqia),
                "Militar" => out.push_category(Military),
                "Mundial" => out.push_tag("World History"),
                "Rusia" => {
                    out.push_category(Europe);
                    out.push_tag("Russia");
                }
                _ => {}
            }
            out
        }

        ["Hogar y jardín"] => mapped(&[HomeGarden], &[]),
        ["Hogar y jardín", leaf] => match *leaf {
            "Casa y hogar" => mapped(&[HomeGarden], &[]),
            "Comida y vino" => mapped(&[CookingFood], &[]),
            "Jardinería y horticultura" => mapped(&[HomeGarden], &["Gardening & Horticulture"]),
            "Mascotas y cuidado de animales" => mapped(&[HomeGarden], &["Pets & Animal Care"]),
            "Vida sostenible y ecológica" => {
                mapped(&[HomeGarden, NatureEnvironment], &["Sustainable Living"])
            }
            _ => CategoryMapping::default(),
        },

        ["Informática y tecnología"] => mapped(&[Technology], &[]),
        ["Informática y tecnología", leaf] => match *leaf {
            "Creación de contenido y redes sociales" => {
                mapped(&[Technology], &["Content Creation & Social Media"])
            }
            "Historia y cultura" => mapped(&[Technology], &["Technology History & Culture"]),
            _ => CategoryMapping::default(),
        },

        ["LGBTQ+"] => mapped(&[Lgbtqia], &[]),
        ["LGBTQ+", leaf] => {
            let mut out = mapped(&[Lgbtqia], &[]);
            match *leaf {
                "Biografías y memorias" => {
                    out.push_category(Biography);
                    out.push_category(Memoir);
                }
                "Ciencia ficción y fantasía" => out.push_tag("Science Fiction & Fantasy"),
                "Estudios sobre LGBTQ+" => {
                    out.push_category(PoliticsSociety);
                    out.push_tag("LGBTQ+ Studies");
                }
                "Historia" => out.push_category(History),
                "Literatura y ficción" => {}
                "Misterio, negra y suspense" => out.push_tag("Mystery, Crime & Suspense"),
                "Romántica" => out.push_category(Romance),
                _ => {}
            }
            out
        }

        ["Literatura y ficción"] => mapped(&[], &["Literature & Fiction"]),
        ["Literatura y ficción", leaf] => match *leaf {
            "Acción y aventura" => mapped(&[ActionAdventure], &[]),
            "Afroamericana" => mapped(&[PocRepresentation], &["African American"]),
            "Antologías y relatos breves" => mapped(&[Anthology, ShortStories], &[]),
            "Clásicos" => mapped(&[], &["Classics"]),
            "Drama y teatro" => mapped(&[DramaPlays], &[]),
            "Ensayos" => mapped(&[Essays], &[]),
            "Erótica" => mapped(&[Romance, Erotica], &[]),
            "Humor y sátira" => mapped(&[Funny, Satire], &[]),
            "LGBT" => mapped(&[Lgbtqia], &[]),
            "Literatura antigua, clásica y medieval" => {
                mapped(&[Ancient, Medieval], &["Classical Literature"])
            }
            "Literatura de género" => mapped(&[], &["Genre Fiction"]),
            "Narrativa femenina" => mapped(&[], &["Women's Fiction"]),
            "Novela histórica" => mapped(&[Historical], &[]),
            "Poesía" => mapped(&[Poetry], &[]),
            "Terror" => mapped(&[Horror], &[]),
            _ => CategoryMapping::default(),
        },

        ["Negocios y profesiones"] => mapped(&[Business], &[]),
        ["Negocios y profesiones", leaf] => {
            let mut out = mapped(&[Business], &[]);
            match *leaf {
                "Comportamiento organizacional y en el lugar de trabajo" => {
                    out.push_tag("Workplace & Organizational Behavior");
                }
                "Desarrollo empresarial y emprendimiento" => {}
                "Gestión y liderazgo" => out.push_tag("Management & Leadership"),
                "Marketing y ventas" => out.push_tag("Marketing & Sales"),
                "Mujeres en los negocios" => out.push_tag("Women in Business"),
                "Éxito profesional" => {
                    out.push_category(SelfHelp);
                    out.push_tag("Career Success");
                }
                _ => {}
            }
            out
        }

        ["Policíaca, negra y suspense"] => mapped(&[], &["Crime, Noir & Suspense"]),
        ["Policíaca, negra y suspense", leaf] => match *leaf {
            "Crímenes reales" => mapped(&[TrueCrime], &[]),
            "Misterio" => mapped(&[Mystery], &[]),
            "Negra y suspense" => mapped(&[Crime, Noir, Thriller], &[]),
            "Novela negra" => mapped(&[Crime, Noir], &[]),
            _ => CategoryMapping::default(),
        },

        ["Política y ciencias sociales"] => mapped(&[PoliticsSociety], &[]),
        ["Política y ciencias sociales", leaf] => {
            let mut out = mapped(&[PoliticsSociety], &[]);
            match *leaf {
                "Antropología" => out.push_tag("Anthropology"),
                "Ciencias sociales" => {}
                "Legislación" => out.push_tag("Law"),
                "Política y gobierno" => {}
                "Sociología" => {}
                _ => {}
            }
            out
        }

        ["Relaciones, crianza y desarrollo personal"] => {
            mapped(&[], &["Relationships, Parenting & Personal Development"])
        }
        ["Relaciones, crianza y desarrollo personal", leaf] => match *leaf {
            "Crianza y familia" => mapped(&[ParentingFamily], &[]),
            "Desarrollo personal" => mapped(&[SelfHelp], &[]),
            _ => CategoryMapping::default(),
        },

        ["Religión y espiritualidad"] => mapped(&[ReligionSpirituality], &[]),
        ["Religión y espiritualidad", leaf] => {
            let mut out = mapped(&[ReligionSpirituality], &[]);
            match *leaf {
                "Budismo" => out.push_tag("Buddhism"),
                "Cristiandad" => out.push_tag("Christianity"),
                "Espiritualidad" => {}
                "Estudios religiosos" => out.push_tag("Religious Studies"),
                "Hinduismo" => out.push_tag("Hinduism"),
                "Islam" => out.push_tag("Islam"),
                "Judaísmo" => out.push_tag("Judaism"),
                "Ocultismo" => out.push_category(OccultEsotericism),
                "Otras religiones, prácticas y textos" => {
                    out.push_tag("Other Religions, Practices & Texts");
                }
                _ => {}
            }
            out
        }

        ["Romántica"] => mapped(&[Romance], &[]),
        ["Romántica", leaf] => {
            let mut out = mapped(&[Romance], &[]);
            match *leaf {
                "Acción y aventura" => out.push_category(ActionAdventure),
                "Antologías y relatos breves" => {
                    out.push_category(Anthology);
                    out.push_category(ShortStories);
                }
                "Ciencia ficción" => out.push_category(ScienceFiction),
                "Comedia romántica" => out.push_category(RomanticComedy),
                "Contemporánea" => out.push_category(Contemporary),
                "Cortejo" => out.push_tag("Courtship"),
                "Fantástico" => out.push_category(Fantasy),
                "Histórico" => out.push_category(Historical),
                "LGBT" => out.push_category(Lgbtqia),
                "Militar" => out.push_category(Military),
                "Multicultural" => {
                    out.push_category(PocRepresentation);
                    out.push_tag("Multicultural");
                }
                "Oeste americano" => out.push_category(Western),
                "Suspense romántico" => out.push_category(RomanticSuspense),
                _ => {}
            }
            out
        }

        ["Salud y bienestar"] => mapped(&[HealthWellness], &[]),
        ["Salud y bienestar", leaf] => {
            let mut out = mapped(&[HealthWellness], &[]);
            match *leaf {
                "Adicción y recuperación" => {
                    out.push_category(SelfHelp);
                    out.push_tag("Addiction & Recovery");
                }
                "Ejercicio, dieta y nutrición" => {}
                "Enfermedad física y trastornos" => {
                    out.push_category(Medicine);
                    out.push_tag("Physical Illness & Disorders");
                }
                "Envejecimiento y longevidad" => out.push_tag("Aging & Longevity"),
                "Higiene y vida sana" => {}
                "Medicina y sector de la salud" => out.push_category(Medicine),
                "Psicología y salud mental" => out.push_category(Psychology),
                "Salud sexual y reproductiva" => {
                    out.push_tag("Sexual & Reproductive Health");
                }
                _ => {}
            }
            out
        }

        ["Viajes y turismo"] => mapped(&[Travel], &[]),
        ["Viajes y turismo", leaf] => {
            let mut out = mapped(&[Travel], &[]);
            match *leaf {
                "Asia" => out.push_tag("Asia"),
                "Europa" => out.push_category(Europe),
                "Oriente Medio" => out.push_category(MiddleEast),
                "Reportajes y artículos" => {
                    out.push_category(Essays);
                    out.push_tag("Travel Writing & Journalism");
                }
                "Viajes de aventura" => {
                    out.push_category(ActionAdventure);
                    out.push_tag("Adventure Travel");
                }
                "Visitas guiadas" => out.push_category(GuideManual),
                "África" => out.push_category(Africa),
                _ => {}
            }
            out
        }

        _ => CategoryMapping::default(),
    }
}

pub fn map_audible_es_ladder(path: &[&str]) -> LadderMatch {
    let original_path: Vec<String> = path.iter().map(|segment| (*segment).to_string()).collect();

    let exact = map_audible_es_path_exact(path);
    if !exact.is_empty() {
        return LadderMatch {
            original_path,
            matched_path: path.iter().map(|segment| (*segment).to_string()).collect(),
            depth: MappingDepth::ExactFullPath,
            mapping: exact,
        };
    }

    if path.len() >= 2 {
        let two = map_audible_es_path(&path[..2]);
        if !two.is_empty() {
            return LadderMatch {
                original_path,
                matched_path: path[..2]
                    .iter()
                    .map(|segment| (*segment).to_string())
                    .collect(),
                depth: MappingDepth::FallbackTwoLevel,
                mapping: two,
            };
        }
    }

    if !path.is_empty() {
        let one = map_audible_es_path(&path[..1]);
        if !one.is_empty() {
            return LadderMatch {
                original_path,
                matched_path: path[..1]
                    .iter()
                    .map(|segment| (*segment).to_string())
                    .collect(),
                depth: MappingDepth::FallbackTopLevel,
                mapping: one,
            };
        }
    }

    LadderMatch {
        original_path,
        matched_path: Vec::new(),
        depth: MappingDepth::Unmapped,
        mapping: CategoryMapping::default(),
    }
}

pub fn map_category_ladders(ladders: &[CategoryLadder]) -> AggregateCategoryResult {
    let mut categories = BTreeSet::new();
    let mut freeform_tags = BTreeSet::new();
    let mut ladder_matches = Vec::new();
    let mut unmapped_paths = Vec::new();

    for ladder in ladders {
        if ladder.root != "Genres" {
            continue;
        }

        let path: Vec<&str> = ladder
            .ladder
            .iter()
            .map(|node| node.name.as_str())
            .collect();
        let ladder_match = map_audible_es_ladder(&path);

        if ladder_match.depth == MappingDepth::Unmapped {
            unmapped_paths.push(ladder_match.original_path.clone());
        }

        for category in &ladder_match.mapping.categories {
            categories.insert(*category);
        }
        for tag in &ladder_match.mapping.freeform_tags {
            freeform_tags.insert(tag.clone());
        }

        ladder_matches.push(ladder_match);
    }

    AggregateCategoryResult {
        categories: categories.into_iter().collect(),
        freeform_tags: freeform_tags.into_iter().collect(),
        ladder_matches,
        unmapped_paths,
    }
}

pub fn three_plus_override_candidates(ladders: &[CategoryLadder]) -> Vec<LadderMatch> {
    ladders
        .iter()
        .filter(|ladder| ladder.root == "Genres" && ladder.ladder.len() >= 3)
        .map(|ladder| {
            let path: Vec<&str> = ladder
                .ladder
                .iter()
                .map(|node| node.name.as_str())
                .collect();
            map_audible_es_ladder(&path)
        })
        .filter(|ladder_match| ladder_match.depth != MappingDepth::ExactFullPath)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linker::folder::Ladder;

    #[test]
    fn maps_exact_override_before_fallback() {
        let result = map_audible_es_ladder(&[
            "Literatura y ficción",
            "Literatura de género",
            "Coming of age",
        ]);

        assert_eq!(result.depth, MappingDepth::ExactFullPath);
        assert!(result.mapping.categories.contains(&Category::ComingOfAge));
        assert!(
            result
                .mapping
                .freeform_tags
                .contains(&"Genre Fiction".to_string())
        );
    }

    #[test]
    fn reports_three_level_fallback_as_override_candidate() {
        let ladders = vec![CategoryLadder {
            root: "Genres".to_string(),
            ladder: vec![
                Ladder {
                    id: "1".to_string(),
                    name: "Literatura y ficción".to_string(),
                },
                Ladder {
                    id: "2".to_string(),
                    name: "Clásicos".to_string(),
                },
                Ladder {
                    id: "3".to_string(),
                    name: "Europeos".to_string(),
                },
            ],
        }];

        let candidates = three_plus_override_candidates(&ladders);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].depth, MappingDepth::FallbackTwoLevel);
        assert_eq!(
            candidates[0].matched_path,
            vec!["Literatura y ficción".to_string(), "Clásicos".to_string()]
        );
    }

    #[test]
    fn aggregates_categories_and_tags_from_genre_ladders() {
        let mut first = mapped(&[Category::Crime], &["One"]);
        first.extend(mapped(&[Category::Noir], &["Two"]));

        assert_eq!(first.categories, vec![Category::Crime, Category::Noir]);
        assert_eq!(
            first.freeform_tags,
            vec!["One".to_string(), "Two".to_string()]
        );
    }
}
