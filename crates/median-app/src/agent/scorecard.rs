use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use median_core::{
    DAY, Deal, Instant, Outcome, Rates, Scorecard, Traded, burned, depth, fills, per_exchange,
    seconds, stretches,
};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::account;
use crate::agent::gear::Gear;
use crate::market::{self, Bucket};

/// How often the scorecard is written to the log.
const PAUSE: Duration = Duration::from_secs(3_600);

/// How far back the logged scorecard reaches, in seconds.
const WINDOW: Instant = 14 * DAY;

/// Every hour, measures the fill rate of the agent's bids at each depth over all recorded
/// bids, passes it to the agent, and logs the scorecard of the last `WINDOW`.
pub async fn tend(gear: Arc<Gear>) -> Result<(), String> {
    loop {
        let now = seconds();
        match scorecard(&gear.history, &gear.market_cache, 0, now).await {
            Ok(whole) => gear.learn(Rates::of(&whole.fills)),
            Err(why) => warn!(error = %why, "agent fill rates read failed"),
        }
        match scorecard(&gear.history, &gear.market_cache, now - WINDOW, now).await {
            Ok(card) => {
                let fills: Vec<String> = card
                    .fills
                    .iter()
                    .map(|fills| {
                        format!(
                            "-{}0%={}/{} day={:.2}",
                            fills.depth.tenths(),
                            fills.filled,
                            fills.stretches,
                            fills.within_day
                        )
                    })
                    .collect();
                let burned: u32 = card.burned.iter().map(|day| day.exchanges).sum();
                // Hours the market showed the account in game, the only hours a bid can fill.
                let in_game: Instant = gear
                    .history
                    .presence_since(now - WINDOW, now)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .map(|(began, ended)| ended - began.max(&(now - WINDOW)))
                    .sum();
                info!(
                    fills = fills.join(" "),
                    platinum_per_trade = card.per_exchange.map(|platinum| format!("{platinum:.1}")),
                    trades_used = burned,
                    days = card.burned.len(),
                    in_game_hours = format!("{:.1}", in_game as f64 / 3_600.0),
                    "agent scorecard"
                );
            }
            Err(why) => warn!(error = %why, "agent scorecard read failed"),
        }
        sleep(PAUSE).await;
    }
}

/// Scorecard of the agent's trading over the last `days` days, in IPC form.
pub async fn scorecard_over(
    history: &account::Store,
    market_cache: &market::Store,
    days: u32,
) -> Result<median_ipc::Scorecard, String> {
    let now = seconds();
    let card = scorecard(history, market_cache, now - Instant::from(days) * DAY, now).await?;
    Ok(median_ipc::Scorecard {
        fills: card
            .fills
            .iter()
            .map(|fills| median_ipc::DepthFills {
                tenths: fills.depth.tenths(),
                stretches: u32::try_from(fills.stretches).unwrap_or(u32::MAX),
                filled: u32::try_from(fills.filled).unwrap_or(u32::MAX),
                within_day: fills.within_day,
            })
            .collect(),
        per_exchange: card.per_exchange,
        burned: card
            .burned
            .iter()
            .map(|day| median_ipc::BurnedDay {
                day: u32::try_from(day.day).unwrap_or_default(),
                exchanges: day.exchanges,
            })
            .collect(),
    })
}

/// Scorecard of the agent's trading from `from` to `now`: fill rates by depth, platinum per
/// trade, and trades used per day.
pub async fn scorecard(
    history: &account::Store,
    market_cache: &market::Store,
    from: Instant,
    now: Instant,
) -> Result<Scorecard, String> {
    let lives = history.bid_lives(from).await.map_err(|e| e.to_string())?;
    let pairs: BTreeSet<(String, String)> = lives
        .iter()
        .map(|lived| (lived.slug.clone(), lived.variant.clone()))
        .collect();
    let recent = market_cache
        .recent(pairs.into_iter().collect())
        .await
        .map_err(|e| e.to_string())?;
    let hours: BTreeMap<&(String, String), Vec<Traded>> = recent
        .iter()
        .map(|(pair, held)| (pair, held.hour.iter().map(Bucket::traded).collect()))
        .collect();

    let exposures: Vec<_> = lives
        .iter()
        .flat_map(|lived| {
            let traded = hours
                .get(&(lived.slug.clone(), lived.variant.clone()))
                .map_or(&[][..], Vec::as_slice);
            stretches(&lived.life, now)
                .into_iter()
                .filter(|stretch| stretch.at >= from)
                .filter_map(move |stretch| {
                    depth(stretch.platinum, traded, stretch.at)
                        .map(|depth| (depth, stretch.exposure))
                })
        })
        .collect();

    let made = history
        .exchanges_since(0)
        .await
        .map_err(|e| e.to_string())?;
    let deals: Vec<Deal> = made
        .iter()
        .filter(|exchange| exchange.agent)
        .map(account::Exchange::deal)
        .collect();

    let left = history
        .exchanges_left_since(from)
        .await
        .map_err(|e| e.to_string())?;
    let swapped: Vec<Instant> = history
        .swaps_since(from)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|swap| swap.outcome == Outcome::Done)
        .map(|swap| swap.at)
        .collect();

    Ok(Scorecard {
        fills: fills(&exposures),
        per_exchange: per_exchange(&deals, from),
        burned: burned(&left, &swapped, now),
    })
}
