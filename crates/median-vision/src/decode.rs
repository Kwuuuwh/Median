/// The characters the model outputs, one per line, in the order of its output classes.
const CHARACTERS: &str = include_str!("../assets/eslav.dict");

/// The blank class: no character, and the separator between repeats of one character.
const BLANK: usize = 0;

/// Recognised text of one line of a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub text: String,
    /// Mean score of the kept characters; zero when the text is empty.
    pub confidence: f32,
}

/// The classes the model scores: a blank, then the characters, then a space.
pub(crate) struct Alphabet(Vec<char>);

impl Alphabet {
    pub(crate) fn new() -> Self {
        Self(
            CHARACTERS
                .lines()
                .filter_map(|line| line.chars().next())
                .collect(),
        )
    }

    pub(crate) fn classes(&self) -> usize {
        self.0.len() + 2
    }

    /// The character of a class; `None` for the blank or a class out of range.
    fn character(&self, class: usize) -> Option<char> {
        match class {
            BLANK => None,
            _ if class <= self.0.len() => Some(self.0[class - 1]),
            _ if class == self.0.len() + 1 => Some(' '),
            _ => None,
        }
    }

    /// Decodes the best class of every step into text, dropping blanks and collapsing runs of
    /// one class into one character.
    pub(crate) fn spell(&self, steps: &[(usize, f32)]) -> Reading {
        let mut text = String::new();
        let mut scored = 0.0;
        let mut kept = 0usize;
        let mut previous = usize::MAX;

        for &(class, score) in steps {
            if class != previous
                && let Some(character) = self.character(class)
            {
                text.push(character);
                scored += score;
                kept += 1;
            }
            previous = class;
        }

        Reading {
            text,
            confidence: match kept {
                0 => 0.0,
                _ => scored / kept as f32,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_alphabet_answers_as_many_classes_as_the_model_scores() {
        assert_eq!(Alphabet::new().classes(), 519);
    }

    #[test]
    fn a_run_of_one_class_spells_one_character() {
        let alphabet = Alphabet::new();
        let a = 1 + alphabet.0.iter().position(|&c| c == 'a').expect("an a");

        let reading = alphabet.spell(&[(a, 0.9), (a, 0.7), (BLANK, 0.5), (a, 0.5)]);

        assert_eq!(reading.text, "aa");
        assert!((reading.confidence - 0.7).abs() < 1e-6);
    }

    #[test]
    fn a_line_of_blanks_reads_empty() {
        let reading = Alphabet::new().spell(&[(BLANK, 0.9), (BLANK, 0.9)]);

        assert_eq!(reading.text, "");
        assert_eq!(reading.confidence, 0.0);
    }
}
