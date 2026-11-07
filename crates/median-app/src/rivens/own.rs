use median_ipc::OwnAuction;
use median_market::Auction;

use super::lot::{riven_of, weapon_of};
use crate::catalog::Catalog;

/// The account's riven auctions that are not closed, newest first.
pub async fn own_auctions(
    catalog: &Catalog,
    auctions: Vec<Auction>,
) -> Result<Vec<OwnAuction>, String> {
    let mut own = Vec::with_capacity(auctions.len());
    for auction in auctions.into_iter().filter(|auction| !auction.closed) {
        let weapon = weapon_of(catalog, &auction).await?;
        let riven = match &weapon {
            Some(weapon) => riven_of(catalog, weapon.clone(), &auction).await?,
            None => None,
        };
        own.push(OwnAuction {
            id: auction.id,
            weapon,
            riven,
            name: auction.name,
            rank: auction.rank,
            starting_price: auction.starting_price,
            buyout_price: auction.buyout_price,
            top_bid: auction.top_bid,
            is_direct_sell: auction.is_direct_sell,
            visible: auction.visible,
            created: auction.created,
        });
    }
    own.sort_by(|one, other| other.created.cmp(&one.created));
    Ok(own)
}
