use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

use median_core::{DAY, Holder, Instant, Interest, Need, seconds};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::market::needs::Needs;
use crate::market::store::Store;
use crate::state_sink::{SharedState, from_policy};

/// How far back the trade volume and median of a pair are read.
const WINDOW: Instant = 7 * DAY;

/// How far back the observed bids of a pair are read.
const BIDS: Instant = DAY;

/// Interval between rebuilds of the heat map.
const RECOUNT: Duration = Duration::from_secs(900);

/// Retry delay while the market cache has no trades yet or a read fails.
const RETRY: Duration = Duration::from_secs(60);

type Pair = (String, String);

/// Pairs traded often enough and at a high enough price to flip, split into hot pairs, whose
/// usual highest bid leaves a margin, and sampled pairs, whose bid does not.
#[derive(Clone, Default)]
pub struct Heat(Arc<RwLock<Hot>>);

#[derive(Default)]
struct Hot {
    items: Vec<String>,
    pairs: HashSet<Pair>,
}

impl Heat {
    /// Items with at least one hot pair, sorted by slug; empty until the first build.
    pub fn items(&self) -> Vec<String> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .items
            .clone()
    }

    /// Whether this pair is hot or sampled.
    pub fn holds(&self, slug: &str, variant: &str) -> bool {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .pairs
            .contains(&(slug.to_owned(), variant.to_owned()))
    }

    fn set(&self, hot: &HashSet<Pair>, sampled: &HashSet<Pair>) {
        let items: BTreeSet<String> = hot.iter().map(|(slug, _)| slug.clone()).collect();
        let mut held = self.0.write().unwrap_or_else(PoisonError::into_inner);
        held.items = items.into_iter().collect();
        held.pairs = hot.union(sampled).cloned().collect();
    }
}

/// Splits pairs into hot ones, whose usual highest bid leaves a margin by `room`, and the rest.
fn split(
    traded: Vec<(Pair, f64)>,
    bids: &HashMap<Pair, u32>,
    room: impl Fn(f64, u32) -> bool,
) -> (HashSet<Pair>, HashSet<Pair>) {
    let (hot, sampled): (Vec<_>, Vec<_>) = traded
        .into_iter()
        .partition(|(pair, median)| bids.get(pair).is_some_and(|bid| room(*median, *bid)));
    let pairs = |side: Vec<(Pair, f64)>| side.into_iter().map(|(pair, _)| pair).collect();
    (pairs(hot), pairs(sampled))
}

/// One need per pair, with this interest.
fn needs_of(pairs: &HashSet<Pair>, interest: Interest) -> impl Iterator<Item = Need> + '_ {
    pairs.iter().map(move |(slug, variant)| Need {
        slug: slug.clone(),
        variant: variant.clone(),
        interest,
    })
}

/// Rebuilds the heat map every `RECOUNT` with the current policy, declaring hot pairs as
/// `Watched` needs and the rest as `Sampled`.
pub async fn tend(
    store: Arc<Store>,
    heat: Heat,
    shared: SharedState,
    needs: Needs,
) -> Result<(), String> {
    loop {
        let policy = {
            let held = shared.0.read().unwrap_or_else(PoisonError::into_inner);
            from_policy(&held.policy)
        };
        let now = seconds();
        let again = match store.demand(now - WINDOW).await {
            Ok(demand) if demand.is_empty() => RETRY,
            Ok(demand) => {
                let traded: Vec<(Pair, f64)> = demand
                    .into_iter()
                    .filter(|pair| policy.worth_watching(pair.per_day, pair.median))
                    .map(|pair| ((pair.slug, pair.variant), pair.median))
                    .collect();
                let asked = traded.iter().map(|(pair, _)| pair.clone()).collect();
                match store.bids_since(asked, now - BIDS).await {
                    Ok(bids) => {
                        let (hot, sampled) =
                            split(traded, &bids, |median, bid| policy.leaves_room(median, bid));
                        needs.declare(
                            Holder::Heat,
                            needs_of(&hot, Interest::Watched)
                                .chain(needs_of(&sampled, Interest::Sampled))
                                .collect(),
                        );
                        heat.set(&hot, &sampled);
                        info!(
                            items = heat.items().len(),
                            hot = hot.len(),
                            sampled = sampled.len(),
                            "market heat map built"
                        );
                        RECOUNT
                    }
                    Err(why) => {
                        warn!(error = %why, "market heat map bids read failed");
                        RETRY
                    }
                }
            }
            Err(why) => {
                warn!(error = %why, "market heat map build failed");
                RETRY
            }
        };
        sleep(again).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(slug: &str, variant: &str) -> Pair {
        (slug.to_owned(), variant.to_owned())
    }

    #[test]
    fn an_item_is_listed_once_however_many_of_its_pairs_are_watched() {
        let heat = Heat::default();
        heat.set(
            &HashSet::from([
                pair("serration", "mod_rank=0"),
                pair("serration", "mod_rank=10"),
                pair("ash_prime_set", ""),
            ]),
            &HashSet::new(),
        );

        assert_eq!(heat.items(), vec!["ash_prime_set", "serration"]);
        assert!(heat.holds("serration", "mod_rank=10"));
        assert!(!heat.holds("serration", "mod_rank=5"));
    }

    #[test]
    fn a_pair_without_room_is_weighed_but_not_waited_on() {
        let heat = Heat::default();
        heat.set(
            &HashSet::from([pair("archon_stretch", "mod_rank=10")]),
            &HashSet::from([pair("blind_rage", "mod_rank=0")]),
        );

        assert_eq!(heat.items(), vec!["archon_stretch"]);
        assert!(heat.holds("blind_rage", "mod_rank=0"));
    }

    #[test]
    fn only_a_pair_whose_usual_bid_leaves_room_is_hot() {
        let traded = vec![
            (pair("wide", ""), 90.0),
            (pair("narrow", ""), 20.0),
            (pair("unseen", ""), 90.0),
        ];
        let bids = HashMap::from([(pair("wide", ""), 60), (pair("narrow", ""), 15)]);

        let (hot, sampled) = split(traded, &bids, |median, bid| median - f64::from(bid) >= 7.0);

        assert_eq!(hot, HashSet::from([pair("wide", "")]));
        assert_eq!(
            sampled,
            HashSet::from([pair("narrow", ""), pair("unseen", "")])
        );
    }
}
