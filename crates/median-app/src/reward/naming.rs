use std::collections::{HashMap, HashSet};

use median_core::{Instant, Offered, Owned, Reached};
use median_ipc::Good;
use tracing::warn;

use super::Card;
use crate::catalog::{self, Catalog};
use crate::dictionary::{Pool, canon};
use crate::market::{self, Store};
use crate::state_sink::SharedHoldings;

/// One reward screen card, matched against the catalog and priced from the market cache.
#[derive(Debug, Clone)]
pub struct Found {
    /// Text recognised on the card.
    pub text: String,
    pub confidence: f32,
    /// Catalog item the text matches; `None` when no catalog name matches.
    pub good: Option<Good>,
    /// Estimated value in platinum; `None` when the market cache has too few closed trades.
    pub platinum: Option<u32>,
    /// Lowest price asked by a seller who is in game, from the order book last fetched into the
    /// market cache.
    pub ingame: Option<u32>,
    /// Market pair `(slug, variant)` of the card, for reading its prices again.
    pub pair: Option<(String, String)>,
    /// When the market cache last fetched the pair's open orders.
    pub looked: Option<Instant>,
    /// Copies traded per day, in tenths; `None` when the market cache has no data for it.
    pub per_day_tenths: Option<u32>,

    /// Parts of the item's set, with the count of each part the account holds.
    pub kit: Vec<Owned>,
    /// Price of the whole set; `None` when the market cache has no price for it.
    pub whole: Option<u32>,
    /// Whether the item, or the item its set builds, is marked a favourite.
    pub favourite: bool,
    /// Progress of the account with the item the card's set builds.
    pub reached: Option<Reached>,
}

impl Found {
    /// Unique name of the matched catalog item; `None` when the card matched nothing.
    pub fn item(&self) -> Option<&str> {
        self.good.as_ref().map(|good| good.item.as_str())
    }

    /// The card in the form sent to the window; an unmatched card shows its recognised text.
    pub fn offered(&self, own: bool) -> Offered {
        match &self.good {
            Some(good) => Offered {
                item: good.item.clone(),
                name: good.name.clone(),
                image: good.image.clone(),
                platinum: self.platinum,
                ingame: self.ingame,
                per_day_tenths: self.per_day_tenths,
                ducats: good.ducats,
                own,
                favourite: self.favourite,
                kit: self.kit.clone(),
                whole: self.whole,
                reached: self.reached,
            },
            None => Offered {
                item: String::new(),
                name: self.text.clone(),
                image: None,
                platinum: None,
                ingame: None,
                per_day_tenths: None,
                ducats: None,
                own,
                favourite: false,
                kit: Vec::new(),
                whole: None,
                reached: None,
            },
        }
    }
}

/// The same cards priced again: the recognition is kept, only the market cache data is read again.
pub(super) async fn repriced(
    catalog: &Catalog,
    market_cache: &Store,
    holdings: &SharedHoldings,
    favourites: &HashSet<String>,
    lang: &str,
    cards: &[Found],
) -> Vec<Found> {
    let known: Vec<String> = cards
        .iter()
        .filter_map(|card| card.item().map(str::to_owned))
        .collect();
    let (kits, whole, built) = sets(catalog, market_cache, holdings, lang, &known).await;
    let reached = reached(catalog, holdings, &built).await;
    let quotes = match market::quoted(catalog, market_cache, &known).await {
        Ok(quotes) => quotes,
        Err(why) => {
            warn!(error = %why, "reward prices read failed");
            Vec::new()
        }
    };

    cards
        .iter()
        .map(|card| {
            let Some(good) = card.good.as_ref() else {
                return card.clone();
            };
            let quote = quotes.iter().find(|quote| quote.item == good.item);
            Found {
                platinum: quote.and_then(|quote| quote.platinum).or(card.platinum),
                ingame: quote.and_then(|quote| quote.ingame),
                looked: quote.and_then(|quote| quote.looked).or(card.looked),
                per_day_tenths: quote
                    .map(|quote| median_ipc::tenths(quote.prospect.per_day))
                    .or(card.per_day_tenths),
                pair: quote
                    .map(|quote| (quote.slug.clone(), quote.variant.clone()))
                    .or_else(|| card.pair.clone()),
                kit: kits.get(&good.item).cloned().unwrap_or_default(),
                whole: whole.get(&good.item).copied().flatten(),
                favourite: favourites.contains(&good.item)
                    || built
                        .get(&good.item)
                        .is_some_and(|whole| favourites.contains(whole)),
                reached: reached.get(&good.item).copied(),
                ..card.clone()
            }
        })
        .collect()
}

/// Chooses the grid the screen shows, matches each of its cards against the catalog and prices
/// it from the market cache, keeping the cards in screen order.
pub(super) async fn found(
    catalog: &Catalog,
    market_cache: &Store,
    holdings: &SharedHoldings,
    favourites: &HashSet<String>,
    lang: &str,
    ours: Option<&str>,
    rows: &[Vec<Card>],
) -> Vec<Found> {
    let (cards, keys, goods) = named(catalog, lang, ours, rows).await;
    let known: Vec<String> = keys.iter().flatten().cloned().collect();
    let (kits, whole, built) = sets(catalog, market_cache, holdings, lang, &known).await;
    let reached = reached(catalog, holdings, &built).await;

    let quotes = match market::quoted(catalog, market_cache, &known).await {
        Ok(quotes) => quotes,
        Err(why) => {
            warn!(error = %why, "reward prices read failed");
            Vec::new()
        }
    };

    cards
        .iter()
        .zip(keys)
        .map(|(card, key)| {
            let good = key.and_then(|key| goods.iter().find(|good| good.item == key).cloned());
            let quote = good
                .as_ref()
                .and_then(|good| quotes.iter().find(|quote| quote.item == good.item));
            let platinum = quote.and_then(|quote| quote.platinum);
            let ingame = quote.and_then(|quote| quote.ingame);
            let looked = quote.and_then(|quote| quote.looked);
            let per_day_tenths = quote.map(|quote| median_ipc::tenths(quote.prospect.per_day));
            let pair = quote.map(|quote| (quote.slug.clone(), quote.variant.clone()));

            let kit = good
                .as_ref()
                .and_then(|good| kits.get(&good.item))
                .cloned()
                .unwrap_or_default();

            let of = good
                .as_ref()
                .and_then(|good| whole.get(&good.item))
                .copied()
                .flatten();

            let favourite = good.as_ref().is_some_and(|good| {
                favourites.contains(&good.item)
                    || built
                        .get(&good.item)
                        .is_some_and(|whole| favourites.contains(whole))
            });
            let reached = good
                .as_ref()
                .and_then(|good| reached.get(&good.item))
                .copied();

            Found {
                text: card.text.clone(),
                confidence: card.confidence,
                good,
                platinum,
                ingame,
                pair,
                looked,
                per_day_tenths,
                kit,
                whole: of,
                favourite,
                reached,
            }
        })
        .collect()
}

/// The grid the screen shows, the item key each of its cards matches, and the catalog data of
/// the matched keys.
async fn named(
    catalog: &Catalog,
    lang: &str,
    ours: Option<&str>,
    rows: &[Vec<Card>],
) -> (Vec<Card>, Vec<Option<String>>, Vec<Good>) {
    let asking = rows.to_vec();
    let own = ours.map(str::to_owned);
    let tongue = lang.to_owned();

    let answered = catalog
        .read(move |connection| {
            let pool = Pool::new(catalog::reward_names(connection, &tongue)?);
            let (cards, keys) = standing(&pool, &asking, own.as_deref());

            let known: Vec<String> = keys.iter().flatten().cloned().collect();
            let goods = catalog::goods(connection, &tongue, &known)?;
            Ok((cards, keys, goods))
        })
        .await;

    match answered {
        Ok(answered) => answered,
        Err(why) => {
            warn!(error = %why, "reward items lookup failed");
            let row: Vec<Card> = rows
                .first()
                .map(|row| row.iter().filter(|card| carries(card)).cloned().collect())
                .unwrap_or_default();
            let keys = vec![None; row.len()];
            (row, keys, Vec::new())
        }
    }
}

/// The grid that holds the cards — the one with the most dictionary matches, then the one that
/// contains this account's reward, then the fuller one — and the key each of its cards matches.
/// Positions without a card are omitted.
pub(super) fn standing(
    pool: &Pool,
    rows: &[Vec<Card>],
    ours: Option<&str>,
) -> (Vec<Card>, Vec<Option<String>>) {
    let read: Vec<Vec<Option<String>>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|card| pool.nearest(uncounted(&card.text)).map(str::to_owned))
                .collect()
        })
        .collect();

    let Some((at, keys)) = read.iter().enumerate().max_by_key(|(at, keys)| {
        let known = keys.iter().flatten().count();
        let mine = ours.is_some_and(|ours| keys.iter().flatten().any(|key| key == ours));
        (known, mine, std::cmp::Reverse(*at))
    }) else {
        return (Vec::new(), Vec::new());
    };

    if !keys.iter().any(Option::is_some) {
        for row in rows {
            let texts: Vec<&str> = row.iter().map(|card| card.text.as_str()).collect();
            warn!(
                cards = row.len(),
                texts = texts.join(" | "),
                "reward cards match no catalog item"
            );
        }
    }

    rows[at]
        .iter()
        .cloned()
        .zip(keys.iter().cloned())
        .filter(|(card, _)| carries(card))
        .unzip()
}

/// Name after the count printed on a card of several copies: `1,200 X Кува` gives `Кува`.
fn uncounted(text: &str) -> &str {
    let rest =
        text.trim_start_matches(|c: char| c.is_ascii_digit() || matches!(c, ',' | '.' | ' '));
    if rest.len() == text.len() {
        return text;
    }
    match rest.strip_prefix(['X', 'x', 'Х', 'х']) {
        Some(name) if name.starts_with(' ') => name.trim_start(),
        _ => text,
    }
}

/// Minimum number of letters recognised at a position for it to count as a card; fewer is the
/// noise of an empty position, and no reward name is that short.
const LEAST_LETTERS: usize = 3;

/// Whether a position holds a card: an empty position reads as nothing, or as one or two marks.
fn carries(card: &Card) -> bool {
    canon(&card.text).chars().count() >= LEAST_LETTERS
}

/// Progress of the account with the item each card's set builds, keyed by card item.
async fn reached(
    catalog: &Catalog,
    holdings: &SharedHoldings,
    built: &HashMap<String, String>,
) -> HashMap<String, Reached> {
    let mut wholes: Vec<String> = built.values().cloned().collect();
    wholes.sort();
    wholes.dedup();
    let reached = match crate::mastery::reached(catalog, holdings, &wholes).await {
        Ok(reached) => reached,
        Err(why) => {
            warn!(error = %why, "set progress read failed");
            return HashMap::new();
        }
    };
    built
        .iter()
        .filter_map(|(card, whole)| Some((card.clone(), *reached.get(whole)?)))
        .collect()
}

/// For each item: the parts of its set with the count the account holds, the set's price and
/// the item the set builds. Items in no set are absent from the result.
#[allow(clippy::type_complexity)]
async fn sets(
    catalog: &Catalog,
    market_cache: &Store,
    holdings: &SharedHoldings,
    lang: &str,
    items: &[String],
) -> (
    HashMap<String, Vec<Owned>>,
    HashMap<String, Option<u32>>,
    HashMap<String, String>,
) {
    if items.is_empty() {
        return (HashMap::new(), HashMap::new(), HashMap::new());
    }

    let asking = items.to_vec();
    let tongue = lang.to_owned();
    let (parts, rows) = match catalog
        .read(move |connection| {
            Ok((
                catalog::kit(connection, &tongue, &asking)?,
                catalog::sets(connection, &tongue)?,
            ))
        })
        .await
    {
        Ok(read) => read,
        Err(why) => {
            warn!(error = %why, "reward sets read failed");
            return (HashMap::new(), HashMap::new(), HashMap::new());
        }
    };
    let builds: HashMap<String, String> = rows
        .into_iter()
        .filter_map(|row| Some((row.slug, row.built?)))
        .collect();

    let mut slugs: Vec<String> = parts.iter().map(|part| part.slug.clone()).collect();
    slugs.sort();
    slugs.dedup();
    let priced = match market::quoted_sets(market_cache, &slugs).await {
        Ok(priced) => priced,
        Err(why) => {
            warn!(error = %why, "reward set prices read failed");
            HashMap::new()
        }
    };

    let held = holdings.counted();
    let mut kits: HashMap<String, Vec<Owned>> = HashMap::new();
    let mut whole: HashMap<String, Option<u32>> = HashMap::new();
    let mut built: HashMap<String, String> = HashMap::new();
    for part in parts {
        whole.insert(part.of.clone(), priced.get(&part.slug).copied());
        if let Some(item) = builds.get(&part.slug) {
            built.insert(part.of.clone(), item.clone());
        }
        kits.entry(part.of.clone()).or_default().push(Owned {
            image: part.image,
            count: held.get(&part.item).copied().unwrap_or(0),
            need: part.per_set,
            this: part.item == part.of,
        });
    }
    (kits, whole, built)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Catalog names of the rewards used below, copied from the catalog.
    const NAMES: [(&str, &str); 8] = [
        ("trumna", "Трумна Прайм (Чертеж)"),
        ("banshee-helmet", "Банши Прайм: Нейрооптика (Чертеж)"),
        ("nautilus-carapace", "Наутилус Прайм: Панцирь"),
        ("kestrel-blade", "Кестрел Прайм: Клинок"),
        ("braton-barrel", "Брэйтон Прайм: Ствол"),
        ("tatsu-blade", "Татсу Прайм: Лезвие"),
        ("fang-handle", "Клыки Прайм: Рукоять"),
        ("forma", "Форма (Чертеж)"),
    ];

    fn pool() -> Pool {
        Pool::new(
            NAMES
                .iter()
                .map(|(item, name)| ((*item).to_owned(), (*name).to_owned()))
                .collect(),
        )
    }

    fn row(texts: &[&str]) -> Vec<Card> {
        texts
            .iter()
            .map(|text| Card {
                text: (*text).to_owned(),
                confidence: 0.9,
            })
            .collect()
    }

    /// Recognised text of a real three-card screen, for both position grids.
    fn three() -> Vec<Vec<Card>> {
        vec![
            row(&[
                "Чертёж: Тру",
                "-Чертёж: Ба мна ПраймНейро",
                "ши Прайм:Наутилу птикаПан",
                "Прайм: ирь",
            ]),
            row(&[
                "Чертёж: Трумна Прайм",
                "Чертёж: Банши Прайм: Нейрооптика",
                "Наутилус-Прайм: Панцирь",
            ]),
        ]
    }

    /// The same for a real two-card screen.
    fn two() -> Vec<Vec<Card>> {
        vec![
            row(&[
                "",
                "Кк Кестрел Прайм: Клинок",
                "Брэйтон Прайм: Ствол",
                "___",
            ]),
            row(&["к KестрелПра", "k= йм: Клинок Брэйтон Пр", "йм: Ствол"]),
        ]
    }

    #[test]
    fn a_count_before_the_name_is_passed_over() {
        assert_eq!(uncounted("1,200 X Кува"), "Кува");
        assert_eq!(uncounted("2 x Forma"), "Forma");
        assert_eq!(uncounted("Форма (Чертеж)"), "Форма (Чертеж)");
        assert_eq!(uncounted("1,200"), "1,200");
    }

    #[test]
    fn three_cards_stand_on_the_shorter_row() {
        let (cards, keys) = standing(&pool(), &three(), None);

        assert_eq!(cards.len(), 3);
        assert_eq!(
            keys,
            vec![
                Some("trumna".to_owned()),
                Some("banshee-helmet".to_owned()),
                Some("nautilus-carapace".to_owned())
            ]
        );
    }

    #[test]
    fn two_cards_stand_on_the_longer_row_with_its_ends_empty() {
        let (cards, keys) = standing(&pool(), &two(), None);

        assert_eq!(
            cards
                .iter()
                .map(|card| card.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Кк Кестрел Прайм: Клинок", "Брэйтон Прайм: Ствол"]
        );
        assert_eq!(
            keys,
            vec![
                Some("kestrel-blade".to_owned()),
                Some("braton-barrel".to_owned())
            ]
        );
    }

    /// The same two-card screen, with its empty first position read as a single letter.
    #[test]
    fn a_lone_letter_off_an_empty_place_is_no_card() {
        let mut rows = two();
        rows[0][0].text = "А".to_owned();

        let (cards, keys) = standing(&pool(), &rows, None);

        assert_eq!(cards.len(), 2);
        assert_eq!(
            keys,
            vec![
                Some("kestrel-blade".to_owned()),
                Some("braton-barrel".to_owned())
            ]
        );
    }

    /// Every position of the wrong grid read as non-empty text, and none of it matched an item.
    #[test]
    fn a_row_read_off_the_wrong_places_names_nothing() {
        let (_, wrong) = standing(&pool(), &[three()[0].clone()], None);

        assert!(wrong.iter().all(Option::is_none));
    }

    #[test]
    fn our_own_reward_breaks_a_tie() {
        let rows = vec![
            row(&["Чертёж: Трумна Прайм"]),
            row(&["Клыки Прайм: Рукоять"]),
        ];

        let (_, keys) = standing(&pool(), &rows, Some("fang-handle"));

        assert_eq!(keys, vec![Some("fang-handle".to_owned())]);
    }
}
