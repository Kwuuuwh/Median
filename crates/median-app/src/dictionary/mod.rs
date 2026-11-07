mod canon;

pub(crate) use canon::{canon, letters};

/// Maximum edit distance at which an OCR reading still matches a name: 1, plus 1 for every 4
/// characters of the reading.
fn reach(letters: usize) -> usize {
    1 + letters / 4
}

/// Closed dictionary of item names for OCR matching, each stored in normalized form.
pub(crate) struct Pool {
    /// Normalization applied to both the names and the readings.
    fold: fn(&str) -> String,
    /// Game key of each item and its name normalized by `fold`.
    entries: Vec<(String, Vec<char>)>,
}

impl Pool {
    /// Builds the dictionary from catalog `(key, name)` pairs in the game client's language,
    /// normalized by `canon`.
    pub(crate) fn new(names: Vec<(String, String)>) -> Self {
        Self::folded(names, canon)
    }

    /// Builds the dictionary with `fold` as the normalization of names and readings.
    pub(crate) fn folded(names: Vec<(String, String)>, fold: fn(&str) -> String) -> Self {
        Self {
            fold,
            entries: names
                .into_iter()
                .map(|(item, name)| (item, fold(&name).chars().collect()))
                .collect(),
        }
    }

    /// Key of the item an OCR reading matches; `None` when no single item matches.
    ///
    /// An exact match after normalization is returned at once. Otherwise the closest name
    /// matches only if it is within `reach` **and** strictly closer than any other name: a tie
    /// means two names fit the reading equally well, and returning whichever came first would
    /// be a guess.
    pub(crate) fn nearest(&self, text: &str) -> Option<&str> {
        let asking: Vec<char> = (self.fold)(text).chars().collect();
        if asking.is_empty() {
            return None;
        }

        let ceiling = reach(asking.len());
        let (mut best, mut runner_up) = (None, usize::MAX);
        for (item, name) in &self.entries {
            let far = distance(&asking, name, ceiling);
            match best {
                Some((nearest, _)) if far >= nearest => runner_up = runner_up.min(far),
                Some((nearest, _)) => {
                    runner_up = nearest;
                    best = Some((far, item.as_str()));
                }
                None => best = Some((far, item.as_str())),
            }
            if far == 0 {
                return Some(item.as_str());
            }
        }

        match best {
            Some((far, item)) if far <= ceiling && far < runner_up => Some(item),
            _ => None,
        }
    }
}

/// Edit distance between two texts after both are normalized by `canon`.
pub(crate) fn apart(one: &str, other: &str) -> usize {
    let (one, other): (Vec<char>, Vec<char>) =
        (canon(one).chars().collect(), canon(other).chars().collect());
    distance(&one, &other, one.len().max(other.len()))
}

/// Levenshtein distance between two names, capped: beyond `ceiling` it returns `ceiling + 1`,
/// which is enough for a caller comparing names and lets most dictionary entries be rejected
/// after a row or two.
fn distance(one: &[char], two: &[char], ceiling: usize) -> usize {
    if one.len().abs_diff(two.len()) > ceiling {
        return ceiling + 1;
    }

    let mut row: Vec<usize> = (0..=two.len()).collect();
    for (down, here) in one.iter().enumerate() {
        let mut corner = row[0];
        row[0] = down + 1;
        let mut least = row[0];
        for (across, there) in two.iter().enumerate() {
            let above = row[across + 1];
            row[across + 1] = (corner + usize::from(here != there))
                .min(above + 1)
                .min(row[across] + 1);
            corner = above;
            least = least.min(row[across + 1]);
        }
        if least > ceiling {
            return ceiling + 1;
        }
    }
    row[two.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Names the readings below refer to, plus names that a looser match confuses with them.
    const NAMES: [(&str, &str); 12] = [
        ("yareli-chassis", "Ярели Прайм: Каркас (Чертеж)"),
        ("yareli", "Ярели Прайм (Чертеж)"),
        ("lavos-chassis", "Лавос Прайм: Каркас (Чертеж)"),
        ("caliban-chassis", "Калибан Прайм: Каркас (Чертеж)"),
        ("voruna-chassis", "Воруна Прайм: Каркас (Чертеж)"),
        ("voruna", "Воруна Прайм (Чертеж)"),
        ("forma", "Форма (Чертеж)"),
        ("braton-stock", "Брэйтон Прайм: Приклад"),
        ("braton-barrel", "Брэйтон Прайм: Ствол"),
        ("afentis-handle", "Афентис Прайм: Рукоять"),
        ("gara", "Гара Прайм (Чертеж)"),
        ("kara", "Кара Прайм (Чертеж)"),
    ];

    fn pool() -> Pool {
        Pool::new(
            NAMES
                .iter()
                .map(|(item, name)| ((*item).to_owned(), (*name).to_owned()))
                .collect(),
        )
    }

    /// Real OCR output from ten reward screens, with the item each reading refers to.
    #[test]
    fn names_what_the_recogniser_read_off_the_screen() {
        let pool = pool();
        let read = [
            ("Чертёж:Ярели Прайм: Kapkac", "yareli-chassis"),
            ("Чертёж:ЛавосПрайм: Kapkac", "lavos-chassis"),
            ("ЧертежКалибан Прайм:Каркас", "caliban-chassis"),
            ("Чертёж Воруна Прайм: Kapkac", "voruna-chassis"),
            ("Чертеж Воруна Прайм: Kapkac", "voruna-chassis"),
            ("2 X Чертёж:Форма", "forma"),
            ("2 X Чертёж: Форма", "forma"),
            ("БрэйтонПрайм: Приклад", "braton-stock"),
            ("АфентисПрайм: Рукоять", "afentis-handle"),
        ];

        for (text, want) in read {
            assert_eq!(pool.nearest(text), Some(want), "reading {text:?}");
        }
    }

    /// These readings need the edit-distance limit: a match that requires every word intact
    /// matches none of them.
    #[test]
    fn names_a_reading_the_model_damaged_inside_a_word() {
        let pool = pool();

        assert_eq!(
            pool.nearest("Чертёж: Воруна Прайм: Каркао"),
            Some("voruna-chassis")
        );
        assert_eq!(pool.nearest("Брэйтом Прайм: Прикпад"), Some("braton-stock"));
        assert_eq!(
            pool.nearest("Афентис Праим: Рукоят"),
            Some("afentis-handle")
        );
    }

    /// Two names one edit apart are why a match must be strictly closer than the runner-up.
    #[test]
    fn says_nothing_when_two_names_stand_equally_near() {
        let pool = pool();

        assert_eq!(pool.nearest("Мара Прайм (Чертеж)"), None);
        assert_eq!(pool.nearest("Гара Прайм (Чертеж)"), Some("gara"));
        assert_eq!(pool.nearest("Кара Прайм (Чертеж)"), Some("kara"));
    }

    #[test]
    fn says_nothing_when_nothing_in_the_pool_is_near() {
        let pool = pool();

        assert_eq!(pool.nearest("Мод на прочность щитов"), None);
        assert_eq!(pool.nearest(""), None);
    }

    #[test]
    fn an_exact_name_is_taken_before_anything_is_measured() {
        assert_eq!(
            pool().nearest("Брэйтон Прайм: Ствол"),
            Some("braton-barrel")
        );
    }

    #[test]
    fn a_name_further_off_than_the_ceiling_answers_past_it() {
        let one: Vec<char> = "каркас".chars().collect();
        let two: Vec<char> = "рукоять".chars().collect();

        assert_eq!(distance(&one, &two, 2), 3);
        assert_eq!(distance(&one, &one, 2), 0);
        assert_eq!(distance(&one, &"карkас".chars().collect::<Vec<_>>(), 2), 1);
    }
}
