use std::collections::HashMap;

use crate::{FlagBits, Flags};

impl Flags {
    pub fn from_bitfield(field: u8) -> Flags {
        Flags {
            crude_language: Some(field & (1 << 1) > 0),
            violence: Some(field & (1 << 2) > 0),
            some_explicit: Some(field & (1 << 3) > 0),
            explicit: Some(field & (1 << 4) > 0),
            abridged: Some(field & (1 << 5) > 0),
            lgbt: Some(field & (1 << 6) > 0),
        }
    }

    pub fn as_search_bitfield(&self) -> (bool, Vec<u8>) {
        let shows = self.fields().filter(|f| f.unwrap_or(false)).count();
        let hides = self.fields().filter(|f| f.is_some_and(|f| !f)).count();
        let is_hide = hides > shows;
        let mut field = vec![];
        if self.crude_language.is_some_and(|f| f != is_hide) {
            field.push(1 << 1);
        }
        if self.violence.is_some_and(|f| f != is_hide) {
            field.push(1 << 2);
        }
        if self.some_explicit.is_some_and(|f| f != is_hide) {
            field.push(1 << 3);
        }
        if self.explicit.is_some_and(|f| f != is_hide) {
            field.push(1 << 4);
        }
        if self.abridged.is_some_and(|f| f != is_hide) {
            field.push(1 << 5);
        }
        if self.lgbt.is_some_and(|f| f != is_hide) {
            field.push(1 << 6);
        }
        (is_hide, field)
    }

    pub fn as_bitfield(&self) -> u8 {
        let mut field = 0;
        if self.crude_language.unwrap_or_default() {
            field += 1 << 1;
        }
        if self.violence.unwrap_or_default() {
            field += 1 << 2;
        }
        if self.some_explicit.unwrap_or_default() {
            field += 1 << 3;
        }
        if self.explicit.unwrap_or_default() {
            field += 1 << 4;
        }
        if self.abridged.unwrap_or_default() {
            field += 1 << 5;
        }
        if self.lgbt.unwrap_or_default() {
            field += 1 << 6;
        }
        field
    }

    pub fn matches(&self, other: &Flags) -> bool {
        self.fields()
            .zip(other.fields())
            .all(|(t, o)| t.is_none_or(|_| t == o))
    }

    fn fields(&self) -> impl Iterator<Item = Option<bool>> {
        [
            self.crude_language,
            self.violence,
            self.some_explicit,
            self.explicit,
            self.abridged,
            self.lgbt,
        ]
        .into_iter()
    }
}

impl From<Flags> for FlagBits {
    fn from(value: Flags) -> Self {
        FlagBits::new(value.as_bitfield())
    }
}

impl From<FlagBits> for Flags {
    fn from(value: FlagBits) -> Self {
        Flags::from_bitfield(value.0)
    }
}
impl TryFrom<HashMap<String, bool>> for Flags {
    type Error = String;

    fn try_from(value: HashMap<String, bool>) -> Result<Self, Self::Error> {
        let mut flags = Flags::default();
        for (key, value) in value.into_iter() {
            match key.to_lowercase().as_str() {
                "crude" => flags.crude_language = Some(value),
                "language" => flags.crude_language = Some(value),
                "crude language" => flags.crude_language = Some(value),
                "violence" => flags.violence = Some(value),
                "some explicit" => flags.some_explicit = Some(value),
                "explicit" => flags.explicit = Some(value),
                "abridged" => flags.abridged = Some(value),
                "lgbt" => flags.lgbt = Some(value),
                _ => return Err(format!("invalid flag {key}")),
            }
        }
        Ok(flags)
    }
}

impl std::fmt::Display for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut flags = vec![];
        if self.crude_language == Some(true) {
            flags.push("crude language");
        }
        if self.violence == Some(true) {
            flags.push("violence");
        }
        if self.some_explicit == Some(true) {
            flags.push("some explicit");
        }
        if self.explicit == Some(true) {
            flags.push("explicit");
        }
        if self.abridged == Some(true) {
            flags.push("abridged");
        }
        if self.lgbt == Some(true) {
            flags.push("lgbt");
        }
        write!(f, "{}", flags.join(", "))?;
        Ok(())
    }
}
