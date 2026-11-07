use crate::rivens::{CardText, ShownLine};

/// Prefixes of a multiplier stat value, as in `x1.58`.
const TIMES: [char; 3] = ['x', 'х', '×'];

/// Signs a stat value can have.
const SIGNS: [char; 3] = ['+', '-', '−'];

/// Builds the card text from its OCR lines: lines before the first stat form the name, and a
/// line without a leading number continues the previous stat.
pub fn card_text(lines: &[String]) -> CardText {
    let mut text = CardText::default();
    for line in lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
    {
        match (opening(line), text.stats.last_mut()) {
            (Some((number, words)), _) => text.stats.push(ShownLine {
                number,
                words: words.to_owned(),
            }),
            (None, Some(last)) => {
                last.words.push(' ');
                last.words.push_str(line);
            }
            (None, None) => {
                if !text.name.is_empty() && !text.name.ends_with('-') {
                    text.name.push(' ');
                }
                text.name.push_str(line);
            }
        }
    }
    text
}

/// Characters the OCR model may output before a stat's sign, such as its icon read as `0`
/// or `.`.
const STRAY: usize = 2;

/// Signed number at the start of a stat line and the words after it; `None` when the line does
/// not start with a sign or multiplier (after at most `STRAY` non-letter characters).
fn opening(line: &str) -> Option<(f64, &str)> {
    let (at, _) = line
        .char_indices()
        .take(STRAY + 1)
        .find(|(_, char)| SIGNS.contains(char) || TIMES.contains(char))?;
    if line[..at].chars().any(char::is_alphabetic) {
        return None;
    }
    let line = &line[at..];
    let (sign, rest) = match line.chars().next()? {
        first if SIGNS.contains(&first) => (
            if first == '+' { 1.0 } else { -1.0 },
            &line[first.len_utf8()..],
        ),
        first if TIMES.contains(&first) => (1.0, &line[first.len_utf8()..]),
        _ => return None,
    };
    let rest = rest.trim_start();
    let end = rest
        .find(|char: char| !(char.is_ascii_digit() || char == '.' || char == ','))
        .unwrap_or(rest.len());
    let number: f64 = rest[..end].replace(',', ".").parse().ok()?;
    let words = rest[end..].trim_start_matches('%').trim();
    Some((sign * number, words))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(read: &[&str]) -> Vec<String> {
        read.iter().map(|line| (*line).to_owned()).collect()
    }

    #[test]
    fn name_lines_join_and_wrapped_stats_rejoin_their_number() {
        let text = card_text(&lines(&[
            "Парные.",
            "Токсоцисты Сrопи-",
            "saiis",
            "+162.4% мультивыстрел",
            "+121.4% крит.урона",
            " +102.9% K",
            "скорострельности",
            "+86% к отдаче",
        ]));

        assert_eq!(text.name, "Парные. Токсоцисты Сrопи-saiis");
        assert_eq!(text.stats.len(), 4);
        assert_eq!(text.stats[2].number, 102.9);
        assert_eq!(text.stats[2].words, "K скорострельности");
        assert_eq!(text.stats[3].number, 86.0);
    }

    #[test]
    fn a_curse_keeps_its_sign_and_a_faction_its_multiplier() {
        let text = card_text(&lines(&[
            "Огрис Igni-argibin",
            "-15.3% урона",
            "x1,45 урона Гринир",
        ]));

        assert_eq!(text.stats[0].number, -15.3);
        assert_eq!(text.stats[1].number, 1.45);
        assert_eq!(text.stats[1].words, "урона Гринир");
    }

    #[test]
    fn a_stat_icon_read_before_the_sign_stays_out_of_the_name() {
        let text = card_text(&lines(&[
            "Зенит Satiata",
            "0+124.4% мультивыстрел",
            ".+218.4% урона",
            "-31.2% урона Корпусу",
        ]));

        assert_eq!(text.name, "Зенит Satiata");
        assert_eq!(text.stats.len(), 3);
        assert_eq!(text.stats[0].number, 124.4);
        assert_eq!(text.stats[1].words, "урона");
    }
}
