use crate::Instant;
use crate::riven::Riven;

/// One line of a trade: an item, or the platinum or credits paid for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwapPart {
    /// Name as the game writes it, in the game's language.
    pub name: String,
    pub count: u32,
    pub kind: Kind,
    /// Rank the game shows in brackets after the item name; `None` when it shows none.
    pub rank: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Item,
    Platinum,
    Credits,
}

impl Kind {
    pub fn as_code(self) -> &'static str {
        match self {
            Kind::Item => "item",
            Kind::Platinum => "platinum",
            Kind::Credits => "credits",
        }
    }
}

/// Result of a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Failed,
    Cancelled,
}

impl Outcome {
    pub fn as_code(self) -> &'static str {
        match self {
            Outcome::Done => "done",
            Outcome::Failed => "failed",
            Outcome::Cancelled => "cancelled",
        }
    }
}

/// One trade as the game log reports it: the partner, what the account gave and what it received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swap {
    pub at: Instant,
    pub partner: String,
    pub given: Vec<SwapPart>,
    pub taken: Vec<SwapPart>,
    pub outcome: Outcome,
}

impl Swap {
    fn platinum(parts: &[SwapPart]) -> u32 {
        parts
            .iter()
            .filter(|part| part.kind == Kind::Platinum)
            .map(|part| part.count)
            .sum()
    }

    /// Whether the account paid platinum and received an item.
    pub fn buying(&self) -> bool {
        Self::platinum(&self.given) > 0 && self.taken.iter().any(|part| part.kind == Kind::Item)
    }

    /// Whether the account gave an item and received platinum.
    pub fn selling(&self) -> bool {
        Self::platinum(&self.taken) > 0 && self.given.iter().any(|part| part.kind == Kind::Item)
    }

    /// Platinum transferred in the trade, in either direction.
    pub fn platinum_moved(&self) -> u32 {
        Self::platinum(&self.given).max(Self::platinum(&self.taken))
    }

    /// Items transferred: those received when buying, those given otherwise.
    pub fn items(&self) -> &[SwapPart] {
        match self.buying() {
            true => &self.taken,
            false => &self.given,
        }
    }
}

/// Splits the platinum of one trade across its item lines in proportion to their copy counts (at
/// least 1 each); the last line takes the remainder, so the parts always add up to `platinum`.
/// Example: 58 over four single copies gives 14, 14, 14, 16.
pub fn split(platinum: u32, counts: &[u32]) -> Vec<u32> {
    let whole: u32 = counts.iter().copied().map(|count| count.max(1)).sum();
    if whole == 0 {
        return counts.iter().map(|_| 0).collect();
    }

    let mut parts = Vec::with_capacity(counts.len());
    let mut given = 0;
    for (at, count) in counts.iter().enumerate() {
        let part = match at + 1 == counts.len() {
            true => platinum.saturating_sub(given),
            false => (u64::from(platinum) * u64::from((*count).max(1)) / u64::from(whole)) as u32,
        };
        given += part;
        parts.push(part);
    }
    parts
}

/// In-game whisper received by the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Whisper {
    pub player: String,
    pub at: Instant,
}

/// One part of a reward's set, and how many copies of it the account holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owned {
    pub image: Option<String>,
    pub count: u32,
    /// Copies the set needs.
    pub need: u32,
    /// Whether this is the part shown on the reward screen.
    pub this: bool,
}

/// Progress of the account with the item a reward's set builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reached {
    /// Owned or built in the foundry, and not yet at its maximum rank.
    Built,
    /// Ranked up to its maximum rank.
    Mastered,
}

/// One reward card on the relic reward screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offered {
    /// Catalog key; empty when the card was not recognised.
    pub item: String,
    /// Display name: the catalog name, or the text read from the card when no item matched.
    pub name: String,
    pub image: Option<String>,
    /// Median's own price estimate; `None` when the market cache has too few closed trades.
    pub platinum: Option<u32>,
    /// Lowest ask of a seller who can be traded with now, from the latest order book in the
    /// market cache.
    pub ingame: Option<u32>,
    /// Copies traded per day, in tenths; `None` when the market cache has no data.
    pub per_day_tenths: Option<u32>,
    pub ducats: Option<u32>,
    /// Whether this card is the reward from the account's own relic, which the game reports by
    /// name.
    pub own: bool,
    /// Whether the reward, or the item its set builds, is marked as a favourite.
    pub favourite: bool,
    /// Parts of the reward's set, in catalog order; empty when the reward belongs to no set.
    pub kit: Vec<Owned>,
    /// Price of the whole set; `None` when the reward belongs to no set or the market cache has
    /// no price.
    pub whole: Option<u32>,
    /// Account progress with the item the set builds; `None` when the item is neither built nor
    /// mastered, or the reward belongs to no set.
    pub reached: Option<Reached>,
}

/// Maximum number of whispers kept for the window.
const KEPT: usize = 32;

/// Maximum number of trades kept for the window.
const SWAPS: usize = 16;

/// State read from the game log.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Game {
    /// Trades seen in the game log since the last inventory dump reported the daily trades left.
    /// The game reports that count again only when the player leaves the dojo, and by then each
    /// of these trades has used up one daily trade.
    pub spent: u32,
    /// Platinum received minus platinum paid in completed trades since the last inventory dump.
    pub platinum_moved: i64,
    /// Whether the game log (the game's debug output) is being read.
    pub listening: bool,
    /// Whether another application is also attached to the game's debug output.
    pub shared: bool,
    /// Whispers received, newest first, one per player; at most `KEPT`.
    pub whispers: Vec<Whisper>,
    /// Trades the game log reported, newest first; at most `SWAPS`.
    pub swaps: Vec<Swap>,
    /// Relic reward screen state, as the overlay shows it.
    pub rewards: Rewards,
    /// Riven cycling screen state, as the overlay shows it.
    pub cycling: Cycling,
    /// Star chart node the game last loaded; `None` on the ship, at a camp, or before the first
    /// level load.
    pub node: Option<String>,
    /// When the game window was first seen in this run of the game; `None` while the game is not
    /// running or before its window was seen.
    pub running_since: Option<Instant>,
    /// Whether reward cards can be read: the recognition model is loaded, missing or broken.
    pub recognition: Recognition,
}

/// State of the model that reads reward card names off the screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Recognition {
    /// The game log task has not loaded it yet.
    #[default]
    Unchecked,
    /// Loaded; reward cards are read.
    Ready,
    /// The model embedded in the build did not load; reward cards are not read.
    Failed,
}

/// State of the relic reward screen, from when it opens until it closes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Rewards {
    #[default]
    Gone,
    /// The screen is open and its cards are still being read.
    Reading,
    /// The cards read; `fresh` once their market data has been fetched again.
    Read { cards: Vec<Offered>, fresh: bool },
}

/// State of the riven cycling screen, from when it opens until it closes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Cycling {
    #[default]
    Gone,
    /// The screen is open: the cards read from it and the rerolls made since it opened.
    Open {
        cards: Vec<CycleCard>,
        rolls: Vec<CycleRoll>,
        /// Whether new cards are being read.
        reading: bool,
        /// Why the last cards could not be read as rivens; `None` when they could.
        unread: Option<Unread>,
    },
}

/// Reason the cards on the cycling screen could not be read as rivens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unread {
    /// The model that reads the screen did not load.
    NoReader,
    /// No riven in the inventory has the name on the card.
    NotHeld,
    /// Rivens of the card's weapon are not supported.
    Unsupported,
    /// The stats on the card could not be read.
    Stats,
}

/// Position of a card on the cycling screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleSide {
    /// The chosen card, in the middle.
    Chosen,
    /// The old riven, shown next to the new one while the player chooses.
    Left,
    /// The new riven, shown next to the old one while the player chooses.
    Right,
}

/// One card of the cycling screen, read as a riven.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleCard {
    pub side: CycleSide,
    pub riven: Riven,
    pub rank: u8,
    pub name: String,
    /// Whether it is the riven the last reroll produced.
    pub fresh: bool,
}

/// One reroll made on the cycling screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleRoll {
    pub kuva: u32,
    /// Name of the riven it produced; `None` until it is read.
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    /// Reading of the game log started or stopped; `shared` is whether another application is
    /// also attached to the game's debug output.
    Listening {
        listening: bool,
        shared: bool,
    },
    Whispered(Whisper),
    Traded(Box<Swap>),
    /// The relic reward screen opened, was read, was repriced, or closed.
    Rewards(Rewards),
    /// The game loaded a level: a star chart node, or `None` for a location that is not a node.
    Entered(Option<String>),
    /// The riven cycling screen opened, was read, had a reroll, or closed.
    Cycling(Cycling),
    /// The game window was first seen at `at`.
    Started {
        at: Instant,
    },
    /// The game window is gone.
    Stopped,
    /// The recognition model was loaded, or found missing or broken.
    Recognition(Recognition),
}

/// Daily trades left: the count from the last inventory dump minus the trades seen in the game
/// log since then, never below 0; `None` when no dump has reported a count.
pub fn exchanges_left(stated: Option<u32>, game: &Game) -> Option<u32> {
    stated.map(|left| left.saturating_sub(game.spent))
}

/// Platinum held: the amount from the last inventory dump, adjusted by the trades seen in the
/// game log since then; `None` when no dump has reported an amount.
pub fn platinum_left(stated: Option<u32>, game: &Game) -> Option<u32> {
    stated.map(|held| (i64::from(held) + game.platinum_moved).clamp(0, i64::from(u32::MAX)) as u32)
}

/// Game state after `event`.
pub fn update(game: &Game, event: GameEvent) -> Game {
    let mut next = game.clone();
    match event {
        GameEvent::Listening { listening, shared } => {
            next.listening = listening;
            next.shared = shared;
        }
        GameEvent::Whispered(whisper) => {
            next.whispers.retain(|held| held.player != whisper.player);
            next.whispers.insert(0, whisper);
            next.whispers.truncate(KEPT);
        }
        GameEvent::Traded(swap) => {
            next.spent += 1;
            if swap.outcome == Outcome::Done {
                next.platinum_moved +=
                    i64::from(Swap::platinum(&swap.taken)) - i64::from(Swap::platinum(&swap.given));
            }
            next.swaps.insert(0, *swap);
            next.swaps.truncate(SWAPS);
        }
        GameEvent::Rewards(rewards) => next.rewards = rewards,
        GameEvent::Entered(node) => next.node = node,
        GameEvent::Cycling(cycling) => next.cycling = cycling,
        GameEvent::Started { at } => next.running_since = Some(at),
        GameEvent::Stopped => next.running_since = None,
        GameEvent::Recognition(state) => next.recognition = state,
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, count: u32, kind: Kind) -> SwapPart {
        SwapPart {
            name: name.into(),
            count,
            kind,
            rank: None,
        }
    }

    fn swap(given: Vec<SwapPart>, taken: Vec<SwapPart>) -> Swap {
        Swap {
            at: 100,
            partner: "kafasi".into(),
            given,
            taken,
            outcome: Outcome::Done,
        }
    }

    #[test]
    fn the_game_runs_from_its_window_appearing_until_it_is_gone() {
        let started = update(&Game::default(), GameEvent::Started { at: 1_000 });
        assert_eq!(started.running_since, Some(1_000));

        let stopped = update(&started, GameEvent::Stopped);
        assert_eq!(stopped.running_since, None);
    }

    #[test]
    fn the_recognition_state_is_what_the_game_log_task_reported() {
        assert_eq!(Game::default().recognition, Recognition::Unchecked);

        let failed = update(
            &Game::default(),
            GameEvent::Recognition(Recognition::Failed),
        );

        assert_eq!(failed.recognition, Recognition::Failed);
    }

    #[test]
    fn platinum_out_and_an_item_in_is_a_purchase() {
        let bought = swap(
            vec![part("Платина", 108, Kind::Platinum)],
            vec![part("Мистическая зарядка", 1, Kind::Item)],
        );

        assert!(bought.buying());
        assert!(!bought.selling());
        assert_eq!(bought.platinum_moved(), 108);
        assert_eq!(bought.items()[0].name, "Мистическая зарядка");
    }

    #[test]
    fn an_item_out_and_platinum_in_is_a_sale() {
        let sold = swap(
            vec![part("Мистическая зарядка", 1, Kind::Item)],
            vec![part("Платина", 140, Kind::Platinum)],
        );

        assert!(sold.selling());
        assert_eq!(sold.items()[0].count, 1);
    }

    #[test]
    fn an_exchange_of_items_alone_is_neither() {
        let barter = swap(
            vec![part("Форма Чертёж", 1, Kind::Item)],
            vec![part("Мезо F3 Реликвия", 2, Kind::Item)],
        );

        assert!(!barter.buying());
        assert!(!barter.selling());
        assert_eq!(barter.platinum_moved(), 0);
    }

    #[test]
    fn the_platinum_of_an_exchange_falls_on_the_lines_by_their_copies() {
        assert_eq!(split(58, &[1, 1, 1, 1]), vec![14, 14, 14, 16]);
        assert_eq!(split(100, &[1]), vec![100]);
        assert_eq!(split(60, &[3, 1]), vec![45, 15]);
    }

    #[test]
    fn the_parts_of_a_split_always_add_up_to_the_whole() {
        for platinum in [0, 1, 7, 58, 999] {
            for counts in [vec![1], vec![1, 1, 1], vec![2, 5], vec![1, 1, 1, 1, 1]] {
                let parts = split(platinum, &counts);
                assert_eq!(
                    parts.iter().sum::<u32>(),
                    platinum,
                    "{platinum} over {counts:?}"
                );
            }
        }
    }

    #[test]
    fn an_exchange_counts_against_the_day_before_the_game_says_so() {
        let game = update(
            &Game::default(),
            GameEvent::Traded(Box::new(swap(
                vec![part("Платина", 10, Kind::Platinum)],
                vec![part("Форма Чертёж", 1, Kind::Item)],
            ))),
        );

        assert_eq!(game.spent, 1);
        assert_eq!(exchanges_left(Some(3), &game), Some(2));
        assert_eq!(exchanges_left(Some(0), &game), Some(0), "never below none");
        assert_eq!(exchanges_left(None, &game), None, "no dump has said");
    }

    #[test]
    fn the_newest_whisper_from_a_player_replaces_the_older_one() {
        let game = update(
            &Game::default(),
            GameEvent::Whispered(Whisper {
                player: "kafasi".into(),
                at: 10,
            }),
        );
        let game = update(
            &game,
            GameEvent::Whispered(Whisper {
                player: "kafasi".into(),
                at: 20,
            }),
        );

        assert_eq!(game.whispers.len(), 1);
        assert_eq!(game.whispers[0].at, 20);
    }

    #[test]
    fn what_falls_off_the_end_stops_being_kept() {
        let mut game = Game::default();
        for at in 0..(KEPT as Instant + 4) {
            game = update(
                &game,
                GameEvent::Whispered(Whisper {
                    player: format!("player{at}"),
                    at,
                }),
            );
        }

        assert_eq!(game.whispers.len(), KEPT);
        assert_eq!(game.whispers[0].player, format!("player{}", KEPT + 3));
    }

    #[test]
    fn the_node_last_loaded_stands_until_the_next_level() {
        let there = update(
            &Game::default(),
            GameEvent::Entered(Some("SolNode236".into())),
        );
        assert_eq!(there.node.as_deref(), Some("SolNode236"));

        let back = update(&there, GameEvent::Entered(None));
        assert_eq!(back.node, None);
    }

    #[test]
    fn the_purse_moves_with_the_exchanges_caught_since_the_dump() {
        let bought = swap(
            vec![part("Platinum", 40, Kind::Platinum)],
            vec![part("Arcane Energize", 1, Kind::Item)],
        );
        let sold = swap(
            vec![part("Arcane Energize", 1, Kind::Item)],
            vec![part("Platinum", 55, Kind::Platinum)],
        );
        let game = update(&Game::default(), GameEvent::Traded(Box::new(bought)));
        let game = update(&game, GameEvent::Traded(Box::new(sold)));

        assert_eq!(platinum_left(Some(100), &game), Some(115));
        assert_eq!(platinum_left(None, &game), None);
    }

    #[test]
    fn an_exchange_that_did_not_go_through_moves_no_platinum() {
        let mut failed = swap(
            vec![part("Platinum", 40, Kind::Platinum)],
            vec![part("Arcane Energize", 1, Kind::Item)],
        );
        failed.outcome = Outcome::Cancelled;

        let game = update(&Game::default(), GameEvent::Traded(Box::new(failed)));

        assert_eq!(platinum_left(Some(100), &game), Some(100));
    }
}
