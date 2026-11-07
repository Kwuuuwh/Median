use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use deadpool_sqlite::{InteractError, Pool, PoolError};
use median_core::{
    Acquired, Affiliation, Crafting, DAY, Equipped, Formed, Haul, Holding, Instant, Intrinsic,
    Inventory, Mastered, Mission, Profile, Progression, Purse, Variant, changes, haul_gains,
    seconds,
};
use median_engine::detail;
use rusqlite::{Connection, OptionalExtension, params};
use tokio::sync::Mutex;

use crate::account::db;
use crate::account::{
    Answered, Choice, Conversion, Exchange, Lived, Made, Position, RivenBought, Standing,
};
use crate::schema::OpenError;

/// Account holdings: copy count per item and variant.
type Held = BTreeMap<(String, Variant), u32>;

/// Error of an account store operation.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("the history could not be opened: {0}")]
    Open(#[from] OpenError),
    #[error("the history could not be reached: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("no connection to the history: {0}")]
    Pool(#[from] PoolError),
    #[error("the work did not finish: {0}")]
    Interact(#[from] InteractError),
    #[error("the reading says nowhere whose holdings these are")]
    Nobody,
}

/// Window in seconds within which a matching exchange from another origin counts as the same
/// exchange recorded twice.
const ECHO: Instant = 600;

/// Account store: what the account held and when, and what changed between inventory readings.
pub struct Store {
    pool: Pool,
    /// Locked for the duration of a write: SQLite allows one writer, and the stored state a write
    /// starts from must not change under it.
    writing: Mutex<()>,
}

impl Store {
    /// Opens the account store `account.sqlite` under `root`, creating it when missing.
    pub fn under(root: &Path) -> Result<Self, StoreError> {
        Ok(Self {
            pool: db::open(&root.join("account.sqlite"))?,
            writing: Mutex::new(()),
        })
    }

    /// The last stored inventory with its profile and holdings; `None` when nothing is stored or
    /// the read fails.
    pub async fn restore(&self) -> Option<Inventory> {
        let connection = self.pool.get().await.ok()?;
        connection
            .interact(|connection| taken(connection))
            .await
            .ok()?
    }

    /// Stores an inventory: in full when it is the profile's first snapshot of a UTC day,
    /// otherwise only the changes.
    pub async fn keep(&self, inventory: Arc<Inventory>) -> Result<(), StoreError> {
        self.keep_at(inventory, seconds()).await
    }

    /// Stores the market's full list of the account's own orders.
    pub async fn keep_all_orders(&self, mine: Vec<Answered>) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let seen = seconds();

        connection
            .interact(move |connection| crate::account::keep_all_orders(connection, &mine, seen))
            .await?
    }

    /// Stores one order without marking any other order gone.
    pub async fn keep_one_order(&self, order: Answered) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let seen = seconds();

        connection
            .interact(move |connection| crate::account::keep_one_order(connection, &order, seen))
            .await?
    }

    /// Stores one order the agent placed, with its expected value.
    pub async fn keep_placed_order(
        &self,
        order: Answered,
        promised: Option<u32>,
    ) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let seen = seconds();

        connection
            .interact(move |connection| {
                crate::account::keep_placed_order(connection, &order, promised, seen)
            })
            .await?
    }

    /// Marks one of the account's own orders gone after it is withdrawn from the market.
    pub async fn withdraw_order(&self, id: String) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let at = seconds();

        connection
            .interact(move |connection| {
                crate::account::drop_orders(connection, &[id], at).map(|_| ())
            })
            .await?
    }

    /// Records the quantity left on one of the account's own orders; zero marks it gone.
    pub async fn shrink_order(&self, id: String, left: u32) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let at = seconds();

        connection
            .interact(move |connection| crate::account::shrink_order(connection, &id, left, at))
            .await?
    }

    /// Records one exchange and updates the position on its pair, in one transaction.
    pub async fn record_exchange(&self, made: Made) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let at = seconds();

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                crate::account::record_exchange(&transaction, &made, at)?;
                crate::account::settle_position(&transaction, &made, at)?;
                transaction.commit()?;
                Ok(())
            })
            .await?
    }

    /// Time of the last stored snapshot; `None` when there is none.
    pub async fn last_reading(&self) -> Result<Option<Instant>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(|connection| {
                Ok(
                    connection
                        .query_row("SELECT MAX(taken) FROM snapshot", [], |row| row.get(0))?,
                )
            })
            .await?
    }

    /// Whether this exchange was already recorded from another origin within the last `ECHO`
    /// seconds.
    pub async fn recorded_lately(&self, made: Made) -> Result<bool, StoreError> {
        let connection = self.pool.get().await?;
        let from = seconds() - ECHO;

        connection
            .interact(move |connection| crate::account::exchange_recorded(connection, &made, from))
            .await?
    }

    /// Adds an item to the profile's favourites, or removes it when `on` is false.
    pub async fn set_favourite(
        &self,
        whose: Option<Profile>,
        item: String,
        on: bool,
    ) -> Result<(), StoreError> {
        let whose = whose.ok_or(StoreError::Nobody)?;
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let at = seconds();

        connection
            .interact(move |connection| {
                let owner = owner(connection, &whose)?;
                crate::account::set_favourite(connection, owner, &item, on, at)
            })
            .await?
    }

    /// Every item the profile marked a favourite; empty when the profile is unknown.
    pub async fn favourites(&self, whose: Option<Profile>) -> Result<Vec<String>, StoreError> {
        let Some(whose) = whose else {
            return Ok(Vec::new());
        };
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| match known(connection, &whose)? {
                Some(owner) => crate::account::favourites(connection, owner),
                None => Ok(Vec::new()),
            })
            .await?
    }

    /// Adds an item to the profile's roadmap, or removes it when `on` is false.
    pub async fn set_roadmap(
        &self,
        whose: Option<Profile>,
        item: String,
        on: bool,
    ) -> Result<(), StoreError> {
        let whose = whose.ok_or(StoreError::Nobody)?;
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let at = seconds();

        connection
            .interact(move |connection| {
                let owner = owner(connection, &whose)?;
                crate::account::set_roadmap(connection, owner, &item, on, at)
            })
            .await?
    }

    /// Every item in the profile's roadmap.
    pub async fn roadmap(&self, whose: Option<Profile>) -> Result<Vec<String>, StoreError> {
        let Some(whose) = whose else {
            return Ok(Vec::new());
        };
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| match known(connection, &whose)? {
                Some(owner) => crate::account::roadmap(connection, owner),
                None => Ok(Vec::new()),
            })
            .await?
    }

    /// Removes every item from the profile's roadmap.
    pub async fn clear_roadmap(&self, whose: Option<Profile>) -> Result<(), StoreError> {
        let Some(whose) = whose else {
            return Ok(());
        };
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| match known(connection, &whose)? {
                Some(owner) => crate::account::clear_roadmap(connection, owner),
                None => Ok(()),
            })
            .await?
    }

    /// Records whether a part of an item in the profile's roadmap is to be bought.
    pub async fn set_part_choice(
        &self,
        whose: Option<Profile>,
        choice: Choice,
    ) -> Result<(), StoreError> {
        let whose = whose.ok_or(StoreError::Nobody)?;
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let owner = owner(connection, &whose)?;
                crate::account::set_part_choice(connection, owner, &choice)
            })
            .await?
    }

    /// The choices made for the parts of items in the profile's roadmap.
    pub async fn part_choices(&self, whose: Option<Profile>) -> Result<Vec<Choice>, StoreError> {
        let Some(whose) = whose else {
            return Ok(Vec::new());
        };
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| match known(connection, &whose)? {
                Some(owner) => crate::account::part_choices(connection, owner),
                None => Ok(Vec::new()),
            })
            .await?
    }

    /// Records one in-game trade the game confirmed, with the items given and taken.
    pub async fn keep_swap(&self, swap: median_core::Swap) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                crate::account::record_swap(&transaction, &swap)?;
                transaction.commit()?;
                Ok(())
            })
            .await?
    }

    /// Every in-game trade from `from` on, newest first.
    pub async fn swaps_since(
        &self,
        from: median_core::Instant,
    ) -> Result<Vec<median_core::Swap>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| crate::account::swaps_since(connection, from))
            .await?
    }

    /// Every open position, oldest first.
    pub async fn open_positions(&self) -> Result<Vec<Position>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(|connection| crate::account::open_positions(connection))
            .await?
    }

    /// Records on a position how many ranked-up copies of the item the account holds.
    pub async fn mark_ranked(
        &self,
        slug: String,
        variant: String,
        ranked: u32,
    ) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                crate::account::mark_ranked(connection, &slug, &variant, ranked)
            })
            .await?
    }

    /// Moves copies from the consumed positions into the position of the conversion's result,
    /// carrying over their cost.
    pub async fn convert_positions(&self, conversion: Conversion) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let at = seconds();

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                crate::account::convert_positions(&transaction, &conversion, at)?;
                transaction.commit()?;
                Ok(())
            })
            .await?
    }

    /// Records a riven bought for resale.
    pub async fn keep_riven_buy(&self, riven: RivenBought) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| crate::account::rivens::keep(connection, &riven))
            .await?
    }

    /// Records that the riven bought from `auction` sold for `sale` platinum; returns the number
    /// of rows updated.
    pub async fn sell_riven(&self, auction: String, sale: u32) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let at = seconds();

        connection
            .interact(move |connection| {
                crate::account::rivens::sell(connection, &auction, sale, at)
            })
            .await?
    }

    /// Every riven bought, newest first.
    pub async fn rivens_bought(&self) -> Result<Vec<RivenBought>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(|connection| crate::account::rivens::bought(connection))
            .await?
    }

    /// Total purchase price in platinum of the bought rivens not sold yet.
    pub async fn in_rivens(&self) -> Result<u32, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(|connection| crate::account::rivens::held(connection))
            .await?
    }

    /// Gains of the account since its haul started.
    pub async fn haul(&self, profile: Profile) -> Result<Haul, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| hauled(connection, &profile))
            .await?
    }

    /// Restarts the account's haul at `now`, deleting its recorded gains and claims; later gains
    /// are counted against the last stored inventory.
    pub async fn restart_haul(&self, profile: Profile, now: Instant) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let tx = connection.transaction()?;
                let owner = owner(&tx, &profile)?;
                tx.execute(
                    "INSERT INTO haul_start (profile, started) VALUES (?1, ?2)
                     ON CONFLICT (profile) DO UPDATE SET started = excluded.started",
                    params![owner, now],
                )?;
                tx.execute("DELETE FROM haul_gain WHERE profile = ?1", params![owner])?;
                tx.execute("DELETE FROM haul_claim WHERE profile = ?1", params![owner])?;
                tx.commit()?;
                Ok(())
            })
            .await?
    }

    /// Every exchange from `from` on, newest first.
    pub async fn exchanges_since(&self, from: Instant) -> Result<Vec<Exchange>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| crate::account::exchanges_since(connection, from))
            .await?
    }

    /// Records that warframe.market reports the account in game from `at`, or no longer in game.
    pub async fn mark_presence(&self, in_game: bool, at: Instant) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| match in_game {
                true => crate::account::began_presence(connection, at),
                false => crate::account::ended_presence(connection, at),
            })
            .await?
    }

    /// Closes an in-game interval a previous run left open, at the last event recorded in it.
    pub async fn settle_presence(&self) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(|connection| crate::account::settle_presence(connection))
            .await?
    }

    /// In-game intervals that end at `from` or later; the open interval ends at `now`.
    pub async fn presence_since(
        &self,
        from: Instant,
        now: Instant,
    ) -> Result<Vec<(Instant, Instant)>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| crate::account::presence_since(connection, from, now))
            .await?
    }

    /// Every buy order the agent placed that was still open at `from` or later.
    pub async fn bid_lives(&self, from: Instant) -> Result<Vec<Lived>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| crate::account::bid_lives(connection, from))
            .await?
    }

    /// Trades left for the day in each snapshot of the most recently stored profile from `from`
    /// on, as `(taken, trades_left)`.
    pub async fn exchanges_left_since(
        &self,
        from: Instant,
    ) -> Result<Vec<(Instant, u32)>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT taken, trades_left FROM snapshot
                      WHERE taken >= ?1 AND trades_left IS NOT NULL
                        AND profile = (SELECT profile FROM snapshot ORDER BY id DESC LIMIT 1)
                      ORDER BY taken",
                )?;
                let rows = statement
                    .query_map(params![from], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            })
            .await?
    }

    /// Marks the given orders gone without querying the market; returns the number updated.
    pub async fn drop_orders(&self, ids: Vec<String>) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;
        let at = seconds();

        connection
            .interact(move |connection| crate::account::drop_orders(connection, &ids, at))
            .await?
    }

    /// Assigns an open order to the agent; returns the number of rows updated.
    pub async fn hand_order(&self, id: String) -> Result<usize, StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| crate::account::hand_order(connection, &id))
            .await?
    }

    /// Deletes a position and returns the agent's open sell order on its pair to the user.
    pub async fn release_position(&self, slug: String, variant: String) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| {
                let transaction = connection.transaction()?;
                crate::account::release_position(&transaction, &slug, &variant)?;
                transaction.commit()?;
                Ok(())
            })
            .await?
    }

    /// Every open own order, highest price first.
    pub async fn standing_orders(&self) -> Result<Vec<Standing>, StoreError> {
        let connection = self.pool.get().await?;

        connection
            .interact(|connection| crate::account::standing_orders(connection))
            .await?
    }

    async fn keep_at(&self, inventory: Arc<Inventory>, taken: Instant) -> Result<(), StoreError> {
        let _writing = self.writing.lock().await;
        let connection = self.pool.get().await?;

        connection
            .interact(move |connection| write(connection, &inventory, taken))
            .await?
    }
}

/// The last stored inventory, rebuilt from the snapshots; `None` when nothing is stored or a
/// read fails.
fn taken(connection: &Connection) -> Option<Inventory> {
    let last = connection
        .query_row(
            "SELECT profile.id, profile.created, profile.name,
                         snapshot.platinum, snapshot.credits, snapshot.trades_left,
                         snapshot.endo
               FROM snapshot
               JOIN profile ON profile.id = snapshot.profile
              ORDER BY snapshot.id DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    Profile {
                        created: row.get(1)?,
                        name: row.get(2)?,
                    },
                    Purse {
                        platinum: row.get(3)?,
                        credits: row.get::<_, Option<i64>>(4)?.map(|credits| credits as u64),
                        endo: row.get::<_, Option<i64>>(6)?.map(|endo| endo as u64),
                        trades_left: row.get(5)?,
                    },
                ))
            },
        )
        .optional()
        .ok()
        .flatten();

    let (owner, profile, purse) = last?;
    let until: i64 = connection
        .query_row(
            "SELECT max(id) FROM snapshot WHERE profile = ?1",
            params![owner],
            |row| row.get(0),
        )
        .ok()?;

    Some(Inventory {
        profile: Some(profile),
        holdings: fold(connection, owner, until).ok().flatten()?,
        synced: None,
        purse,
        mastery: mastery(connection, owner),
        equipped: installed(connection, owner),
        progression: progressed(connection, owner),
    })
}

/// Writes one inventory into the account store: a full snapshot when it is the profile's first
/// of a UTC day, otherwise only the changed holdings; `Nobody` when it has no profile.
fn write(
    connection: &mut Connection,
    inventory: &Inventory,
    taken: Instant,
) -> Result<(), StoreError> {
    let Some(profile) = &inventory.profile else {
        return Err(StoreError::Nobody);
    };

    let tx = connection.transaction()?;

    let owner = owner(&tx, profile)?;
    let last = latest(&tx, owner)?;
    let before = match &last {
        Some(kept) => fold(&tx, owner, kept.id)?,
        None => None,
    };
    let held = match &last {
        Some(kept) if taken / DAY == kept.taken / DAY => before.clone(),
        _ => None,
    };
    haul(&tx, owner, before.as_deref(), inventory, taken)?;
    let lines = match &held {
        Some(before) => changes(before, &inventory.holdings),
        None => inventory.holdings.clone(),
    };

    let digest = inventory.digest().bytes();
    tx.execute(
        "INSERT INTO snapshot
           (profile, taken, digest, whole, platinum, credits, trades_left, endo)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            owner,
            taken,
            digest.as_slice(),
            held.is_none(),
            inventory.purse.platinum,
            inventory.purse.credits.map(|credits| credits as i64),
            inventory.purse.trades_left,
            inventory.purse.endo.map(|endo| endo as i64)
        ],
    )?;
    let snapshot = tx.last_insert_rowid();

    {
        let mut intern = tx.prepare("INSERT OR IGNORE INTO item (key) VALUES (?1)")?;
        let mut find = tx.prepare("SELECT id FROM item WHERE key = ?1")?;
        let mut insert = tx.prepare(
            "INSERT INTO holding (snapshot, item, variant, count) VALUES (?1, ?2, ?3, ?4)",
        )?;

        for line in &lines {
            intern.execute(params![line.item])?;
            let item: i64 = find.query_row(params![line.item], |row| row.get(0))?;
            insert.execute(params![snapshot, item, line.variant.0, line.count])?;
        }
    }

    levels(&tx, owner, inventory)?;
    progression(&tx, owner, &inventory.progression)?;

    tx.commit()?;
    Ok(())
}

/// The account's stored haul; empty when the profile is not stored.
fn hauled(connection: &Connection, profile: &Profile) -> Result<Haul, StoreError> {
    let owner: Option<i64> = connection
        .query_row(
            "SELECT id FROM profile WHERE created = ?1",
            params![profile.created],
            |row| row.get(0),
        )
        .optional()?;
    let Some(owner) = owner else {
        return Ok(Haul::default());
    };
    let started: Option<Instant> = connection
        .query_row(
            "SELECT started FROM haul_start WHERE profile = ?1",
            params![owner],
            |row| row.get(0),
        )
        .optional()?;
    let mut statement =
        connection.prepare("SELECT item, variant, count FROM haul_gain WHERE profile = ?1")?;
    let gained = statement
        .query_map(params![owner], |row| {
            Ok((
                (row.get::<_, String>(0)?, Variant(row.get(1)?)),
                row.get::<_, u32>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut statement =
        connection.prepare("SELECT blueprint, count FROM haul_claim WHERE profile = ?1")?;
    let claimed = statement
        .query_map(params![owner], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Haul {
        started,
        gained,
        claimed,
    })
}

/// Adds to the haul the holdings that grew since the previous inventory and the foundry builds
/// claimed since, when a mission was completed in between: changes without a mission are the
/// user's own crafting, refining and upgrading. The first inventory of an account without a
/// haul only sets the haul's start time.
fn haul(
    tx: &Connection,
    owner: i64,
    before: Option<&[Holding]>,
    reading: &Inventory,
    taken: Instant,
) -> Result<(), StoreError> {
    let started: Option<Instant> = tx
        .query_row(
            "SELECT started FROM haul_start WHERE profile = ?1",
            params![owner],
            |row| row.get(0),
        )
        .optional()?;
    if started.is_none() {
        tx.execute(
            "INSERT INTO haul_start (profile, started) VALUES (?1, ?2)",
            params![owner, taken],
        )?;
        return Ok(());
    }
    let Some(before) = before else {
        return Ok(());
    };
    if !reading.progression.missions.is_empty() {
        let finished: i64 = tx.query_row(
            "SELECT coalesce(sum(completes), 0) FROM mission WHERE profile = ?1",
            params![owner],
            |row| row.get(0),
        )?;
        let now: i64 = reading
            .progression
            .missions
            .iter()
            .map(|mission| i64::from(mission.completes))
            .sum();
        if now <= finished {
            return Ok(());
        }
    }
    let mut add = tx.prepare(
        "INSERT INTO haul_gain (profile, item, variant, count) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (profile, item, variant) DO UPDATE SET count = count + excluded.count",
    )?;
    for gained in haul_gains(before, &reading.holdings) {
        add.execute(params![owner, gained.item, gained.variant.0, gained.count])?;
    }
    if let Some(crafting) = &reading.progression.crafting {
        let building: HashSet<&str> = crafting.iter().map(|build| build.id.as_str()).collect();
        let mut stood = tx.prepare("SELECT id, item FROM crafting WHERE profile = ?1")?;
        let left: Vec<String> = stood
            .query_map(params![owner], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter(|(id, _)| !building.contains(id.as_str()))
            .map(|(_, blueprint)| blueprint)
            .collect();
        let mut claim = tx.prepare(
            "INSERT INTO haul_claim (profile, blueprint, count) VALUES (?1, ?2, 1)
             ON CONFLICT (profile, blueprint) DO UPDATE SET count = count + 1",
        )?;
        for blueprint in left {
            claim.execute(params![owner, blueprint])?;
        }
    }
    Ok(())
}

/// Stores the mastery XP of each item, keeping the highest value, and the equipped copies,
/// which replace the stored ones; an empty list leaves the stored rows unchanged.
fn levels(tx: &Connection, owner: i64, inventory: &Inventory) -> Result<(), StoreError> {
    if !inventory.mastery.is_empty() {
        let mut put = tx.prepare(
            "INSERT INTO mastered (profile, item, xp) VALUES (?1, ?2, ?3)
             ON CONFLICT(profile, item) DO UPDATE SET xp = max(xp, excluded.xp)",
        )?;
        for levelled in &inventory.mastery {
            put.execute(params![owner, levelled.item, levelled.xp as i64])?;
        }
    }
    if !inventory.equipped.is_empty() {
        tx.execute("DELETE FROM equipped WHERE profile = ?1", params![owner])?;
        let mut put =
            tx.prepare("INSERT INTO equipped (profile, item, variant) VALUES (?1, ?2, ?3)")?;
        for copy in &inventory.equipped {
            put.execute(params![owner, copy.item, copy.variant.0])?;
        }
    }
    Ok(())
}

/// Stored mastery XP per item; empty when the read fails.
fn mastery(connection: &Connection, owner: i64) -> Vec<Mastered> {
    let Ok(mut statement) = connection.prepare("SELECT item, xp FROM mastered WHERE profile = ?1")
    else {
        return Vec::new();
    };
    let rows = statement.query_map(params![owner], |row| {
        Ok(Mastered {
            item: row.get(0)?,
            xp: row.get::<_, i64>(1)?.max(0) as u64,
        })
    });
    rows.map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

/// Stored equipped copies; empty when the read fails.
fn installed(connection: &Connection, owner: i64) -> Vec<Equipped> {
    let Ok(mut statement) =
        connection.prepare("SELECT item, variant FROM equipped WHERE profile = ?1")
    else {
        return Vec::new();
    };
    let rows = statement.query_map(params![owner], |row| {
        Ok(Equipped {
            item: row.get(0)?,
            variant: Variant(row.get(1)?),
        })
    });
    rows.map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

/// Stores the account's progression. Each part replaces its stored rows; a part missing from
/// the inventory leaves its stored rows unchanged.
fn progression(tx: &Connection, owner: i64, progression: &Progression) -> Result<(), StoreError> {
    if let Some(rank) = progression.rank {
        tx.execute(
            "INSERT OR REPLACE INTO mastery_rank (profile, rank, next_test) VALUES (?1, ?2, ?3)",
            params![owner, rank, progression.next_test],
        )?;
    }
    if !progression.missions.is_empty() {
        tx.execute("DELETE FROM mission WHERE profile = ?1", params![owner])?;
        let mut put = tx.prepare(
            "INSERT OR REPLACE INTO mission (profile, tag, completes, tier)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        for mission in &progression.missions {
            put.execute(params![owner, mission.tag, mission.completes, mission.tier])?;
        }
    }
    if !progression.intrinsics.is_empty() {
        tx.execute("DELETE FROM intrinsic WHERE profile = ?1", params![owner])?;
        let mut put = tx.prepare(
            "INSERT OR REPLACE INTO intrinsic (profile, school, rank) VALUES (?1, ?2, ?3)",
        )?;
        for intrinsic in &progression.intrinsics {
            put.execute(params![owner, intrinsic.school, intrinsic.rank])?;
        }
    }
    if !progression.affiliations.is_empty() {
        tx.execute("DELETE FROM affiliation WHERE profile = ?1", params![owner])?;
        let mut put = tx.prepare(
            "INSERT OR REPLACE INTO affiliation (profile, tag, standing, title)
             VALUES (?1, ?2, ?3, ?4)",
        )?;
        for affiliation in &progression.affiliations {
            put.execute(params![
                owner,
                affiliation.tag,
                affiliation.standing,
                affiliation.title
            ])?;
        }
    }
    if let Some(crafting) = &progression.crafting {
        tx.execute("DELETE FROM crafting WHERE profile = ?1", params![owner])?;
        let mut put = tx.prepare(
            "INSERT OR REPLACE INTO crafting (profile, id, item, ready) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for build in crafting {
            put.execute(params![owner, build.id, build.item, build.ready])?;
        }
    }
    if !progression.acquired.is_empty() {
        tx.execute("DELETE FROM acquired WHERE profile = ?1", params![owner])?;
        let mut put =
            tx.prepare("INSERT OR REPLACE INTO acquired (profile, item, at) VALUES (?1, ?2, ?3)")?;
        for acquired in &progression.acquired {
            put.execute(params![owner, acquired.item, acquired.at])?;
        }
        tx.execute("DELETE FROM formed WHERE profile = ?1", params![owner])?;
        let mut put =
            tx.prepare("INSERT OR REPLACE INTO formed (profile, item, forma) VALUES (?1, ?2, ?3)")?;
        for formed in &progression.formed {
            put.execute(params![owner, formed.item, formed.forma])?;
        }
    }
    Ok(())
}

/// Stored progression of the account; a part that fails to read is empty.
fn progressed(connection: &Connection, owner: i64) -> Progression {
    let ranked = connection
        .query_row(
            "SELECT rank, next_test FROM mastery_rank WHERE profile = ?1",
            params![owner],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, Option<Instant>>(1)?)),
        )
        .optional()
        .ok()
        .flatten();

    Progression {
        rank: ranked.map(|(rank, _)| rank),
        next_test: ranked.and_then(|(_, next_test)| next_test),
        missions: listed(
            connection,
            owner,
            "SELECT tag, completes, tier FROM mission WHERE profile = ?1 ORDER BY tag",
            |row| {
                Ok(Mission {
                    tag: row.get(0)?,
                    completes: row.get(1)?,
                    tier: row.get(2)?,
                })
            },
        ),
        intrinsics: listed(
            connection,
            owner,
            "SELECT school, rank FROM intrinsic WHERE profile = ?1 ORDER BY school",
            |row| {
                Ok(Intrinsic {
                    school: row.get(0)?,
                    rank: row.get(1)?,
                })
            },
        ),
        crafting: Some(listed(
            connection,
            owner,
            "SELECT id, item, ready FROM crafting WHERE profile = ?1 ORDER BY ready",
            |row| {
                Ok(Crafting {
                    id: row.get(0)?,
                    item: row.get(1)?,
                    ready: row.get(2)?,
                })
            },
        )),
        acquired: listed(
            connection,
            owner,
            "SELECT item, at FROM acquired WHERE profile = ?1 ORDER BY item",
            |row| {
                Ok(Acquired {
                    item: row.get(0)?,
                    at: row.get(1)?,
                })
            },
        ),
        formed: listed(
            connection,
            owner,
            "SELECT item, forma FROM formed WHERE profile = ?1 ORDER BY item",
            |row| {
                Ok(Formed {
                    item: row.get(0)?,
                    forma: row.get(1)?,
                })
            },
        ),
        affiliations: listed(
            connection,
            owner,
            "SELECT tag, standing, title FROM affiliation WHERE profile = ?1 ORDER BY tag",
            |row| {
                Ok(Affiliation {
                    tag: row.get(0)?,
                    standing: row.get(1)?,
                    title: row.get(2)?,
                })
            },
        ),
        allowances: Vec::new(),
    }
}

/// Every row `sql` returns for the profile; empty when the query fails, and rows that fail to
/// read are skipped.
fn listed<T>(
    connection: &Connection,
    owner: i64,
    sql: &str,
    row: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Vec<T> {
    let Ok(mut statement) = connection.prepare(sql) else {
        return Vec::new();
    };
    statement
        .query_map(params![owner], row)
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}

/// Id and time of one stored snapshot.
struct Kept {
    id: i64,
    taken: Instant,
}

/// Id of the profile's row, inserted or renamed as needed.
fn owner(tx: &Connection, profile: &Profile) -> Result<i64, rusqlite::Error> {
    tx.execute(
        "INSERT INTO profile (created, name) VALUES (?1, ?2)
         ON CONFLICT (created) DO UPDATE SET name = excluded.name",
        params![profile.created, profile.name],
    )?;
    tx.query_row(
        "SELECT id FROM profile WHERE created = ?1",
        params![profile.created],
        |row| row.get(0),
    )
}

/// Id of the profile's row; `None` when the profile is not stored.
fn known(connection: &Connection, profile: &Profile) -> Result<Option<i64>, rusqlite::Error> {
    connection
        .query_row(
            "SELECT id FROM profile WHERE created = ?1",
            params![profile.created],
            |row| row.get(0),
        )
        .optional()
}

/// Latest snapshot of the profile; `None` when it has none.
fn latest(tx: &Connection, owner: i64) -> Result<Option<Kept>, rusqlite::Error> {
    tx.query_row(
        "SELECT id, taken FROM snapshot WHERE profile = ?1 ORDER BY id DESC LIMIT 1",
        params![owner],
        |row| {
            Ok(Kept {
                id: row.get(0)?,
                taken: row.get(1)?,
            })
        },
    )
    .optional()
}

/// Holdings of the profile as of snapshot `until`, rebuilt from the latest full snapshot at or
/// before it plus the changes after it; `None` when there is no such full snapshot.
fn fold(tx: &Connection, owner: i64, until: i64) -> Result<Option<Vec<Holding>>, rusqlite::Error> {
    let base: Option<i64> = tx
        .query_row(
            "SELECT id FROM snapshot
              WHERE profile = ?1 AND whole = 1 AND id <= ?2
              ORDER BY id DESC LIMIT 1",
            params![owner, until],
            |row| row.get(0),
        )
        .optional()?;
    let Some(base) = base else {
        return Ok(None);
    };

    let mut statement = tx.prepare(
        "SELECT item.key, holding.variant, holding.count
           FROM holding
           JOIN snapshot ON snapshot.id = holding.snapshot
           JOIN item ON item.id = holding.item
          WHERE snapshot.profile = ?1 AND holding.snapshot BETWEEN ?2 AND ?3
          ORDER BY holding.snapshot",
    )?;
    let rows = statement.query_map(params![owner, base, until], |row| {
        Ok((
            row.get::<_, String>(0)?,
            Variant(row.get::<_, String>(1)?),
            row.get::<_, u32>(2)?,
        ))
    })?;

    let mut held = Held::new();
    for row in rows {
        let (item, variant, count) = row?;
        match count {
            0 => held.remove(&(item, variant)),
            _ => held.insert((item, variant), count),
        };
    }

    Ok(Some(
        held.into_iter()
            .map(|((item, variant), count)| Holding {
                detail: detail(&variant),
                item,
                variant,
                count,
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use median_core::Detail;

    use super::*;

    /// Separate directory per test, so tests running in parallel do not share an account store.
    fn place() -> PathBuf {
        static MADE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "median-account-{}-{}",
            std::process::id(),
            MADE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).expect("a place for the history");
        root
    }

    #[tokio::test]
    async fn a_riven_bought_twice_over_is_counted_once_and_leaves_once_sold() {
        let store = Store::under(&place()).expect("an open history");
        let buy = |auction: &str, price| {
            store.keep_riven_buy(RivenBought {
                auction: auction.into(),
                weapon: "/Lotus/Weapons/ClanTech/Bio/BioWeapon".into(),
                name: "crita-visican".into(),
                riven: "{}".into(),
                rank: 0,
                price,
                bought: 1_787_400_000,
                sold: None,
                sale: None,
            })
        };

        buy("a", 300).await.expect("kept");
        buy("a", 300).await.expect("kept");
        buy("b", 150).await.expect("kept");
        assert_eq!(store.in_rivens().await.expect("read"), 450);

        store.sell_riven("a".into(), 420).await.expect("sold");

        assert_eq!(store.in_rivens().await.expect("read"), 150);
        let bought = store.rivens_bought().await.expect("read");
        let sold = bought.iter().find(|riven| riven.auction == "a");
        assert_eq!(sold.and_then(|riven| riven.sale), Some(420));
    }

    #[tokio::test]
    async fn an_exchange_marked_by_hand_after_the_game_confirmed_it_is_known() {
        let store = Store::under(&place()).expect("an open history");
        let made = Made {
            item: "/Lotus/Upgrades/Mods/Pistol/Expert/WeaponFireDamageModExpert".into(),
            slug: "primed_heated_charge".into(),
            variant: "mod_rank=10".into(),
            selling: false,
            platinum: 74,
            quantity: 1,
            origin: "stream".into(),
            own: Some("6ac827af".into()),
        };
        assert!(!store.recorded_lately(made.clone()).await.expect("read"));

        store.record_exchange(made.clone()).await.expect("written");

        let by_hand = Made {
            origin: "hand".into(),
            ..made.clone()
        };
        assert!(store.recorded_lately(by_hand.clone()).await.expect("read"));
        let dearer = Made {
            platinum: 80,
            ..by_hand.clone()
        };
        assert!(!store.recorded_lately(dearer).await.expect("read"));

        // Another exchange from the `stream` origin is a new exchange, not a duplicate.
        assert!(!store.recorded_lately(made.clone()).await.expect("read"));
        store
            .record_exchange(by_hand.clone())
            .await
            .expect("written");
        assert!(!store.recorded_lately(by_hand).await.expect("read"));
    }

    fn tenno(created: i64) -> Option<Profile> {
        Some(Profile {
            created,
            name: "Tenno".into(),
        })
    }

    fn reading(created: i64, holdings: &[(&str, u32)]) -> Arc<Inventory> {
        Arc::new(Inventory {
            profile: Some(Profile {
                created,
                name: "Tenno".into(),
            }),
            holdings: holdings
                .iter()
                .map(|(item, count)| Holding {
                    item: (*item).into(),
                    variant: Variant::default(),
                    detail: None,
                    count: *count,
                })
                .collect(),
            synced: None,
            purse: Purse::default(),
            ..Inventory::default()
        })
    }

    /// Runs one read on a connection of the account store, for test assertions.
    async fn ask<T: Send + 'static>(
        store: &Store,
        read: impl FnOnce(&Connection) -> T + Send + 'static,
    ) -> T {
        let connection = store.pool.get().await.expect("a connection");
        connection
            .interact(move |connection| read(connection))
            .await
            .expect("a finished read")
    }

    /// Holdings of the profile as rebuilt from the stored snapshots.
    async fn folded(store: &Store, created: i64) -> Vec<Holding> {
        ask(store, move |connection| {
            let owner: i64 = connection
                .query_row(
                    "SELECT id FROM profile WHERE created = ?1",
                    params![created],
                    |row| row.get(0),
                )
                .expect("a profile");
            let until: i64 = connection
                .query_row(
                    "SELECT max(id) FROM snapshot WHERE profile = ?1",
                    params![owner],
                    |row| row.get(0),
                )
                .expect("a snapshot");

            fold(connection, owner, until)
                .expect("a fold")
                .expect("a whole snapshot beneath it")
        })
        .await
    }

    async fn snapshots(store: &Store) -> Vec<bool> {
        ask(store, |connection| {
            let mut statement = connection
                .prepare("SELECT whole FROM snapshot ORDER BY id")
                .expect("a query");
            let rows = statement
                .query_map([], |row| row.get::<_, bool>(0))
                .expect("the snapshots");
            rows.map(|row| row.expect("a snapshot")).collect()
        })
        .await
    }

    async fn count(store: &Store, table: &str) -> i64 {
        let table = table.to_string();
        ask(store, move |connection| {
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("a count")
        })
        .await
    }

    #[tokio::test]
    async fn a_reading_that_says_nobody_is_refused() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let orphan = Arc::new(Inventory {
            profile: None,
            holdings: Vec::new(),
            synced: None,
            purse: Purse::default(),
            ..Inventory::default()
        });

        let kept = store.keep_at(orphan, 0).await;

        assert!(matches!(kept, Err(StoreError::Nobody)));
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn an_item_the_game_stopped_naming_keeps_its_mastery() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let levelled = |items: &[(&str, u64)]| {
            Arc::new(Inventory {
                profile: Some(Profile {
                    created: 1,
                    name: "Tenno".into(),
                }),
                mastery: items
                    .iter()
                    .map(|(item, xp)| Mastered {
                        item: (*item).into(),
                        xp: *xp,
                    })
                    .collect(),
                ..Inventory::default()
            })
        };

        store
            .keep_at(levelled(&[("Grendel", 900_000), ("Baza", 450_000)]), 0)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(levelled(&[("Baza", 600_000)]), 60)
            .await
            .expect("a kept snapshot");

        let kept = ask(&store, |connection| mastery(connection, 1)).await;
        let xp = |item: &str| {
            kept.iter()
                .find(|levelled| levelled.item == item)
                .map(|levelled| levelled.xp)
        };
        assert_eq!(xp("Grendel"), Some(900_000));
        assert_eq!(xp("Baza"), Some(600_000));
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_first_reading_of_a_profile_is_kept_whole() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        store
            .keep_at(reading(1, &[("Ferrite", 12), ("Rubedo", 24928)]), 0)
            .await
            .expect("a kept snapshot");

        assert_eq!(snapshots(&store).await, vec![true]);
        assert_eq!(count(&store, "holding").await, 2);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_reading_the_same_day_writes_only_what_moved() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        store
            .keep_at(reading(1, &[("Ferrite", 12), ("Rubedo", 24928)]), 0)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(reading(1, &[("Ferrite", 12), ("Rubedo", 24929)]), 60)
            .await
            .expect("a kept snapshot");

        assert_eq!(snapshots(&store).await, vec![true, false]);
        assert_eq!(count(&store, "holding").await, 3);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_chain_of_changes_folds_back_to_what_was_read() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let last = reading(1, &[("Ferrite", 12), ("Nitain", 3)]);

        store
            .keep_at(reading(1, &[("Ferrite", 9), ("Rubedo", 24928)]), 0)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(reading(1, &[("Ferrite", 12), ("Rubedo", 24928)]), 60)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(Arc::clone(&last), 120)
            .await
            .expect("a kept snapshot");

        assert_eq!(folded(&store, 1).await, last.holdings);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn what_vanished_is_written_as_none_left() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        store
            .keep_at(reading(1, &[("Ferrite", 12), ("Rubedo", 1)]), 0)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(reading(1, &[("Ferrite", 12)]), 60)
            .await
            .expect("a kept snapshot");

        assert_eq!(count(&store, "holding").await, 3);
        assert_eq!(folded(&store, 1).await.len(), 1);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_reading_the_next_day_opens_a_whole_snapshot_again() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        store
            .keep_at(reading(1, &[("Ferrite", 12)]), 0)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(reading(1, &[("Ferrite", 13)]), DAY)
            .await
            .expect("a kept snapshot");

        assert_eq!(snapshots(&store).await, vec![true, true]);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn an_item_is_named_once_however_many_snapshots_hold_it() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        for (at, held) in [(0, 1), (60, 2), (120, 3)] {
            store
                .keep_at(reading(1, &[("Ferrite", held)]), at)
                .await
                .expect("a kept snapshot");
        }

        assert_eq!(count(&store, "item").await, 1);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn what_tells_copies_apart_reaches_the_history() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let riven = Arc::new(Inventory {
            profile: Some(Profile {
                created: 1,
                name: "Tenno".into(),
            }),
            holdings: vec![
                Holding {
                    item: "/Lotus/Upgrades/Mods/Randomized/Rifle".into(),
                    variant: Variant("rolled once".into()),
                    detail: None,
                    count: 1,
                },
                Holding {
                    item: "/Lotus/Upgrades/Mods/Randomized/Rifle".into(),
                    variant: Variant("rolled twice".into()),
                    detail: None,
                    count: 1,
                },
            ],
            synced: None,
            purse: Purse::default(),
            ..Inventory::default()
        });

        store
            .keep_at(Arc::clone(&riven), 0)
            .await
            .expect("a kept snapshot");

        assert_eq!(folded(&store, 1).await, riven.holdings);
        assert_eq!(count(&store, "item").await, 1);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn what_a_variant_says_is_read_back_out_of_the_history() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let ranked = Arc::new(Inventory {
            profile: Some(Profile {
                created: 1,
                name: "Tenno".into(),
            }),
            holdings: vec![Holding {
                item: "/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod".into(),
                variant: Variant(r#"{"lvl":5}"#.into()),
                detail: Some(Detail::Rank(5)),
                count: 1,
            }],
            synced: None,
            purse: Purse::default(),
            ..Inventory::default()
        });

        store
            .keep_at(Arc::clone(&ranked), 0)
            .await
            .expect("a kept snapshot");

        assert_eq!(folded(&store, 1).await, ranked.holdings);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn two_accounts_keep_their_own_history() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        store
            .keep_at(reading(1, &[("Ferrite", 12)]), 0)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(reading(2, &[("Rubedo", 5)]), 60)
            .await
            .expect("a kept snapshot");

        assert_eq!(snapshots(&store).await, vec![true, true]);
        assert_eq!(folded(&store, 1).await.len(), 1);
        assert_eq!(folded(&store, 2).await[0].item, "Rubedo");
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn one_account_changing_does_not_reach_the_other() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        store
            .keep_at(reading(1, &[("Ferrite", 12)]), 0)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(reading(2, &[("Rubedo", 5)]), 60)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(reading(1, &[("Ferrite", 13)]), 120)
            .await
            .expect("a kept snapshot");

        assert_eq!(
            folded(&store, 1).await,
            reading(1, &[("Ferrite", 13)]).holdings
        );
        assert_eq!(
            folded(&store, 2).await,
            reading(2, &[("Rubedo", 5)]).holdings
        );
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_history_gives_back_what_it_last_took_across_a_reopening() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let kept = reading(1, &[("Ferrite", 12), ("Rubedo", 24928)]);
        store
            .keep_at(Arc::clone(&kept), 0)
            .await
            .expect("a kept snapshot");
        drop(store);

        let reopened = Store::under(&root).expect("an open history");

        let restored = reopened.restore().await.expect("a reading");
        assert_eq!(
            restored.profile.as_ref().map(|profile| profile.created),
            Some(1)
        );
        assert_eq!(restored.holdings, kept.holdings);
        assert_eq!(restored.digest(), kept.digest());
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_history_that_has_taken_nothing_says_so() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        assert!(store.restore().await.is_none());
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_account_that_moved_last_is_the_one_that_is_given_back() {
        let root = place();
        let store = Store::under(&root).expect("an open history");

        store
            .keep_at(reading(1, &[("Ferrite", 12)]), 0)
            .await
            .expect("a kept snapshot");
        store
            .keep_at(reading(2, &[("Rubedo", 5)]), 60)
            .await
            .expect("a kept snapshot");

        let restored = store.restore().await.expect("a reading");
        assert_eq!(
            restored.profile.as_ref().map(|profile| profile.created),
            Some(2)
        );
        assert_eq!(restored.holdings, reading(2, &[("Rubedo", 5)]).holdings);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_writes_at_once_both_reach_the_history() {
        let root = place();
        let store = Arc::new(Store::under(&root).expect("an open history"));
        let other = Arc::clone(&store);

        let first =
            tokio::spawn(async move { store.keep_at(reading(1, &[("Ferrite", 12)]), 0).await });
        let second =
            tokio::spawn(async move { other.keep_at(reading(1, &[("Ferrite", 13)]), 60).await });

        first
            .await
            .expect("a finished write")
            .expect("a kept snapshot");
        second
            .await
            .expect("a finished write")
            .expect("a kept snapshot");
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn what_was_levelled_and_what_is_installed_come_back_with_the_history() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let kept = Arc::new(Inventory {
            profile: Some(Profile {
                created: 1,
                name: "Tenno".into(),
            }),
            holdings: vec![Holding {
                item: "/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod".into(),
                variant: Variant(r#"{"lvl":5}"#.into()),
                detail: Some(Detail::Rank(5)),
                count: 1,
            }],
            synced: None,
            purse: Purse::default(),
            mastery: vec![Mastered {
                item: "/Lotus/Weapons/Tenno/Rifle/BurstonPrime".into(),
                xp: 450_000,
            }],
            equipped: vec![Equipped {
                item: "/Lotus/Upgrades/Mods/Warframe/AvatarShieldMaxMod".into(),
                variant: Variant(r#"{"lvl":5}"#.into()),
            }],
            progression: Progression::default(),
        });
        store.keep_at(Arc::clone(&kept), 0).await.expect("kept");

        let back = Store::under(&root)
            .expect("an open history")
            .restore()
            .await
            .expect("a reading");

        assert_eq!(back.mastery, kept.mastery);
        assert_eq!(back.equipped, kept.equipped);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_favourite_stays_marked_until_the_mark_is_taken_off() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let gara = "/Lotus/Powersuits/Glass/GlassPrime".to_string();
        let vitality = "/Lotus/Upgrades/Mods/Warframe/AvatarHealthMaxMod".to_string();

        store
            .set_favourite(tenno(1), gara.clone(), true)
            .await
            .expect("marked");
        store
            .set_favourite(tenno(1), gara.clone(), true)
            .await
            .expect("marked again");
        store
            .set_favourite(tenno(1), vitality.clone(), true)
            .await
            .expect("marked");
        assert_eq!(
            store.favourites(tenno(1)).await.expect("favourites"),
            vec![gara.clone(), vitality.clone()]
        );

        store
            .set_favourite(tenno(1), gara, false)
            .await
            .expect("unmarked");
        assert_eq!(
            store.favourites(tenno(1)).await.expect("favourites"),
            vec![vitality]
        );
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn taking_an_item_out_of_the_roadmap_forgets_what_was_chosen_for_its_parts() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let gara = "/Lotus/Powersuits/Glass/GlassPrime".to_string();
        let choice = Choice {
            item: gara.clone(),
            part: "/Lotus/Types/Recipes/WarframeRecipes/GlassPrimeHelmetBlueprint".into(),
            buy: true,
        };

        store
            .set_roadmap(tenno(1), gara.clone(), true)
            .await
            .expect("put in");
        store
            .set_part_choice(tenno(1), choice.clone())
            .await
            .expect("chosen");
        assert_eq!(
            store.part_choices(tenno(1)).await.expect("choices"),
            vec![choice]
        );

        store
            .set_roadmap(tenno(1), gara, false)
            .await
            .expect("taken out");
        assert!(store.roadmap(tenno(1)).await.expect("roadmap").is_empty());
        assert!(
            store
                .part_choices(tenno(1))
                .await
                .expect("choices")
                .is_empty()
        );
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn clearing_the_roadmap_takes_out_every_item_and_its_choices() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let gara = "/Lotus/Powersuits/Glass/GlassPrime".to_string();
        let volt = "/Lotus/Powersuits/Volt/VoltPrime".to_string();

        store
            .set_roadmap(tenno(1), gara.clone(), true)
            .await
            .expect("put in");
        store
            .set_roadmap(tenno(1), volt, true)
            .await
            .expect("put in");
        store
            .set_part_choice(
                tenno(1),
                Choice {
                    item: gara,
                    part: "/Lotus/Types/Recipes/WarframeRecipes/GlassPrimeHelmetBlueprint".into(),
                    buy: true,
                },
            )
            .await
            .expect("chosen");

        store.clear_roadmap(tenno(1)).await.expect("cleared");

        assert!(store.roadmap(tenno(1)).await.expect("roadmap").is_empty());
        assert!(
            store
                .part_choices(tenno(1))
                .await
                .expect("choices")
                .is_empty()
        );
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn each_account_keeps_its_own_marks() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let gara = "/Lotus/Powersuits/Glass/GlassPrime".to_string();
        let volt = "/Lotus/Powersuits/Volt/VoltPrime".to_string();

        store
            .set_favourite(tenno(1), gara.clone(), true)
            .await
            .expect("marked");
        store
            .set_roadmap(tenno(1), gara.clone(), true)
            .await
            .expect("put in");
        store
            .set_roadmap(tenno(2), volt.clone(), true)
            .await
            .expect("put in");
        store.clear_roadmap(tenno(2)).await.expect("cleared");

        assert_eq!(
            store.favourites(tenno(1)).await.expect("favourites"),
            vec![gara.clone()]
        );
        assert!(
            store
                .favourites(tenno(2))
                .await
                .expect("favourites")
                .is_empty()
        );
        assert_eq!(store.roadmap(tenno(1)).await.expect("roadmap"), vec![gara]);
        assert!(store.roadmap(tenno(2)).await.expect("roadmap").is_empty());
        assert!(store.roadmap(tenno(3)).await.expect("roadmap").is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn one_account_levelling_does_not_reach_the_other() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let levelled = |created: i64, item: &str, xp: u64, rank: u32| {
            Arc::new(Inventory {
                profile: tenno(created),
                holdings: vec![Holding {
                    item: "/Lotus/Types/Items/MiscItems/Rubedo".into(),
                    variant: Variant::default(),
                    detail: None,
                    count: u32::try_from(created).expect("a small count"),
                }],
                mastery: vec![Mastered {
                    item: item.into(),
                    xp,
                }],
                equipped: vec![Equipped {
                    item: item.into(),
                    variant: Variant::default(),
                }],
                progression: Progression {
                    rank: Some(rank),
                    crafting: Some(vec![Crafting {
                        id: format!("{created}"),
                        item: item.into(),
                        ready: 0,
                    }]),
                    ..Progression::default()
                },
                ..Inventory::default()
            })
        };
        let main = levelled(1, "/Lotus/Weapons/Tenno/Rifle/BurstonPrime", 450_000, 18);
        let other = levelled(2, "/Lotus/Weapons/Tenno/Pistol/Lato", 900_000, 3);

        store.keep_at(Arc::clone(&main), 0).await.expect("kept");
        store.keep_at(Arc::clone(&other), 60).await.expect("kept");
        store.keep_at(Arc::clone(&main), 120).await.expect("kept");

        let back = Store::under(&root)
            .expect("an open history")
            .restore()
            .await
            .expect("a reading");

        assert_eq!(back.mastery, main.mastery);
        assert_eq!(back.equipped, main.equipped);
        assert_eq!(back.progression, main.progression);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn how_far_the_account_had_come_outlives_a_reading_silent_on_it() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let progression = Progression {
            rank: Some(18),
            next_test: Some(1_788_871_622),
            missions: vec![
                Mission {
                    tag: "EarthToVenusJunction".into(),
                    completes: 1,
                    tier: None,
                },
                Mission {
                    tag: "SolNode27".into(),
                    completes: 49,
                    tier: Some(1),
                },
            ],
            intrinsics: vec![Intrinsic {
                school: "LPS_PILOTING".into(),
                rank: 6,
            }],
            crafting: Some(vec![Crafting {
                id: "66e0a1b2c3d4e5f6a7b8c9d0".into(),
                item: "/Lotus/Types/Recipes/Weapons/PolearmPrimeBlueprint".into(),
                ready: 1_788_358_770,
            }]),
            acquired: vec![Acquired {
                item: "/Lotus/Weapons/Tenno/Rifle/Braton".into(),
                at: 1_605_217_453,
            }],
            formed: vec![Formed {
                item: "/Lotus/Weapons/Grineer/KuvaLich/Secondaries/Nukor/KuvaNukor".into(),
                forma: 5,
            }],
            affiliations: vec![Affiliation {
                tag: "RedVeilSyndicate".into(),
                standing: 298_500,
                title: 5,
            }],
            allowances: Vec::new(),
        };
        let reading = |progression: Progression| {
            Arc::new(Inventory {
                profile: Some(Profile {
                    created: 1,
                    name: "Tenno".into(),
                }),
                holdings: vec![Holding {
                    item: "/Lotus/Types/Items/MiscItems/Rubedo".into(),
                    variant: Variant::default(),
                    detail: None,
                    count: 1,
                }],
                progression,
                ..Inventory::default()
            })
        };
        store
            .keep_at(reading(progression.clone()), 0)
            .await
            .expect("kept");
        store
            .keep_at(reading(Progression::default()), 1)
            .await
            .expect("kept");

        let back = Store::under(&root)
            .expect("an open history")
            .restore()
            .await
            .expect("a reading");

        assert_eq!(back.progression, progression);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_purse_comes_back_with_the_history() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let kept = Arc::new(Inventory {
            profile: Some(Profile {
                created: 1,
                name: "Tenno".into(),
            }),
            holdings: vec![Holding {
                item: "/Lotus/Types/Items/MiscItems/Rubedo".into(),
                variant: Variant::default(),
                detail: None,
                count: 1,
            }],
            synced: None,
            purse: Purse {
                platinum: Some(340),
                credits: Some(11_737_725),
                endo: Some(68_015),
                trades_left: Some(14),
            },
            ..Inventory::default()
        });
        store.keep_at(Arc::clone(&kept), 0).await.expect("kept");

        let back = Store::under(&root)
            .expect("an open history")
            .restore()
            .await
            .expect("a reading");

        assert_eq!(back.purse, kept.purse);
        assert_eq!(back.digest(), kept.digest());
    }

    #[tokio::test]
    async fn the_haul_counts_what_each_reading_gained_and_outlives_a_restart() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let tenno = Profile {
            created: 1,
            name: "Tenno".into(),
        };

        store
            .keep_at(reading(1, &[("/Part", 1), ("/Relic", 5)]), 0)
            .await
            .expect("kept");
        store
            .keep_at(reading(1, &[("/Part", 3), ("/Relic", 4)]), 60)
            .await
            .expect("kept");
        store
            .keep_at(reading(1, &[("/Part", 4), ("/Mod", 1)]), 120)
            .await
            .expect("kept");

        let reopened = Store::under(&root).expect("the history opens again");
        let haul = reopened.haul(tenno.clone()).await.expect("a haul");
        let gained = |item: &str| {
            haul.gained
                .get(&(item.to_owned(), Variant::default()))
                .copied()
        };
        assert_eq!(haul.started, Some(0));
        assert_eq!(gained("/Part"), Some(3));
        assert_eq!(gained("/Mod"), Some(1));
        assert_eq!(gained("/Relic"), None);

        reopened
            .restart_haul(tenno.clone(), 200)
            .await
            .expect("restarted");
        let haul = reopened.haul(tenno).await.expect("a haul");
        assert_eq!(haul.started, Some(200));
        assert!(haul.gained.is_empty());
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_haul_leaves_out_what_moved_without_a_mission() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let after = |completes: u32, holdings: &[(&str, u32)]| {
            let mut held = (*reading(1, holdings)).clone();
            held.progression.missions = vec![Mission {
                tag: "SolNode27".into(),
                completes,
                tier: None,
            }];
            Arc::new(held)
        };

        store
            .keep_at(after(4, &[("/Ore", 1)]), 0)
            .await
            .expect("kept");
        store
            .keep_at(after(4, &[("/Ore", 1), ("/Radiant", 1)]), 60)
            .await
            .expect("kept");
        store
            .keep_at(after(5, &[("/Ore", 1), ("/Radiant", 1), ("/Part", 2)]), 120)
            .await
            .expect("kept");

        let haul = store
            .haul(Profile {
                created: 1,
                name: "Tenno".into(),
            })
            .await
            .expect("a haul");
        assert_eq!(
            haul.gained.into_iter().collect::<Vec<_>>(),
            vec![(("/Part".to_owned(), Variant::default()), 2)]
        );
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_haul_notes_each_build_that_left_the_foundry() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let building = |builds: &[(&str, &str)]| {
            let mut held = (*reading(1, &[("/Part", 1)])).clone();
            held.progression.crafting = Some(
                builds
                    .iter()
                    .map(|(id, blueprint)| Crafting {
                        id: (*id).into(),
                        item: (*blueprint).into(),
                        ready: 0,
                    })
                    .collect(),
            );
            Arc::new(held)
        };

        store.keep_at(building(&[]), 0).await.expect("kept");
        store
            .keep_at(
                building(&[("a", "/GunBlueprint"), ("b", "/FormaBlueprint")]),
                60,
            )
            .await
            .expect("kept");
        store
            .keep_at(building(&[("b", "/FormaBlueprint")]), 120)
            .await
            .expect("kept");

        let haul = store
            .haul(Profile {
                created: 1,
                name: "Tenno".into(),
            })
            .await
            .expect("a haul");
        assert_eq!(
            haul.claimed.into_iter().collect::<Vec<_>>(),
            vec![("/GunBlueprint".to_owned(), 1)]
        );
        fs::remove_dir_all(&root).ok();
    }

    fn answered(id: &str) -> crate::account::Answered {
        let mine = median_market::Mine {
            id: id.into(),
            side: median_market::Side::Sell,
            platinum: 20,
            quantity: 1,
            per_trade: None,
            visible: true,
            rank: None,
            subtype: None,
            amber_stars: None,
            cyan_stars: None,
            item_id: Some(format!("{id}-item")),
            created_at: "2026-10-07T18:00:00Z".into(),
            updated_at: "2026-10-07T18:00:00Z".into(),
        };
        (mine, Some(format!("{id}_slug")), String::new())
    }

    #[tokio::test]
    async fn an_order_written_a_moment_ago_outlives_the_markets_silence() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let connection = store.pool.get().await.expect("a connection");
        connection
            .interact(|connection| {
                crate::account::keep_one_order(connection, &answered("fresh"), 1_000)?;
                crate::account::keep_one_order(connection, &answered("stale"), 500)?;
                crate::account::keep_all_orders(connection, &[], 1_060)
            })
            .await
            .expect("the history answered")
            .expect("kept");

        let standing: Vec<String> = store
            .standing_orders()
            .await
            .expect("the orders")
            .into_iter()
            .map(|order| order.id)
            .collect();
        assert_eq!(standing, ["fresh"]);
        fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn every_price_an_order_went_through_is_written_down() {
        let root = place();
        let store = Store::under(&root).expect("an open history");
        let connection = store.pool.get().await.expect("a connection");
        connection
            .interact(|connection| {
                let mut order = answered("bid");
                crate::account::keep_placed_order(connection, &order, Some(10), 1_000)?;
                crate::account::keep_one_order(connection, &order, 1_030)?;
                order.0.platinum = 24;
                crate::account::keep_one_order(connection, &order, 1_060)?;
                crate::account::shrink_order(connection, "bid", 0, 1_090)
            })
            .await
            .expect("the history answered")
            .expect("kept");

        let steps = ask(&store, |connection| {
            let mut statement = connection
                .prepare("SELECT at, platinum, step FROM order_step WHERE own = 'bid' ORDER BY at")
                .expect("a statement");
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })
                .expect("the steps")
                .collect::<rusqlite::Result<Vec<_>>>()
                .expect("the steps")
        })
        .await;
        assert_eq!(
            steps,
            [
                (1_000, 20, "placed".to_owned()),
                (1_060, 24, "moved".to_owned()),
                (1_090, 24, "gone".to_owned()),
            ]
        );
        fs::remove_dir_all(&root).ok();
    }
}
