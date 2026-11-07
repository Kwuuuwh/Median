use median_core::RivenTrades;
use serde::Deserialize;

/// One entry of the game's weekly riven trade statistics.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Line {
    item_type: String,
    compatibility: Option<String>,
    rerolled: bool,
    avg: f64,
    #[serde(default)]
    stddev: f64,
    min: f64,
    max: f64,
    pop: f64,
    median: f64,
}

/// Parses the game's weekly riven trade statistics, published as a JavaScript literal rather
/// than JSON; `Err` holds the parse error.
pub fn parse_riven_trades(text: &str) -> Result<Vec<RivenTrades>, String> {
    let lines: Vec<Line> = serde_json::from_str(&as_json(text)).map_err(|e| e.to_string())?;
    Ok(lines
        .into_iter()
        .map(|line| RivenTrades {
            kind: line.item_type,
            weapon: line.compatibility,
            rerolled: line.rerolled,
            median: line.median,
            average: line.avg,
            deviation: line.stddev,
            low: line.min,
            high: line.max,
            popularity: line.pop,
        })
        .collect())
}

/// Converts a JavaScript literal to JSON: bare keys quoted, single-quoted strings double-quoted.
fn as_json(text: &str) -> String {
    let mut json = String::with_capacity(text.len() + text.len() / 4);
    let mut chars = text.chars().peekable();
    while let Some(next) = chars.next() {
        match next {
            '\'' => {
                json.push('"');
                while let Some(inner) = chars.next() {
                    match inner {
                        '\\' => {
                            if let Some(escaped) = chars.next() {
                                if escaped != '\'' {
                                    json.push('\\');
                                }
                                json.push(escaped);
                            }
                        }
                        '"' => json.push_str("\\\""),
                        '\'' => break,
                        other => json.push(other),
                    }
                }
                json.push('"');
            }
            letter if letter.is_ascii_alphabetic() || letter == '_' => {
                let mut word = String::from(letter);
                while let Some(&more) = chars.peek() {
                    if more.is_ascii_alphanumeric() || more == '_' {
                        word.push(more);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let is_key = chars.clone().find(|ahead| !ahead.is_whitespace()) == Some(':');
                if is_key {
                    json.push('"');
                    json.push_str(&word);
                    json.push('"');
                } else {
                    json.push_str(&word);
                }
            }
            other => json.push(other),
        }
    }
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two entries in the format the game publishes.
    const WEEKLY: &str = "[
  {
    itemType: 'Rifle Riven Mod',
    compatibility: null,
    rerolled: false,
    avg: 123.77,
    stddev: 523.83,
    min: 5,
    max: 4600,
    pop: 7,
    median: 9
  },
  {
    itemType: 'Pistol Riven Mod',
    compatibility: 'Zymos',
    rerolled: true,
    avg: 133.75,
    stddev: 136.08,
    min: 15,
    max: 350,
    pop: 1,
    median: 85
  }
]";

    #[test]
    fn a_weekly_file_reads_veiled_kinds_and_weapons_alike() {
        let trades = parse_riven_trades(WEEKLY).expect("the file parses");

        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].weapon, None);
        assert_eq!(trades[0].kind, "Rifle Riven Mod");
        assert_eq!(trades[1].weapon.as_deref(), Some("Zymos"));
        assert!(trades[1].rerolled);
        assert_eq!(trades[1].median, 85.0);
    }

    #[test]
    fn a_name_with_a_quote_in_it_survives() {
        let trades = parse_riven_trades(
            "[{itemType: 'Rifle Riven Mod', compatibility: 'Dex\\'s \"Gun\"', rerolled: false,
               avg: 1, min: 1, max: 1, pop: 1, median: 1}]",
        )
        .expect("the file parses");

        assert_eq!(trades[0].weapon.as_deref(), Some("Dex's \"Gun\""));
    }
}
