use std::sync::Arc;

use median_core::{
    AgentEvent, Event, Means, Pick, Policy, Riven, RivenPick, pick, riven_permit, seconds,
    worth_a_look,
};
use median_engine::Input;
use median_market::Auction;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::agent::gear::Gear;
use crate::market::Fresh;
use crate::rivens::{self, Slices, Weekly};

/// Most riven auctions evaluated against the market in one pass.
const MOST_LOOKS: usize = 12;

/// Data the riven picker reads besides `Gear`.
pub struct Sources {
    pub gear: Arc<Gear>,
    pub slices: Arc<Slices>,
    pub weekly: Arc<Weekly>,
    pub fresh: Arc<Fresh>,
}

/// Spending limits of one pass: the policy, the account's means, and the platinum in rivens
/// bought and not yet sold.
struct Purse {
    policy: Policy,
    means: Means,
    in_rivens: u32,
}

/// Picks riven auctions worth buying from the new ones in the auction feed, while the agent
/// runs and the policy allows rivens.
pub async fn tend(sources: Arc<Sources>, running: Arc<Mutex<bool>>) -> Result<(), String> {
    loop {
        let lots = sources.fresh.take().await;
        let policy = sources.gear.policy();
        if !*running.lock().await || !policy.rivens {
            continue;
        }
        let standing = sources.gear.standing_now().await;
        let purse = Purse {
            means: sources.gear.means(&standing),
            in_rivens: sources
                .gear
                .history
                .in_rivens()
                .await
                .map_err(|e| e.to_string())?,
            policy,
        };
        let mut looked = 0;
        let mut picked = 0;
        for auction in lots.iter().rev() {
            if looked == MOST_LOOKS {
                break;
            }
            let Some(family) = deserves_a_look(&sources, &purse, auction).await? else {
                continue;
            };
            looked += 1;
            if weigh(&sources, &purse, auction, &family).await {
                picked += 1;
            }
        }
        debug!(
            new_auctions = lots.len(),
            evaluated = looked,
            picked,
            "riven auctions evaluated"
        );
    }
}

/// English weapon family name of an affordable auction whose buyout is at most last week's
/// median trade price; `None` for any other auction.
async fn deserves_a_look(
    sources: &Sources,
    purse: &Purse,
    auction: &Auction,
) -> Result<Option<String>, String> {
    let Some(buyout) = auction.buyout_price else {
        return Ok(None);
    };
    if let Err(reason) = riven_permit(&purse.policy, &purse.means, purse.in_rivens, buyout) {
        debug!(
            auction_id = auction.id,
            reason = reason.as_code(),
            "riven auction skipped"
        );
        return Ok(None);
    }
    let Some(family) = rivens::family_of(&sources.gear.catalog, auction).await? else {
        return Ok(None);
    };
    let traded = sources.weekly.traded(&family, auction.rerolls > 0);
    Ok(worth_a_look(f64::from(buyout), traded).then_some(family))
}

/// Evaluates one auction against the market and sends a pick to the engine as an agent event;
/// returns whether it was picked.
async fn weigh(sources: &Sources, purse: &Purse, auction: &Auction, family: &str) -> bool {
    let gear = &sources.gear;
    let view = rivens::lot_view(
        &gear.catalog,
        &gear.gateway,
        &sources.slices,
        &sources.weekly,
        auction,
    );
    match view.await {
        Ok(Some((riven, view))) => {
            let Some(chosen) = pick(&view, &purse.policy) else {
                return false;
            };
            let found = to_pick(auction, family, riven, &chosen);
            info!(
                auction_id = found.id,
                seller = found.seller,
                buyout = found.buyout,
                estimated_value = found.worth,
                offer = found.offer,
                "riven auction picked"
            );
            let picked = Event::Agent(AgentEvent::Picked(Box::new(found)));
            let _ = gear.tx.send(Input::Command(picked)).await;
            true
        }
        Ok(None) => false,
        Err(why) => {
            warn!(auction_id = auction.id, error = %why, "riven auction evaluation failed");
            false
        }
    }
}

fn to_pick(auction: &Auction, family: &str, riven: Riven, chosen: &Pick) -> RivenPick {
    let (seller, status) = auction
        .owner
        .as_ref()
        .map(|owner| (owner.ingame_name.clone(), owner.status.clone()))
        .unwrap_or_default();
    let offer = chosen.offer.round() as u32;
    RivenPick {
        id: auction.id.clone(),
        at: seconds(),
        name: auction.name.clone(),
        whisper: whisper(&seller, family, &auction.name, offer),
        riven,
        rank: u8::try_from(auction.rank).unwrap_or(u8::MAX),
        seller,
        status,
        reason: chosen.reason,
        buyout: chosen.buyout.round() as u32,
        worth: chosen.worth.round() as u32,
        offer,
    }
}

/// In-game whisper asking the seller to accept `offer` platinum for their riven.
fn whisper(seller: &str, family: &str, name: &str, offer: u32) -> String {
    let name = median_core::riven_cased(name);
    format!(
        "/w {seller} Hi! I'd like to buy your {family} {name} riven from warframe.market. Would you take {offer} platinum?"
    )
}
