use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use median_core::seconds;
use median_market::{ClosedBucket, DayFiles, day_of};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::market::store::{DAY_FILE, Standing, Store};
use median_core::DAY;

/// Number of past days to import.
const DAYS: i64 = 7;

/// Extra days to try beyond `DAYS`, since the newest day file is published a day late.
const REACH: i64 = 3;

const PAUSE: Duration = Duration::from_secs(6 * 3_600);

/// Delay before retrying when a day file failed to download. The host sometimes stalls a
/// response body, and a missed day leaves the whole market without prices for 48 hours.
const RETRY: Duration = Duration::from_secs(5 * 60);

/// Age, in seconds, after which an observation is deleted.
const KEPT: i64 = 30 * DAY;

pub async fn tend(store: Arc<Store>, day_files: Arc<DayFiles>) -> Result<(), String> {
    loop {
        let again = match warm(&store, &day_files).await {
            Ok(true) => PAUSE,
            Ok(false) => RETRY,
            Err(trouble) => {
                warn!(error = %trouble, "market daily stats read failed");
                RETRY
            }
        };
        sleep(again).await;
    }
}

/// Imports up to `DAYS` day files into the market cache, skipping days imported before; returns
/// whether every attempted download succeeded, and `false` while the item list is empty.
async fn warm(store: &Store, day_files: &DayFiles) -> Result<bool, String> {
    let by_id = store.slugs_by_id().await.map_err(|e| e.to_string())?;
    if by_id.is_empty() {
        return Ok(false);
    }

    let now = seconds();
    let mut read = 0;
    let mut refused = false;
    for back in 1..=DAYS + REACH {
        if read >= DAYS {
            break;
        }
        let date = day_of(now - back * DAY);
        if store
            .stamped(&date, DAY_FILE)
            .await
            .map_err(|e| e.to_string())?
            .is_some()
        {
            read += 1;
            continue;
        }

        let rows = match day_files.day(&date).await {
            Ok(Some(rows)) => rows,
            Ok(None) => continue,
            Err(trouble) => {
                warn!(date, error = %trouble, "market daily stats day read failed");
                refused = true;
                continue;
            }
        };

        let mut closed: BTreeMap<String, Vec<ClosedBucket>> = BTreeMap::new();
        let mut standing: BTreeMap<String, Vec<Standing>> = BTreeMap::new();
        let mut strangers = 0;
        for row in rows {
            let Some(slug) = by_id.get(&row.item_id) else {
                strangers += 1;
                continue;
            };
            if row.is_closed() {
                closed.entry(slug.clone()).or_default().push(row.bucket);
            } else if let Some(day) = row
                .side()
                .and_then(|side| Standing::of_day(&row.bucket, side))
            {
                standing.entry(slug.clone()).or_default().push(day);
            }
        }

        let items = closed.len();
        let landed = store
            .keep_days(closed.into_iter().collect())
            .await
            .map_err(|e| e.to_string())?;
        store
            .keep_standing(standing.into_iter().collect())
            .await
            .map_err(|e| e.to_string())?;
        store
            .stamp(&date, DAY_FILE)
            .await
            .map_err(|e| e.to_string())?;

        read += 1;
        info!(
            date,
            items,
            rows_written = landed,
            unknown_items = strangers,
            "market daily stats imported"
        );
    }

    let priced = store.priced_items().await.map_err(|e| e.to_string())?;
    info!(priced, listed = by_id.len(), "market prices ready");

    let forgotten = store
        .forget_observations(now - KEPT)
        .await
        .map_err(|e| e.to_string())?;
    if forgotten > 0 {
        info!(rows = forgotten, "old market observations removed");
    }
    Ok(!refused)
}
