use median_core::{DAY, Holding, Inventory, Point, Traded, curve as closed_curve, seconds};
use median_ipc::{Curve, Spot, StandingDay};
use median_market::Side;

use crate::catalog::Catalog;
use crate::market::prices::{listed, unheld};
use crate::market::store::{Bucket, Span, Standing, Store};

/// Price chart series of one item over the last `days` days (closed trades, sell and buy
/// orders), at its own rank or, with `top_rank`, at max rank; empty when the item is not listed.
pub async fn curve(
    catalog: &Catalog,
    store: &Store,
    item: &str,
    variant: &str,
    days: u32,
    top_rank: bool,
) -> Result<Curve, String> {
    let held = Inventory {
        holdings: vec![Holding {
            item: item.to_owned(),
            variant: median_core::Variant(variant.to_owned()),
            detail: median_engine::detail(&median_core::Variant(variant.to_owned())),
            count: 1,
        }],
        ..Inventory::default()
    };
    let copy = match listed(catalog, store, &held).await?.into_iter().next() {
        Some(copy) => copy,
        None => match unheld(catalog, store, item).await? {
            Some(copy) => copy,
            None => return Ok(Curve::default()),
        },
    };

    let max_rank = match copy.of.rank {
        Some(_) => store
            .max_rank(&copy.slug)
            .await
            .map_err(|e| e.to_string())?,
        None => None,
    };
    let mut of = copy.of;
    if top_rank && let Some(top) = max_rank {
        of.rank = Some(top);
    }

    let from = seconds() - i64::from(days) * DAY;
    let buckets = store
        .closed(&copy.slug, &of, Span::Day)
        .await
        .map_err(|e| e.to_string())?;
    let traded: Vec<Traded> = buckets.iter().map(Bucket::traded).collect();
    let standing = |side| store.standing(&copy.slug, &of, side, from);
    let selling = standing(Side::Sell).await.map_err(|e| e.to_string())?;
    let buying = standing(Side::Buy).await.map_err(|e| e.to_string())?;
    Ok(Curve {
        closed: spots(&closed_curve(&traded, from)),
        selling: selling.iter().map(standing_day).collect(),
        buying: buying.iter().map(standing_day).collect(),
        max_rank,
    })
}

fn spots(points: &[Point]) -> Vec<Spot> {
    points
        .iter()
        .map(|point| Spot {
            at: point.at.max(0) as u32,
            platinum: point.platinum as f32,
            weight: point.weight.min(u64::from(u32::MAX)) as u32,
        })
        .collect()
}

fn standing_day(day: &Standing) -> StandingDay {
    StandingDay {
        at: day.starts.max(0) as u32,
        best: match day.side {
            Side::Sell => day.min_price,
            Side::Buy => day.max_price,
        } as f32,
        median: day.median as f32,
        orders: day.orders.min(u64::from(u32::MAX)) as u32,
    }
}
