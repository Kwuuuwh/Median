use std::cell::RefCell;
use std::collections::HashMap;

use median_core::{
    Odds, Riven as CoreRiven, RivenStat as CoreRivenStat, StatForm as CoreStatForm, odds,
};
use median_ipc::{ListedStat, RerollOdds, Riven, RivenAnalog, RivenChoices};
use median_market::Auction;

use super::appraisal::{to_core, to_form};
use super::asks::{Armory, Slices, pool_stats, to_band};
use super::trades::Weekly;
use crate::catalog::{Catalog, Pooled};
use crate::market::Gateway;

/// Simulated rerolls per lock choice used to estimate the odds.
pub(super) const REROLL_SAMPLES: u32 = 4000;

/// Fixed seed of the simulated rerolls, so a riven always gets the same odds.
pub(super) const REROLL_SEED: u64 = 0x0052_4956_454E;

/// Rivens one transmutation consumes to produce one riven.
const TRANSMUTED: f64 = 4.0;

/// Number of similar auctions shown next to the riven.
const ANALOGS: usize = 8;

/// Value of `riven` when sold (price band) and when transmuted (a quarter of a veiled riven's
/// price), with up to `ANALOGS` similar auctions; labels in `lang`. `None` when the weapon is
/// not in the catalog.
pub async fn choices(
    catalog: &Catalog,
    gateway: &Gateway,
    slices: &Slices,
    weekly: &Weekly,
    lang: &str,
    riven: Riven,
) -> Result<Option<RivenChoices>, String> {
    let Some(armory) = Armory::of(catalog, lang, riven.weapon.clone()).await? else {
        return Ok(None);
    };
    let transmute = weekly
        .veiled(armory.class.trade_kind())
        .map(|price| price / TRANSMUTED);
    let held_bounds = armory.bounds(weekly, riven.rerolls > 0);
    let Some((asks, auctions)) = armory.asks(gateway, slices, &riven).await? else {
        return Ok(Some(RivenChoices {
            sell: None,
            transmute,
            analogs: Vec::new(),
        }));
    };
    let core = to_core(&riven);
    let sell = asks.band(&core, held_bounds);
    if let Some(band) = &sell {
        slices.keep_band(&riven, band).await;
    }

    let by_id: HashMap<&str, &Auction> = auctions
        .iter()
        .map(|auction| (auction.id.as_str(), auction))
        .collect();
    let analogs = asks
        .analogs(&core, ANALOGS)
        .into_iter()
        .filter_map(|analog| {
            let auction = by_id.get(analog.id.as_str())?;
            Some(to_analog(auction, analog.likeness, &armory.pool))
        })
        .collect();
    Ok(Some(RivenChoices {
        sell: sell.as_ref().map(to_band),
        transmute,
        analogs,
    }))
}

/// For no lock and for each stat locked: the chance that a reroll of `riven` is priced above
/// its current median, the expected kuva, and the mean price of those better rolls. `None` when
/// the weapon is not in the catalog or the riven cannot be priced.
pub async fn rolls(
    catalog: &Catalog,
    gateway: &Gateway,
    slices: &Slices,
    weekly: &Weekly,
    riven: Riven,
) -> Result<Option<Vec<RerollOdds>>, String> {
    let Some(armory) = Armory::of(catalog, "en", riven.weapon.clone()).await? else {
        return Ok(None);
    };
    let held_bounds = armory.bounds(weekly, riven.rerolls > 0);
    let rolled_bounds = armory.bounds(weekly, true);
    let Some((asks, _)) = armory.asks(gateway, slices, &riven).await? else {
        return Ok(None);
    };

    tokio::task::spawn_blocking(move || {
        let riven = to_core(&riven);
        let Some(standing) = asks.band(&riven, held_bounds) else {
            return Ok(None);
        };
        let bases = pool_stats(&armory.pool);
        let found = odds(
            &riven,
            &bases,
            REROLL_SAMPLES,
            REROLL_SEED,
            standing.middle,
            priced(|rolled| asks.band(rolled, rolled_bounds).map(|band| band.middle)),
        );
        Ok(Some(found.iter().map(to_odds).collect()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Memoizes `price` by the sorted buff and curse tags, so a stat combination drawn again is not
/// priced again.
pub(super) fn priced(
    price: impl Fn(&CoreRiven) -> Option<f64>,
) -> impl Fn(&CoreRiven) -> Option<f64> {
    let known: RefCell<HashMap<String, Option<f64>>> = RefCell::default();
    move |riven| {
        let tags = |stats: &[CoreRivenStat]| {
            let mut tags: Vec<&str> = stats.iter().map(|stat| stat.tag.as_str()).collect();
            tags.sort_unstable();
            tags.join(",")
        };
        let key = format!("{}|{}", tags(&riven.buffs), tags(&riven.curses));
        if let Some(found) = known.borrow().get(&key) {
            return *found;
        }
        let found = price(riven);
        known.borrow_mut().insert(key, found);
        found
    }
}

/// Converts reroll odds to their IPC form.
pub(super) fn to_odds(odds: &Odds) -> RerollOdds {
    RerollOdds {
        locked: odds.locked.clone(),
        chance: odds.chance,
        expected_kuva: odds.expected_kuva,
        better_mean: odds.better_mean,
    }
}

/// Converts a similar auction to its IPC form, with stat labels from `pool`; stats not in
/// `pool` are dropped.
fn to_analog(auction: &Auction, likeness: f64, pool: &[Pooled]) -> RivenAnalog {
    let stats = auction
        .stats
        .iter()
        .filter_map(|stat| {
            let pooled = pool
                .iter()
                .find(|pooled| pooled.slug.as_deref() == Some(stat.slug.as_str()))?;
            let form = CoreStatForm::from_name(&pooled.form)?;
            Some(ListedStat {
                tag: pooled.tag.clone(),
                label: pooled.label.clone(),
                form: to_form(form),
                amount: form.amount(stat.value),
                is_curse: !stat.positive,
            })
        })
        .collect();
    RivenAnalog {
        id: auction.id.clone(),
        price: auction.buyout_price.unwrap_or(auction.starting_price),
        rank: auction.rank,
        rerolls: auction.rerolls,
        stats,
        seller: auction
            .owner
            .as_ref()
            .map(|owner| owner.ingame_name.clone()),
        status: auction.owner.as_ref().map(|owner| owner.status.clone()),
        likeness,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn riven(buffs: &[&str]) -> CoreRiven {
        CoreRiven {
            weapon: "/Gun".into(),
            mastery: 8,
            rerolls: 0,
            polarity: None,
            buffs: buffs
                .iter()
                .map(|tag| CoreRivenStat {
                    tag: (*tag).into(),
                    roll: 0,
                })
                .collect(),
            curses: Vec::new(),
        }
    }

    #[test]
    fn the_same_stats_in_any_order_are_priced_once() {
        let asked = Cell::new(0);
        let price = priced(|_| {
            asked.set(asked.get() + 1);
            Some(100.0)
        });

        assert_eq!(price(&riven(&["A", "B"])), Some(100.0));
        assert_eq!(price(&riven(&["B", "A"])), Some(100.0));
        assert_eq!(price(&riven(&["A", "C"])), Some(100.0));
        assert_eq!(asked.get(), 2);
    }
}
