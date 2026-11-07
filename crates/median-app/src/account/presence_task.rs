use std::sync::Arc;

use median_core::seconds;
use tracing::warn;

use crate::account::Store;
use crate::state_sink::Presence;

/// Records each in-game interval reported by the account's warframe.market status, after closing
/// an interval the previous run left open; fails when the status channel closes.
pub async fn tend(presence: Presence, history: Arc<Store>) -> Result<(), String> {
    if let Err(why) = history.settle_presence().await {
        warn!(error = %why, "in-game interval from previous run not closed");
    }
    let mut watch = presence.watch();
    let mut was = false;
    loop {
        let in_game = *watch.borrow_and_update();
        if in_game != was {
            if let Err(why) = history.mark_presence(in_game, seconds()).await {
                warn!(error = %why, in_game, "in-game interval not recorded");
            }
            was = in_game;
        }
        watch
            .changed()
            .await
            .map_err(|_| "the status is no longer told".to_owned())?;
    }
}
