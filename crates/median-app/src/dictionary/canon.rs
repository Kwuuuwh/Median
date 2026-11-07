/// Latin letters that look like Cyrillic ones: the OCR model outputs either, and game names
/// contain both, so each is replaced by its Cyrillic look-alike. English names still match,
/// because both sides are normalized the same way and an English name always normalizes to
/// the same form.
const TWINS: [(char, char); 12] = [
    ('a', 'а'),
    ('b', 'в'),
    ('c', 'с'),
    ('e', 'е'),
    ('h', 'н'),
    ('k', 'к'),
    ('m', 'м'),
    ('o', 'о'),
    ('p', 'р'),
    ('t', 'т'),
    ('x', 'х'),
    ('y', 'у'),
];

/// Digits the OCR model outputs in place of letters. No relic reward name contains a digit
/// (all 595 checked), so a digit left in a reading after the leading count is removed is an
/// OCR error.
const FIGURES: [(char, char); 4] = [('0', 'о'), ('3', 'з'), ('4', 'ч'), ('6', 'б')];

/// Normalized form of a name for matching: leading count removed, words the OCR model merged
/// split apart, case and `ё` folded, look-alike letters and digits mapped to one letter,
/// punctuation dropped and the words sorted.
///
/// Sorting is what makes the two sides comparable: the game shows `Чертёж: Ярели Прайм:
/// Каркас` while the catalog has `Ярели Прайм: Каркас (Чертеж)`, and the sorted words match
/// without a rule for either side's word order.
pub(crate) fn canon(text: &str) -> String {
    let mut words: Vec<&str> = Vec::new();
    let folded: String = median_core::fold(&parted(without_count(text)))
        .chars()
        .map(letter)
        .collect();

    words.extend(
        folded
            .split(|character: char| !character.is_alphanumeric())
            .filter(|word| !word.is_empty()),
    );
    words.sort_unstable();
    words.join(" ")
}

/// Letters and digits of a phrase in order, mapped as in `canon`, with spaces and punctuation
/// removed.
pub(crate) fn letters(text: &str) -> String {
    median_core::fold(text)
        .chars()
        .map(letter)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

/// Maps a look-alike Latin letter or digit to its Cyrillic letter; other characters are kept.
fn letter(character: char) -> char {
    let twin = TWINS
        .iter()
        .chain(FIGURES.iter())
        .find(|(drawn, _)| *drawn == character);
    match twin {
        Some((_, written)) => *written,
        None => character,
    }
}

/// The name without the count the game shows before a reward that drops more than one copy,
/// as in `2 X Чертёж: Форма`.
fn without_count(text: &str) -> &str {
    let rest = text.trim_start();
    let digits = rest.trim_start_matches(|character: char| character.is_ascii_digit());
    if digits.len() == rest.len() {
        return text;
    }

    let times = digits.trim_start();
    let Some(after) = times.strip_prefix(['x', 'X', 'х', 'Х']) else {
        return text;
    };
    match after.starts_with(char::is_whitespace) {
        true => after.trim_start(),
        false => text,
    }
}

/// The reading with merged words split apart, as in `ЛавосПрайм`. Runs before case folding,
/// because the case change is the only sign of the word boundary.
fn parted(text: &str) -> String {
    let mut parted = String::with_capacity(text.len() + 4);
    let mut before: Option<char> = None;

    for letter in text.chars() {
        let broken = letter.is_uppercase()
            && before.is_some_and(|before| before.is_lowercase() || before.is_ascii_digit());
        if broken {
            parted.push(' ');
        }
        parted.push(letter);
        before = Some(letter);
    }
    parted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screens_wording_and_the_catalogs_meet() {
        assert_eq!(
            canon("Чертёж: Ярели Прайм: Каркас"),
            canon("Ярели Прайм: Каркас (Чертеж)")
        );
        assert_eq!(canon("2 X Чертёж: Форма"), canon("Форма (Чертеж)"));
    }

    #[test]
    fn sets_apart_words_the_recogniser_ran_together() {
        assert_eq!(canon("ЛавосПрайм: Каркас"), canon("Лавос Прайм: Каркас"));
        assert_eq!(
            canon("ЧертежКалибан Прайм:Каркас"),
            canon("Калибан Прайм: Каркас (Чертеж)")
        );
    }

    #[test]
    fn writes_a_latin_twin_as_the_letter_it_stands_for() {
        assert_eq!(canon("Kapkac"), canon("Каркас"));
        assert_eq!(
            canon("Брэйтон Пpайм: Пpиклад"),
            canon("Брэйтон Прайм: Приклад")
        );
    }

    /// Normalization works for English too: an English name matches an English name, whichever
    /// alphabet the OCR model used.
    #[test]
    fn an_english_name_folds_the_same_on_both_sides() {
        assert_eq!(canon("Вraton Prime Stock"), canon("Braton Prime Stock"));
    }

    #[test]
    fn writes_a_digit_the_recogniser_drew_as_the_letter() {
        assert_eq!(canon("Ф0рма"), canon("Форма"));
    }

    /// A letter that looks like a multiplier is one only after a count. Letters without a
    /// look-alike stay unchanged, so the normalized form mixes alphabets; that is harmless,
    /// because catalog names are normalized by the same rule.
    #[test]
    fn leaves_a_name_that_only_looks_like_a_count() {
        assert_eq!(canon("Xiphos"), "хiрноs");
        assert_eq!(canon("2 Прайм"), canon("Прайм 2"));
        assert_eq!(canon("2 X Xiphos"), canon("Xiphos"));
    }

    #[test]
    fn a_reading_of_nothing_canons_to_nothing() {
        assert_eq!(canon(""), "");
        assert_eq!(canon("   :  "), "");
    }
}
