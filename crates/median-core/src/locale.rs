/// Maximum length of an accepted BCP 47 language tag, in bytes.
const MAX_TAG: usize = 35;

/// UI locale: the lowercase language tag interface messages are looked up by.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Locale(String);

impl Default for Locale {
    fn default() -> Self {
        Self(String::from("ru"))
    }
}

impl Locale {
    /// Parses a language tag such as `"ru"` or `"pt-BR"`; `None` when the text is not a valid tag.
    /// The tag is stored lowercase so file names and settings compare without case folding.
    pub fn parse(code: &str) -> Option<Self> {
        let tag = code.trim().to_ascii_lowercase();
        if tag.len() > MAX_TAG {
            return None;
        }

        let mut parts = tag.split('-');
        let language = parts.next()?;
        if !(2..=3).contains(&language.len()) || !language.bytes().all(|b| b.is_ascii_lowercase()) {
            return None;
        }
        for part in parts {
            if !(1..=8).contains(&part.len()) || !part.bytes().all(|b| b.is_ascii_alphanumeric()) {
                return None;
            }
        }

        Some(Self(tag))
    }

    /// Lowercase language tag, e.g. `"pt-br"`.
    pub fn as_code(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::Locale;

    #[test]
    fn defaults_to_russian() {
        assert_eq!(Locale::default().as_code(), "ru");
    }

    #[test]
    fn parses_language_tags() {
        for code in ["ru", "en", "pl", "por", "pt-br", "sr-latn-rs"] {
            assert_eq!(
                Locale::parse(code).map(|l| l.as_code().to_string()),
                Some(code.to_string())
            );
        }
    }

    #[test]
    fn holds_tags_lowercase() {
        assert_eq!(Locale::parse("pt-BR").unwrap().as_code(), "pt-br");
    }

    #[test]
    fn rejects_what_is_not_a_tag() {
        for code in [
            "",
            "r",
            "русский",
            "ru_RU",
            "ru-",
            "-ru",
            "ru!",
            "toolonglanguage",
        ] {
            assert_eq!(Locale::parse(code), None, "{code:?} should not parse");
        }
    }

    #[test]
    fn code_roundtrips() {
        let locale = Locale::parse("en").unwrap();
        assert_eq!(Locale::parse(locale.as_code()), Some(locale));
    }
}
