use crate::time::Instant;

/// Highest mastery rank before the Legendary ranks.
const PLAIN: u32 = 30;

/// Rank an item reaches on affinity alone; for an item with a higher cap, each Forma on it adds
/// `PER_FORMA` ranks.
const UNFORMED: u32 = 30;

/// Ranks each Forma adds above `UNFORMED`.
const PER_FORMA: u32 = 2;

/// Mastery points per intrinsic rank.
const INTRINSIC: u64 = 1_500;

/// Maximum rank of an intrinsic school.
const SCHOOL: u32 = 10;

/// Railjack intrinsic schools, as keys of the account's skills.
const RAILJACK: [&str; 5] = [
    "LPS_PILOTING",
    "LPS_GUNNERY",
    "LPS_TACTICAL",
    "LPS_ENGINEERING",
    "LPS_COMMAND",
];

/// Drifter intrinsic schools.
const DRIFTER: [&str; 4] = [
    "LPS_DRIFT_RIDING",
    "LPS_DRIFT_COMBAT",
    "LPS_DRIFT_OPPORTUNITY",
    "LPS_DRIFT_ENDURANCE",
];

/// Mission tier of a node completed on the Steel Path.
const STEEL: u32 = 1;

/// Mastery class of an item, which sets its mastery points and affinity per rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mastery {
    /// Weapons of every kind.
    Wielded,
    /// Warframes, companions, archwings, necramechs, K-drives and the Plexus.
    Carried,
}

impl Mastery {
    /// Parses the catalog's class name (`wielded`, `carried`); `None` for anything else.
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "wielded" => Some(Self::Wielded),
            "carried" => Some(Self::Carried),
            _ => None,
        }
    }

    /// Mastery points per rank: 100 for weapons, 200 for the rest.
    pub fn per_rank(self) -> u64 {
        match self {
            Self::Wielded => 100,
            Self::Carried => 200,
        }
    }

    /// Affinity the item needs to reach `rank`.
    pub fn affinity(self, rank: u32) -> u64 {
        let rank = u64::from(rank);
        self.unit() * rank * rank
    }

    /// Rank the item has reached with `xp` affinity, capped at `cap`.
    pub fn rank(self, cap: u32, xp: u64) -> u32 {
        let reached = (xp / self.unit()).isqrt();
        u32::try_from(reached).unwrap_or(u32::MAX).min(cap)
    }

    /// Mastery points the item gives at `rank`.
    pub fn points(self, rank: u32) -> u64 {
        self.per_rank() * u64::from(rank)
    }

    fn unit(self) -> u64 {
        match self {
            Self::Wielded => 500,
            Self::Carried => 1_000,
        }
    }
}

/// One item that gives mastery, with the account's affinity on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Levelled {
    pub mastery: Mastery,
    pub cap: u32,
    /// Affinity; 0 for an item the account has never levelled.
    pub xp: u64,
    /// Forma count on the held copy with the most Forma; `None` when no copy is held.
    pub copy: Option<u32>,
}

impl Levelled {
    pub fn rank(&self) -> u32 {
        self.mastery.rank(self.cap, self.xp)
    }

    pub fn earned(&self) -> u64 {
        self.mastery.points(self.rank())
    }

    /// Mastery points the item can still give.
    pub fn remaining(&self) -> u64 {
        self.mastery.points(self.cap) - self.earned()
    }

    pub fn is_mastered(&self) -> bool {
        self.rank() == self.cap
    }

    /// Forma still needed to reach the cap; a copy not yet held counts as having none.
    pub fn forma(&self) -> u32 {
        match self.is_mastered() {
            true => 0,
            false => self.steps() - self.formed(),
        }
    }

    /// Mastery points the item can still give on affinity alone, before the next Forma.
    pub fn reachable(&self) -> u64 {
        self.mastery
            .points(self.open())
            .saturating_sub(self.earned())
    }

    /// Number of levelling runs left: one up to the rank the copy's Forma allow, then one after
    /// each further Forma, which resets the copy to rank 0.
    pub fn passes(&self) -> u32 {
        if self.is_mastered() {
            return 0;
        }
        let unfinished = self.copy.is_none() || self.rank() < self.open();
        self.forma() + u32::from(unfinished)
    }

    /// Forma needed to reach the cap from `UNFORMED`.
    fn steps(&self) -> u32 {
        self.cap.saturating_sub(UNFORMED).div_ceil(PER_FORMA)
    }

    /// Forma on the held copy, at most `steps`.
    fn formed(&self) -> u32 {
        self.copy.unwrap_or(0).min(self.steps())
    }

    /// Highest rank the levelled copy can reach with its current Forma.
    fn open(&self) -> u32 {
        self.cap.min(UNFORMED + PER_FORMA * self.formed())
    }
}

/// Star chart node and the mastery points its first completion gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub tag: String,
    pub points: u64,
}

impl Node {
    /// Whether the node is a junction between two planets.
    pub fn is_junction(&self) -> bool {
        self.tag.ends_with("Junction")
    }
}

/// A node the account has completed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mission {
    pub tag: String,
    pub completes: u32,
    pub tier: Option<u32>,
}

impl Mission {
    pub fn is_steel_path(&self) -> bool {
        self.tier == Some(STEEL)
    }
}

/// Account rank in one intrinsic school.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intrinsic {
    pub school: String,
    pub rank: u32,
}

/// One mastery source: points earned, and progress as `done` of `of`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Share {
    pub earned: u64,
    pub done: u32,
    pub of: u32,
}

/// Mastery source that could not be counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Uncounted {
    Items,
    Nodes,
}

/// Mastery points the account has earned, by source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Breakdown {
    pub items: Share,
    pub nodes: Share,
    pub steel_path: Share,
    pub junctions: Share,
    pub steel_junctions: Share,
    pub railjack: Share,
    pub drifter: Share,
    /// Sources missing from the total; when not empty, the total is a lower bound.
    pub uncounted: Vec<Uncounted>,
}

impl Breakdown {
    pub fn total(&self) -> u64 {
        [
            self.items,
            self.nodes,
            self.steel_path,
            self.junctions,
            self.steel_junctions,
            self.railjack,
            self.drifter,
        ]
        .iter()
        .map(|share| share.earned)
        .sum()
    }

    pub fn is_exact(&self) -> bool {
        self.uncounted.is_empty()
    }
}

/// Mastery points earned from every source; nodes worth 0 points are skipped.
pub fn breakdown(
    items: &[Levelled],
    nodes: &[Node],
    missions: &[Mission],
    intrinsics: &[Intrinsic],
) -> Breakdown {
    let (junctions, plain): (Vec<&Node>, Vec<&Node>) = nodes
        .iter()
        .filter(|node| node.points > 0)
        .partition(|node| node.is_junction());

    Breakdown {
        items: Share {
            earned: items.iter().map(Levelled::earned).sum(),
            done: count(items.iter().filter(|item| item.is_mastered())),
            of: count(items.iter()),
        },
        nodes: completed(&plain, missions, |_| true),
        steel_path: completed(&plain, missions, Mission::is_steel_path),
        junctions: completed(&junctions, missions, |_| true),
        steel_junctions: completed(&junctions, missions, Mission::is_steel_path),
        railjack: school(&RAILJACK, intrinsics),
        drifter: school(&DRIFTER, intrinsics),
        uncounted: Vec::new(),
    }
}

/// Mastery points of the nodes completed in a way `counted` accepts.
fn completed(nodes: &[&Node], missions: &[Mission], counted: fn(&Mission) -> bool) -> Share {
    let done: Vec<&&Node> = nodes
        .iter()
        .filter(|node| {
            missions
                .iter()
                .any(|mission| mission.tag == node.tag && mission.completes > 0 && counted(mission))
        })
        .collect();
    Share {
        earned: done.iter().map(|node| node.points).sum(),
        done: count(done.iter()),
        of: count(nodes.iter()),
    }
}

/// Mastery points from the ranks held in these intrinsic schools, each capped at `SCHOOL`.
fn school(schools: &[&str], intrinsics: &[Intrinsic]) -> Share {
    let ranks: u32 = intrinsics
        .iter()
        .filter(|intrinsic| schools.contains(&intrinsic.school.as_str()))
        .map(|intrinsic| intrinsic.rank.min(SCHOOL))
        .sum();
    Share {
        earned: INTRINSIC * u64::from(ranks),
        done: ranks,
        of: SCHOOL * count(schools.iter()),
    }
}

fn count<T>(of: impl Iterator<Item = T>) -> u32 {
    u32::try_from(of.count()).unwrap_or(u32::MAX)
}

/// Mastery points required for rank `rank`: 2,500 × rank² up to rank 30, then 147,500 more per
/// Legendary rank.
pub fn threshold(rank: u32) -> u64 {
    match rank.checked_sub(PLAIN) {
        None | Some(0) => 2_500 * u64::from(rank) * u64::from(rank),
        Some(legendary) => 2_250_000 + 147_500 * u64::from(legendary),
    }
}

/// Highest rank that `total` mastery points are enough for.
pub fn reached(total: u64) -> u32 {
    let mut rank = 0;
    while threshold(rank + 1) <= total {
        rank += 1;
    }
    rank
}

/// Whether the mastery test for the next rank can be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankTest {
    /// The account has not earned enough for the next rank.
    Unearned,
    Open,
    /// Earned, but after the last attempt the test stays locked until this instant.
    Shut(Instant),
}

/// Account mastery total compared with its current and next rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outlook {
    pub rank: u32,
    pub total: u64,
    /// Whether every source was counted; otherwise `total` is a lower bound.
    pub exact: bool,
    /// Mastery points the next rank requires.
    pub next: u64,
    pub remaining: u64,
    pub test: RankTest,
    /// Every source was counted, yet the total is below the threshold of the account's rank.
    pub undercounted: bool,
}

/// Compares the account's mastery total with its current rank and the next one.
pub fn outlook(
    rank: u32,
    breakdown: &Breakdown,
    shut_until: Option<Instant>,
    now: Instant,
) -> Outlook {
    let total = breakdown.total();
    let exact = breakdown.is_exact();
    let next = threshold(rank + 1);
    let test = match (total >= next, shut_until) {
        (false, _) => RankTest::Unearned,
        (true, Some(until)) if until > now => RankTest::Shut(until),
        (true, _) => RankTest::Open,
    };
    Outlook {
        rank,
        total,
        exact,
        next,
        remaining: next.saturating_sub(total),
        test,
        undercounted: exact && total < threshold(rank),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A held copy with no Forma set on it.
    fn wielded(cap: u32, xp: u64) -> Levelled {
        Levelled {
            mastery: Mastery::Wielded,
            cap,
            xp,
            copy: Some(0),
        }
    }

    /// A held copy with no Forma set on it.
    fn carried(cap: u32, xp: u64) -> Levelled {
        Levelled {
            mastery: Mastery::Carried,
            cap,
            xp,
            copy: Some(0),
        }
    }

    fn node(tag: &str, points: u64) -> Node {
        Node {
            tag: tag.to_string(),
            points,
        }
    }

    fn mission(tag: &str, tier: Option<u32>) -> Mission {
        Mission {
            tag: tag.to_string(),
            completes: 1,
            tier,
        }
    }

    fn intrinsic(school: &str, rank: u32) -> Intrinsic {
        Intrinsic {
            school: school.to_string(),
            rank,
        }
    }

    #[test]
    fn a_weapon_capped_at_forty_that_stands_on_thirty_is_not_mastered() {
        let kraken = wielded(40, 450_000);

        assert_eq!(kraken.rank(), 30);
        assert!(!kraken.is_mastered());
        assert_eq!(kraken.remaining(), 1_000);
    }

    #[test]
    fn affinity_past_the_cap_counts_the_cap_and_no_more() {
        let nukor = wielded(40, 118_344_961);
        let volt = carried(30, 64_061_238);

        assert_eq!(nukor.earned(), 4_000);
        assert!(nukor.is_mastered());
        assert_eq!(volt.earned(), 6_000);
    }

    #[test]
    fn a_weapon_capped_at_forty_takes_five_forma_past_thirty() {
        let kraken = wielded(40, 450_000);

        assert_eq!(kraken.forma(), 5);
        assert_eq!(kraken.reachable(), 0);
    }

    #[test]
    fn the_forma_set_on_the_held_copy_count_towards_the_cap() {
        let one_set = Levelled {
            copy: Some(1),
            ..wielded(40, Mastery::Wielded.affinity(31))
        };
        let two_set = Levelled {
            copy: Some(2),
            ..wielded(40, Mastery::Wielded.affinity(34))
        };

        assert_eq!(one_set.forma(), 4);
        assert_eq!(one_set.reachable(), 100);
        assert_eq!(two_set.forma(), 3);
        assert_eq!(two_set.reachable(), 0);
    }

    #[test]
    fn a_forma_set_and_not_yet_levelled_is_counted() {
        let kraken = Levelled {
            copy: Some(1),
            ..wielded(40, 450_000)
        };

        assert_eq!(kraken.forma(), 4);
        assert_eq!(kraken.reachable(), 200);
        assert_eq!(kraken.passes(), 5);
    }

    #[test]
    fn a_copy_not_held_starts_bare_whatever_rank_an_old_one_reached() {
        let sold = Levelled {
            copy: None,
            ..wielded(40, Mastery::Wielded.affinity(34))
        };

        assert_eq!(sold.forma(), 5);
        assert_eq!(sold.reachable(), 0);
        assert_eq!(sold.passes(), 6);
    }

    #[test]
    fn forma_past_what_the_cap_takes_change_nothing() {
        let ogris = Levelled {
            copy: Some(7),
            ..wielded(40, Mastery::Wielded.affinity(36))
        };

        assert_eq!(ogris.forma(), 0);
        assert_eq!(ogris.reachable(), 400);
        assert_eq!(ogris.passes(), 1);
    }

    #[test]
    fn a_rank_forty_item_on_thirty_is_levelled_five_more_times() {
        assert_eq!(wielded(40, 450_000).passes(), 5);
        assert_eq!(wielded(30, Mastery::Wielded.affinity(12)).passes(), 1);
        assert_eq!(wielded(40, 118_344_961).passes(), 0);
    }

    #[test]
    fn below_thirty_a_rank_forty_item_levels_to_thirty_and_no_further() {
        let fresh = Levelled {
            copy: None,
            ..carried(40, 0)
        };

        assert_eq!(fresh.forma(), 5);
        assert_eq!(fresh.reachable(), 6_000);
        assert_eq!(fresh.passes(), 6);
    }

    #[test]
    fn an_item_capped_at_thirty_takes_no_forma() {
        let braton = wielded(30, Mastery::Wielded.affinity(12));

        assert_eq!(braton.forma(), 0);
        assert_eq!(braton.reachable(), braton.remaining());
    }

    #[test]
    fn a_mastered_rank_forty_item_takes_no_more_forma() {
        let nukor = wielded(40, 118_344_961);

        assert_eq!(nukor.forma(), 0);
        assert_eq!(nukor.reachable(), 0);
    }

    #[test]
    fn a_necramech_on_nine_hundred_thousand_has_ten_ranks_to_go() {
        let nechrotech = carried(40, 900_000);

        assert_eq!(nechrotech.rank(), 30);
        assert_eq!(nechrotech.remaining(), 2_000);
    }

    #[test]
    fn one_affinity_short_of_a_rank_stays_below_it() {
        assert_eq!(Mastery::Wielded.rank(30, 449_999), 29);
        assert_eq!(Mastery::Wielded.affinity(30), 450_000);
        assert_eq!(Mastery::Carried.affinity(30), 900_000);
    }

    #[test]
    fn the_catalog_names_two_kinds_of_mastery_and_nothing_else() {
        assert_eq!(Mastery::parse("wielded"), Some(Mastery::Wielded));
        assert_eq!(Mastery::parse("carried"), Some(Mastery::Carried));
        assert_eq!(Mastery::parse("worn"), None);
    }

    #[test]
    fn ranks_up_to_thirty_grow_with_the_square_and_legendary_ones_by_a_step() {
        assert_eq!(threshold(18), 810_000);
        assert_eq!(threshold(19), 902_500);
        assert_eq!(threshold(30), 2_250_000);
        assert_eq!(threshold(31), 2_397_500);
        assert_eq!(threshold(36), 3_135_000);
    }

    #[test]
    fn junctions_are_counted_apart_from_the_nodes_they_join() {
        let nodes = [
            node("SolNode27", 24),
            node("SolNode801", 0),
            node("EarthToVenusJunction", 1_000),
            node("VenusToMercuryJunction", 1_000),
        ];
        let missions = [
            mission("SolNode27", Some(1)),
            mission("EarthToVenusJunction", Some(1)),
            mission("VenusToMercuryJunction", None),
        ];

        let counted = breakdown(&[], &nodes, &missions, &[]);

        assert_eq!(
            counted.nodes,
            Share {
                earned: 24,
                done: 1,
                of: 1
            }
        );
        assert_eq!(
            counted.steel_path,
            Share {
                earned: 24,
                done: 1,
                of: 1
            }
        );
        assert_eq!(
            counted.junctions,
            Share {
                earned: 2_000,
                done: 2,
                of: 2
            }
        );
        assert_eq!(
            counted.steel_junctions,
            Share {
                earned: 1_000,
                done: 1,
                of: 2
            }
        );
    }

    #[test]
    fn intrinsics_count_by_school_and_leave_the_points_pools_out() {
        let skills = [
            intrinsic("LPS_PILOTING", 6),
            intrinsic("LPS_TACTICAL", 6),
            intrinsic("LPS_GUNNERY", 7),
            intrinsic("LPS_ENGINEERING", 6),
            intrinsic("LPS_COMMAND", 6),
            intrinsic("LPP_SPACE", 33_845),
            intrinsic("LPS_DRIFT_RIDING", 7),
            intrinsic("LPS_DRIFT_OPPORTUNITY", 9),
            intrinsic("LPS_DRIFT_COMBAT", 7),
            intrinsic("LPS_DRIFT_ENDURANCE", 7),
        ];

        let counted = breakdown(&[], &[], &[], &skills);

        assert_eq!(
            counted.railjack,
            Share {
                earned: 46_500,
                done: 31,
                of: 50
            }
        );
        assert_eq!(
            counted.drifter,
            Share {
                earned: 45_000,
                done: 30,
                of: 40
            }
        );
    }

    #[test]
    fn the_account_measured_on_the_twelfth_stands_seventy_nine_thousand_short() {
        let counted = Breakdown {
            items: Share {
                earned: 691_200,
                done: 181,
                of: 1_000,
            },
            nodes: Share {
                earned: 14_569,
                done: 169,
                of: 169,
            },
            steel_path: Share {
                earned: 4_108,
                done: 66,
                of: 169,
            },
            junctions: Share {
                earned: 13_000,
                done: 13,
                of: 13,
            },
            steel_junctions: Share {
                earned: 9_000,
                done: 9,
                of: 13,
            },
            railjack: Share {
                earned: 46_500,
                done: 31,
                of: 50,
            },
            drifter: Share {
                earned: 45_000,
                done: 30,
                of: 40,
            },
            uncounted: Vec::new(),
        };

        let standing = outlook(18, &counted, None, 0);

        assert_eq!(standing.total, 823_377);
        assert_eq!(standing.remaining, 79_123);
        assert_eq!(standing.test, RankTest::Unearned);
        assert!(!standing.undercounted);
    }

    #[test]
    fn an_earned_test_stays_shut_until_the_last_one_lets_it_open() {
        let counted = Breakdown {
            items: Share {
                earned: 910_000,
                done: 0,
                of: 0,
            },
            ..Breakdown::default()
        };

        assert_eq!(
            outlook(18, &counted, Some(100), 50).test,
            RankTest::Shut(100)
        );
        assert_eq!(outlook(18, &counted, Some(100), 150).test, RankTest::Open);
        assert_eq!(outlook(18, &counted, None, 150).test, RankTest::Open);
    }

    #[test]
    fn a_total_below_the_held_rank_is_only_a_contradiction_when_it_counted_everything() {
        let short = Breakdown {
            items: Share {
                earned: 700_000,
                done: 0,
                of: 0,
            },
            ..Breakdown::default()
        };
        let floor = Breakdown {
            uncounted: vec![Uncounted::Nodes],
            ..short.clone()
        };

        assert!(outlook(18, &short, None, 0).undercounted);
        assert!(!outlook(18, &floor, None, 0).undercounted);
        assert!(!outlook(18, &floor, None, 0).exact);
    }

    #[test]
    fn a_total_reaches_the_highest_rank_its_threshold_allows() {
        assert_eq!(reached(0), 0);
        assert_eq!(reached(902_499), 18);
        assert_eq!(reached(902_500), 19);
        assert_eq!(reached(2_250_000), 30);
        assert_eq!(reached(2_250_000 + 147_500), 31);
    }
}
