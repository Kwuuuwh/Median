/// One whitelist entry: an item and its maximum price per copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capped {
    pub item: String,
    /// Maximum price of one copy, in platinum.
    pub most: u32,
}

/// Limits within which the trading agents act without asking the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// Minimum expected profit per trade, in platinum.
    pub least_profit: u32,
    /// Minimum copies traded per day for a pair to be watched at all.
    pub least_per_day: u32,
    /// Maximum cost of one position, in platinum: the price of a copy times the copies per
    /// trade.
    pub most_per_position: u32,
    /// Maximum platinum the agent may commit to open buy orders.
    pub most_tied: u32,
    /// Whether that limit is taken from the platinum held (`purse_share`) instead of `most_tied`.
    pub tied_from_purse: bool,
    /// Percent of the platinum held that the agent may commit, when `tied_from_purse` is set.
    pub purse_share: u32,
    /// Daily trades left at which the agent stops; the remaining trades are left to the user.
    pub least_exchanges: u32,
    /// Maximum copies offered per trade; warframe.market allows six.
    pub most_per_trade: u32,
    /// Maximum copies one bid asks for; the available platinum may lower it.
    pub most_per_order: u32,
    /// Days a copy is given to sell at the ask before it is sold to the highest bid.
    pub sale_days: u32,
    /// Items the agent never trades.
    pub blacklist: Vec<String>,
    /// When not empty, the only items the agent trades.
    pub whitelist: Vec<Capped>,
    /// Whether the agent also selects riven auction lots.
    pub rivens: bool,
    /// Whether the agent buys mods at rank 0 to resell them ranked up; the user does the fusing.
    pub rank_up: bool,
    /// Maximum platinum in rivens, as a fixed amount.
    pub riven_capital: u32,
    /// Percent of the platinum held for rivens, when `tied_from_purse` is set.
    pub riven_share: u32,
    /// Minimum percent by which a riven lot's buyout must be below its expected resale price.
    pub riven_margin: u32,
    /// Minimum profit of a riven lot over its buyout, in platinum.
    pub riven_least_profit: u32,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            least_profit: 20,
            least_per_day: 20,
            most_per_position: 120,
            // No platinum is committed until the user sets a limit.
            most_tied: 0,
            tied_from_purse: false,
            purse_share: 100,
            least_exchanges: 0,
            most_per_trade: 6,
            most_per_order: 1,
            sale_days: 3,
            blacklist: Vec::new(),
            whitelist: Vec::new(),
            rivens: false,
            rank_up: false,
            riven_capital: 0,
            riven_share: 0,
            riven_margin: 25,
            riven_least_profit: 50,
        }
    }
}

impl Policy {
    /// Minimum copies traded over the 48-hour near window: twice `least_per_day`.
    pub fn least_volume(&self) -> u32 {
        self.least_per_day.saturating_mul(2)
    }

    /// Whether a pair trades often and expensively enough for the agent to trade it.
    pub fn worth_watching(&self, per_day: f64, median: f64) -> bool {
        per_day >= f64::from(self.least_per_day) && median > f64::from(self.least_profit)
    }

    /// Whether a pair trading at `median`, with a usual highest bid of `bid`, leaves at least
    /// `least_profit` between the two.
    pub fn leaves_room(&self, median: f64, bid: u32) -> bool {
        median - f64::from(bid) >= f64::from(self.least_profit)
    }

    /// Minimum profit per copy, in platinum, when one trade carries `carried` copies.
    pub fn least_per_copy(&self, carried: u32) -> u32 {
        self.least_profit.div_ceil(carried.max(1))
    }
}

/// Order slots warframe.market allows one account.
pub const SLOTS: u32 = 100;

/// Lowest price warframe.market accepts.
pub const LEAST: u32 = 1;

/// Order slots kept free so the user can always place an order of their own. This is not a fixed
/// reserve: the agent frees slots as fast as the user fills them, so this only covers the time
/// until the agent's next round notices.
pub const ROOM: u32 = 10;

/// Number of orders the agent may hold at once: `SLOTS` minus the user's own open orders (`hand`)
/// minus `ROOM`. The agent's share is computed from the order slots in use, not from a setting
/// the user must keep in step with their listings; when the user places an order, the agent's
/// lowest-paying bid is removed to free a slot.
pub fn seats(hand: u32) -> u32 {
    SLOTS.saturating_sub(hand).saturating_sub(ROOM)
}

/// One trading opportunity for the agent, with the numbers the policy checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chance {
    /// Game item key.
    pub item: String,
    /// Price of one copy, in platinum.
    pub entry: u32,
    /// Expected sale price of one copy; `None` when the market cache has no price.
    pub worth: Option<u32>,
    /// Copies traded over the last 48 hours; `None` when the market cache has no data.
    pub volume: Option<u32>,
    /// Copies per trade.
    pub per_trade: u32,
    /// Total copies the open order asks for.
    pub copies: u32,
    /// Whether the account already has an open order on this item.
    pub listed: bool,
    /// Whether the item's trade price has been falling.
    pub falling: bool,
}

impl Chance {
    /// Copies per trade after the policy limit, from 1 to `most_per_trade`.
    pub fn carried(&self, policy: &Policy) -> u32 {
        self.per_trade.clamp(1, policy.most_per_trade.max(1))
    }

    /// Expected profit of one trade, in platinum, for a known `worth`.
    fn promises(&self, policy: &Policy, worth: u32) -> i64 {
        (i64::from(worth) - i64::from(self.entry)) * i64::from(self.carried(policy))
    }

    /// Expected profit of one trade, in platinum; `None` while `worth` is unknown.
    pub fn profit(&self, policy: &Policy) -> Option<i64> {
        self.worth.map(|worth| self.promises(policy, worth))
    }

    /// Cost of one trade, in platinum.
    pub fn exchange(&self, policy: &Policy) -> u32 {
        self.entry.saturating_mul(self.carried(policy))
    }

    /// Platinum the whole order commits: every copy it asks for, at least one trade's worth.
    pub fn tied(&self, policy: &Policy) -> u32 {
        let most = policy.most_per_order.max(self.carried(policy));
        self.entry.saturating_mul(self.copies.clamp(1, most))
    }
}

/// Account resources available now; purse values are `None` until an inventory dump reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Means {
    /// Platinum held; `None` until the purse is read.
    pub held: Option<u32>,
    /// Platinum not yet committed to orders; `None` until the purse is read.
    pub free: Option<u32>,
    /// Platinum already committed to open buy orders.
    pub tied: u32,
    /// Daily trades left; `None` until the purse is read.
    pub exchanges: Option<u32>,
    /// Open orders the agent placed.
    pub orders: u32,
    /// Open orders the user placed by hand; the agent's share of order slots shrinks around them.
    pub hand: u32,
}

/// Reason an opportunity was passed over; each has a one-word code for the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Blacklisted,
    NotWhitelisted,
    AlreadyListed,
    ThinVolume,
    TooDear,
    BelowFloor,
    NoCapital,
    NoExchanges,
    TooManyOrders,
    /// The purse has not been read, so no check that needs it can run.
    PurseUnknown,
    /// The market cache has no data on this variant yet.
    NoReading,
    /// The policy allowed it, but warframe.market rejected the order.
    Refused,
    /// No seller who can be traded with lists it, so there is no ask to resell a copy at.
    NoSeller,
    /// Its trade price has been falling, so a copy bought now may sell for less.
    Falling,
    /// Riven lot selection is off.
    RivensOff,
    /// Ranking the copy up would take more endo than is left after the endo already committed.
    NoEndo,
    /// Too few watched items have market data yet to choose among them.
    Warming,
}

impl Reason {
    /// Code the interface uses for this reason, e.g. `below-floor`.
    pub fn as_code(self) -> &'static str {
        match self {
            Reason::Blacklisted => "blacklisted",
            Reason::NotWhitelisted => "not-whitelisted",
            Reason::AlreadyListed => "already-listed",
            Reason::ThinVolume => "thin-volume",
            Reason::TooDear => "too-dear",
            Reason::BelowFloor => "below-floor",
            Reason::NoCapital => "no-capital",
            Reason::NoExchanges => "no-exchanges",
            Reason::TooManyOrders => "too-many-orders",
            Reason::PurseUnknown => "purse-unknown",
            Reason::NoReading => "no-reading",
            Reason::Refused => "refused",
            Reason::NoSeller => "no-seller",
            Reason::Falling => "falling",
            Reason::RivensOff => "rivens-off",
            Reason::NoEndo => "no-endo",
            Reason::Warming => "warming",
        }
    }
}

/// Whether the policy allows acting on `chance` with `means`; `Err` with the first failed check.
/// Private so that outside the module the only way to check is to build a `Permit`.
fn eval(policy: &Policy, chance: &Chance, means: &Means) -> Result<(), Reason> {
    if policy.blacklist.contains(&chance.item) {
        return Err(Reason::Blacklisted);
    }

    let capped = match policy.whitelist.is_empty() {
        true => None,
        false => Some(
            policy
                .whitelist
                .iter()
                .find(|line| line.item == chance.item)
                .ok_or(Reason::NotWhitelisted)?,
        ),
    };

    if chance.listed {
        return Err(Reason::AlreadyListed);
    }
    if means.orders >= seats(means.hand) {
        return Err(Reason::TooManyOrders);
    }

    let (Some(free), Some(exchanges)) = (means.free, means.exchanges) else {
        return Err(Reason::PurseUnknown);
    };

    if exchanges == 0 || exchanges <= policy.least_exchanges {
        return Err(Reason::NoExchanges);
    }

    let (Some(worth), Some(volume)) = (chance.worth, chance.volume) else {
        return Err(Reason::NoReading);
    };

    if volume < policy.least_volume() {
        return Err(Reason::ThinVolume);
    }
    if chance.falling {
        return Err(Reason::Falling);
    }
    if chance.exchange(policy) > policy.most_per_position {
        return Err(Reason::TooDear);
    }
    if let Some(line) = capped
        && chance.entry > line.most
    {
        return Err(Reason::TooDear);
    }

    let cap = ceiling(
        policy,
        &Means {
            free: Some(free),
            ..*means
        },
    );
    if means.tied.saturating_add(chance.tied(policy)) > cap {
        return Err(Reason::NoCapital);
    }
    if chance.promises(policy, worth) < i64::from(policy.least_profit) {
        return Err(Reason::BelowFloor);
    }

    Ok(())
}

/// Permission to place one order; it can only be built from a policy that allows the
/// opportunity it is built for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permit {
    item: String,
}

impl Permit {
    /// Permit for `chance`; `Err` with the reason when the policy does not allow it.
    pub fn issue(policy: &Policy, chance: &Chance, means: &Means) -> Result<Self, Reason> {
        eval(policy, chance, means)?;
        Ok(Self {
            item: chance.item.clone(),
        })
    }

    /// Permit to sell a copy already held. Only the blacklist refuses it; the caller enforces
    /// the minimum price.
    pub fn to_sell(policy: &Policy, item: &str) -> Result<Self, Reason> {
        if policy.blacklist.iter().any(|barred| barred == item) {
            return Err(Reason::Blacklisted);
        }
        Ok(Self {
            item: item.to_owned(),
        })
    }

    /// Item this permit is for; it allows no other item.
    pub fn item(&self) -> &str {
        &self.item
    }
}

/// Maximum platinum committed to open buy orders at once: `most_tied`, or `purse_share` percent
/// of the platinum held when `tied_from_purse` is set.
pub fn ceiling(policy: &Policy, means: &Means) -> u32 {
    match policy.tied_from_purse {
        true => {
            let held = u64::from(
                means
                    .held
                    .unwrap_or_else(|| means.free.unwrap_or(0).saturating_add(means.tied)),
            );
            (held * u64::from(policy.purse_share.min(100)) / 100) as u32
        }
        false => policy.most_tied,
    }
}

/// Maximum platinum in rivens: `riven_capital`, or `riven_share` percent of the platinum held
/// (at most what `purse_share` leaves) when `tied_from_purse` is set.
pub fn riven_ceiling(policy: &Policy, means: &Means) -> u32 {
    match policy.tied_from_purse {
        true => {
            let held = u64::from(
                means
                    .held
                    .unwrap_or_else(|| means.free.unwrap_or(0).saturating_add(means.tied)),
            );
            let left = 100 - policy.purse_share.min(100);
            (held * u64::from(policy.riven_share.min(left)) / 100) as u32
        }
        false => policy.riven_capital,
    }
}

/// Whether a riven lot at `price` may be bought when `in_rivens` platinum is already in rivens;
/// `Err` with the reason otherwise.
pub fn riven_permit(
    policy: &Policy,
    means: &Means,
    in_rivens: u32,
    price: u32,
) -> Result<(), Reason> {
    if !policy.rivens {
        return Err(Reason::RivensOff);
    }
    let (Some(free), Some(exchanges)) = (means.free, means.exchanges) else {
        return Err(Reason::PurseUnknown);
    };
    if exchanges == 0 || exchanges <= policy.least_exchanges {
        return Err(Reason::NoExchanges);
    }
    if price > free || in_rivens.saturating_add(price) > riven_ceiling(policy, means) {
        return Err(Reason::NoCapital);
    }
    Ok(())
}

/// Copies a bid at `entry` may ask for without committing more platinum than the ceiling leaves,
/// at most `most_per_order`. At least 1: whether even one copy fits is the permit's check.
pub fn copies(policy: &Policy, means: &Means, entry: u32) -> u32 {
    let most = policy.most_per_order.max(1);
    let room = ceiling(policy, means).saturating_sub(means.tied);
    match entry {
        0 => most,
        entry => (room / entry).clamp(1, most),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bid_asks_for_as_many_copies_as_the_ceiling_leaves_room_for() {
        let policy = Policy {
            most_tied: 500,
            most_per_order: 7,
            ..Policy::default()
        };
        let means = Means {
            held: None,
            free: Some(500),
            tied: 0,
            exchanges: Some(10),
            orders: 0,
            hand: 0,
        };

        assert_eq!(copies(&policy, &means, 20), 7, "seven fit at twenty apiece");
        assert_eq!(
            copies(&policy, &means, 100),
            5,
            "five is what five hundred buys"
        );
        assert_eq!(
            copies(&policy, &means, 600),
            1,
            "one at least, the permit judges it"
        );
    }

    #[test]
    fn what_is_already_tied_up_leaves_less_room_for_the_next_bid() {
        let policy = Policy {
            most_tied: 500,
            most_per_order: 7,
            ..Policy::default()
        };
        let means = Means {
            held: None,
            free: Some(100),
            tied: 400,
            exchanges: Some(10),
            orders: 3,
            hand: 0,
        };

        assert_eq!(copies(&policy, &means, 50), 2);
    }

    #[test]
    fn reading_the_ceiling_from_the_purse_follows_the_platinum_held() {
        let policy = Policy {
            most_tied: 5_000,
            tied_from_purse: true,
            most_per_order: 7,
            ..Policy::default()
        };
        let means = Means {
            held: None,
            free: Some(120),
            tied: 0,
            exchanges: Some(10),
            orders: 0,
            hand: 0,
        };

        assert_eq!(ceiling(&policy, &means), 120);
        assert_eq!(copies(&policy, &means, 40), 3);
    }

    fn chance(entry: u32, worth: u32) -> Chance {
        Chance {
            item: "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarPowerMaxModExpert".into(),
            entry,
            worth: Some(worth),
            volume: Some(60),
            per_trade: 1,
            copies: 1,
            listed: false,
            falling: false,
        }
    }

    fn means() -> Means {
        Means {
            held: None,
            free: Some(340),
            tied: 0,
            exchanges: Some(14),
            orders: 7,
            hand: 0,
        }
    }

    fn policy() -> Policy {
        Policy {
            most_tied: 240,
            ..Policy::default()
        }
    }

    #[test]
    fn a_chance_above_the_floor_is_allowed() {
        assert_eq!(eval(&policy(), &chance(40, 70), &means()), Ok(()));
    }

    #[test]
    fn each_copy_of_a_lot_clears_its_share_of_the_profit() {
        let policy = Policy {
            least_profit: 30,
            ..Policy::default()
        };

        assert_eq!(policy.least_per_copy(4), 8);
        assert_eq!(policy.least_per_copy(0), 30);
    }

    #[test]
    fn a_small_gain_is_below_the_floor() {
        assert_eq!(
            eval(&policy(), &chance(40, 47), &means()),
            Err(Reason::BelowFloor)
        );
    }

    #[test]
    fn the_purse_can_stand_in_for_the_bound_on_what_is_tied_up() {
        let held = Means {
            free: Some(300),
            tied: 100,
            ..means()
        };
        let mean = Policy {
            most_tied: 0,
            tied_from_purse: true,
            ..policy()
        };

        // The fixed limit allows nothing; the purse share allows 400.
        assert_eq!(eval(&mean, &chance(40, 70), &held), Ok(()));
        assert_eq!(
            eval(&policy_with_tied(0), &chance(40, 70), &held),
            Err(Reason::NoCapital)
        );
    }

    fn policy_with_tied(most: u32) -> Policy {
        Policy {
            most_tied: most,
            ..policy()
        }
    }

    #[test]
    fn a_wide_margin_on_a_cheap_item_still_fails_the_absolute_floor() {
        // A margin of 10 is 200 % of 5 platinum, but below the minimum profit of 20 for a flip,
        // which takes two trades.
        assert_eq!(
            eval(&policy(), &chance(5, 15), &means()),
            Err(Reason::BelowFloor)
        );
    }

    #[test]
    fn emptying_a_list_narrows_nothing() {
        let mut policy = policy();
        policy.whitelist.push(Capped {
            item: "/Lotus/Other".into(),
            most: 95,
        });
        assert!(eval(&policy, &chance(40, 70), &means()).is_err());

        policy.whitelist.clear();

        assert_eq!(eval(&policy, &chance(40, 70), &means()), Ok(()));
    }

    #[test]
    fn the_black_list_is_checked_before_anything_else() {
        let mut policy = policy();
        policy.blacklist.push(chance(40, 70).item);

        assert_eq!(
            eval(&policy, &chance(40, 70), &means()),
            Err(Reason::Blacklisted)
        );
    }

    #[test]
    fn a_permit_is_not_issued_for_a_denied_chance() {
        let mut policy = policy();
        policy.blacklist.push(chance(40, 70).item);

        assert!(Permit::issue(&policy, &chance(40, 70), &means()).is_err());
    }

    #[test]
    fn a_permit_names_the_one_item_it_is_for() {
        let taken = chance(40, 70);
        let permit = Permit::issue(&policy(), &taken, &means()).expect("allowed");

        assert_eq!(permit.item(), taken.item);
    }

    #[test]
    fn nothing_is_tied_up_before_the_ceiling_is_set() {
        assert_eq!(
            eval(&Policy::default(), &chance(40, 70), &means()),
            Err(Reason::NoCapital)
        );
    }

    #[test]
    fn an_item_we_already_stand_on_is_not_taken_twice() {
        let again = Chance {
            listed: true,
            falling: false,
            ..chance(40, 70)
        };

        assert_eq!(
            eval(&policy(), &again, &means()),
            Err(Reason::AlreadyListed)
        );
        assert!(Permit::issue(&policy(), &again, &means()).is_err());
    }

    #[test]
    fn what_is_already_tied_up_counts_against_the_ceiling() {
        let nearly = Means {
            tied: 220,
            ..means()
        };

        // 40 platinum more would put 260 against a ceiling of 240.
        assert_eq!(
            eval(&policy(), &chance(40, 70), &nearly),
            Err(Reason::NoCapital)
        );
        assert_eq!(eval(&policy(), &chance(40, 70), &means()), Ok(()));
    }

    #[test]
    fn the_day_running_out_of_exchanges_binds_by_itself() {
        let out = Means {
            exchanges: Some(0),
            ..means()
        };

        assert_eq!(
            eval(&policy(), &chance(40, 70), &out),
            Err(Reason::NoExchanges)
        );
    }

    #[test]
    fn the_ceiling_on_a_position_counts_the_whole_lot() {
        let six = Chance {
            per_trade: 6,
            ..chance(40, 70)
        };

        assert_eq!(six.exchange(&policy()), 240);
        assert_eq!(eval(&policy(), &six, &means()), Err(Reason::TooDear));
        assert_eq!(eval(&policy(), &chance(40, 70), &means()), Ok(()));
    }

    #[test]
    fn what_is_committed_is_every_copy_the_order_asks_for() {
        let stock = Chance {
            copies: 5,
            ..chance(40, 70)
        };
        let five = Policy {
            most_per_order: 5,
            ..policy()
        };

        assert_eq!(stock.tied(&five), 200);
        assert_eq!(
            stock.exchange(&five),
            40,
            "one exchange still takes one copy"
        );
    }

    #[test]
    fn no_more_copies_are_committed_than_the_policy_allows() {
        let stock = Chance {
            copies: 5,
            ..chance(40, 70)
        };
        let two = Policy {
            most_per_order: 2,
            ..policy()
        };

        assert_eq!(stock.tied(&two), 80);
    }

    #[test]
    fn what_the_market_cache_has_not_learned_is_not_a_thin_market() {
        let unknown = Chance {
            worth: None,
            volume: None,
            ..chance(40, 70)
        };
        let thin = Chance {
            volume: Some(1),
            ..chance(40, 70)
        };

        assert_eq!(eval(&policy(), &unknown, &means()), Err(Reason::NoReading));
        assert_eq!(eval(&policy(), &thin, &means()), Err(Reason::ThinVolume));
        assert_eq!(unknown.profit(&policy()), None);
    }

    #[test]
    fn a_price_alone_is_not_enough_to_judge() {
        let priced = Chance {
            worth: Some(70),
            volume: None,
            ..chance(40, 70)
        };

        assert_eq!(eval(&policy(), &priced, &means()), Err(Reason::NoReading));
    }

    #[test]
    fn what_was_bought_may_be_offered_back() {
        let item = chance(40, 70).item;
        assert!(Permit::to_sell(&policy(), &item).is_ok());

        let barred = Policy {
            blacklist: vec![item.clone()],
            ..policy()
        };
        assert_eq!(Permit::to_sell(&barred, &item), Err(Reason::Blacklisted));
    }

    #[test]
    fn a_purse_never_read_is_not_an_empty_one() {
        let unread = Means {
            free: None,
            exchanges: None,
            ..means()
        };

        assert_eq!(
            eval(&policy(), &chance(40, 70), &unread),
            Err(Reason::PurseUnknown)
        );
    }

    #[test]
    fn an_unread_purse_does_not_hide_a_standing_order() {
        let unread = Means {
            free: None,
            exchanges: None,
            ..means()
        };
        let again = Chance {
            listed: true,
            falling: false,
            ..chance(40, 70)
        };

        assert_eq!(eval(&policy(), &again, &unread), Err(Reason::AlreadyListed));
    }

    #[test]
    fn the_purse_share_bounds_what_is_tied_up() {
        let half = Policy {
            tied_from_purse: true,
            purse_share: 50,
            ..Policy::default()
        };
        let held = Means {
            free: Some(300),
            tied: 100,
            ..means()
        };

        assert_eq!(ceiling(&half, &held), 200);
    }

    #[test]
    fn the_exchanges_kept_for_the_person_stop_the_agent() {
        let keeping = Policy {
            least_exchanges: 14,
            ..policy()
        };

        assert_eq!(
            eval(&keeping, &chance(40, 70), &means()),
            Err(Reason::NoExchanges)
        );
    }

    #[test]
    fn a_pair_is_watched_when_it_trades_often_and_dear_enough_to_clear_a_flip() {
        let policy = Policy {
            least_per_day: 20,
            least_profit: 13,
            ..Policy::default()
        };

        assert!(policy.worth_watching(20.0, 14.0));
        assert!(!policy.worth_watching(19.9, 200.0));
        assert!(!policy.worth_watching(500.0, 13.0));
        assert!(policy.leaves_room(80.0, 67));
        assert!(!policy.leaves_room(80.0, 68));
    }

    fn rivens() -> Policy {
        Policy {
            rivens: true,
            riven_capital: 300,
            ..policy()
        }
    }

    #[test]
    fn rivens_are_bought_only_once_the_person_lets_the_agent() {
        assert_eq!(
            riven_permit(&policy(), &means(), 0, 100),
            Err(Reason::RivensOff)
        );
        assert_eq!(riven_permit(&rivens(), &means(), 0, 100), Ok(()));
    }

    #[test]
    fn a_riven_fits_its_own_capital_less_what_already_sits_in_rivens() {
        assert_eq!(riven_permit(&rivens(), &means(), 200, 100), Ok(()));
        assert_eq!(
            riven_permit(&rivens(), &means(), 250, 100),
            Err(Reason::NoCapital)
        );
    }

    #[test]
    fn a_riven_is_paid_from_the_platinum_the_item_bids_leave_free() {
        let bids = Means {
            free: Some(80),
            tied: 260,
            ..means()
        };

        assert_eq!(
            riven_permit(&rivens(), &bids, 0, 100),
            Err(Reason::NoCapital)
        );
    }

    #[test]
    fn a_riven_takes_an_exchange_the_items_count_on_too() {
        let keeping = Policy {
            least_exchanges: 14,
            ..rivens()
        };

        assert_eq!(
            riven_permit(&keeping, &means(), 0, 100),
            Err(Reason::NoExchanges)
        );
    }

    #[test]
    fn read_from_the_purse_rivens_get_their_share_cut_to_what_items_leave() {
        let shared = Policy {
            tied_from_purse: true,
            purse_share: 70,
            riven_share: 50,
            ..rivens()
        };
        let held = Means {
            held: Some(1_000),
            ..means()
        };

        assert_eq!(ceiling(&shared, &held), 700);
        assert_eq!(riven_ceiling(&shared, &held), 300);
    }
}
