use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::sync::PoisonError;
use std::time::Instant;

use median_core::{
    Accent, AgentEvent, CatalogEvent, Event, Holder, Interest, Locale, MembershipEvent, Need,
    Origin, Status,
};
use median_engine::Input;
use median_ipc::{
    AppState, Build, Card, Coin, Collectible, Curve, Figures, GameData, Good, Haul, Holdings, Icon,
    MasteryRank, Node, Orders, OwnAuction, OwnOrder, Pair, Posting, Priced, Quote, Release,
    ReleaseChannel, Relic, RerollOdds, Riven, RivenAppraisal, RivenBand, RivenBuy, RivenChoices,
    RivenDemand, Roadmap, Scorecard, SetRow, Syndicate, Taxonomy, Tree,
};
use median_market::{AuctionChange, MarketError, Mine, Placing, Session, Side};
use tauri::AppHandle;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info, warn};

use crate::account;
use crate::agent;
use crate::catalog::{
    Catalog, builds, card, coins, game_data, goods, icons, nodes, search, sets, taxonomy,
};
use crate::figures;
use crate::gaps::Gaps;
use crate::market::stream::{Ask, Stream};
use crate::market::{self, Priority, Token};
use crate::relics;
use crate::revision;
use crate::rivens;
use crate::state_sink::{
    SharedHoldings, SharedState, from_alert, from_policy, from_release_channel, from_watch,
    to_holdings, to_release_channel,
};
use crate::update;
use crate::way::MasteryCache;
use crate::window;

/// Sender half of the engine's input channel, held in Tauri state.
pub struct InputTx(pub Sender<Input>);

/// Folder of the app's data and log file, held in Tauri state.
pub struct DataFolder(pub std::path::PathBuf);

/// Wakes the agent's offer task when a buy is recorded; held in Tauri state.
pub struct Bought(pub Arc<tokio::sync::Notify>);

/// Sets the interface language; the new state comes back via `SettingsChanged`.
#[tauri::command]
#[specta::specta]
pub async fn set_locale(code: String, tx: tauri::State<'_, InputTx>) -> Result<(), String> {
    let locale = Locale::parse(&code).ok_or_else(|| format!("unknown locale {code:?}"))?;
    tx.0.send(Input::Command(Event::LocaleSet(locale)))
        .await
        .map_err(|e| e.to_string())
}

/// Sets the interface accent colour; the new state comes back via `SettingsChanged`.
#[tauri::command]
#[specta::specta]
pub async fn set_accent(name: String, tx: tauri::State<'_, InputTx>) -> Result<(), String> {
    let accent = Accent::parse(&name).ok_or_else(|| format!("unknown accent {name:?}"))?;
    tx.0.send(Input::Command(Event::AccentSet(accent)))
        .await
        .map_err(|e| e.to_string())
}

/// Returns this build's revision, commit hash, build time, channel, update state and changes.
#[tauri::command]
#[specta::specta]
pub fn get_build(app: AppHandle) -> Build {
    build(update::enabled(&app))
}

/// This build as `get_build` describes it, given whether it takes `updates`.
pub fn build(updates: bool) -> Build {
    Build {
        revision: revision::REVISION.parse().unwrap_or_default(),
        hash: revision::HASH.to_owned(),
        built: revision::BUILT.parse().unwrap_or_default(),
        channel: update::own_channel().map_or(ReleaseChannel::Dev, to_release_channel),
        updates,
        changes: revision::changes(),
    }
}

/// Checks the chosen release channel for a release to install; `None` when there is none.
#[tauri::command]
#[specta::specta]
pub async fn check_update(
    app: AppHandle,
    shared: tauri::State<'_, SharedState>,
) -> Result<Option<Release>, String> {
    let chosen = shared
        .0
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .update_channel
        .and_then(from_release_channel);
    update::check(&app, chosen).await
}

/// Downloads and installs the release the last check found; progress arrives via
/// `UpdateProgressed`.
#[tauri::command]
#[specta::specta]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    update::install(&app).await
}

/// Chooses the release channel updates come from; the new state comes back via
/// `SettingsChanged`.
#[tauri::command]
#[specta::specta]
pub async fn set_update_channel(
    channel: ReleaseChannel,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    let channel = from_release_channel(channel).ok_or("dev is not a release channel")?;
    tx.0.send(Input::Command(Event::UpdateChannelSet(channel)))
        .await
        .map_err(|e| e.to_string())
}

/// Records that the launcher is done with updates.
#[tauri::command]
#[specta::specta]
pub fn update_settled(app: AppHandle) {
    window::update_settled(&app);
}

/// Returns the folder Overwolf loads the collector from; `None` when no collector is there.
#[tauri::command]
#[specta::specta]
pub fn get_collector_folder(
    folder: tauri::State<'_, crate::collector::CollectorFolder>,
) -> Option<String> {
    folder
        .0
        .join("manifest.json")
        .is_file()
        .then(|| folder.0.display().to_string())
}

/// Opens the data folder in Explorer with the log file selected.
#[tauri::command]
#[specta::specta]
pub fn show_log_folder(folder: tauri::State<'_, DataFolder>) -> Result<(), String> {
    let log = folder.0.join(crate::logfile::NAME);
    let mut explorer = std::process::Command::new("explorer");
    if log.is_file() {
        explorer.arg(format!("/select,{}", log.display()));
    } else {
        explorer.arg(&folder.0);
    }
    explorer.spawn().map(drop).map_err(|e| e.to_string())
}

/// Opens the collector's folder in Explorer.
#[tauri::command]
#[specta::specta]
pub fn show_collector_folder(
    folder: tauri::State<'_, crate::collector::CollectorFolder>,
) -> Result<(), String> {
    if !folder.0.is_dir() {
        return Err("this build placed no collector".into());
    }
    std::process::Command::new("explorer")
        .arg(&folder.0)
        .spawn()
        .map(drop)
        .map_err(|e| e.to_string())
}

/// Returns the open game data's version, when its sources were fetched and the game build it was
/// made from; `Err` while no game data is open.
#[tauri::command]
#[specta::specta]
pub async fn get_game_data(catalog: tauri::State<'_, Arc<Catalog>>) -> Result<GameData, String> {
    catalog.read(game_data).await.map_err(|e| e.to_string())
}

/// Enqueues a catalog check; the new state comes back via `CatalogChanged`.
#[tauri::command]
#[specta::specta]
pub async fn sync_catalog(tx: tauri::State<'_, InputTx>) -> Result<(), String> {
    tx.0.send(Input::Command(Event::Catalog(CatalogEvent::SyncRequested)))
        .await
        .map_err(|e| e.to_string())
}

/// Returns the last published state snapshot; later changes arrive as events.
#[tauri::command]
#[specta::specta]
pub fn get_state(shared: tauri::State<'_, SharedState>) -> AppState {
    shared
        .0
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Returns prices of every held item from the local market cache, without a warframe.market
/// request.
#[tauri::command]
#[specta::specta]
pub async fn get_prices(
    holdings: tauri::State<'_, SharedHoldings>,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
) -> Result<Vec<Priced>, String> {
    let started = Instant::now();
    let held = holdings.read().inventory;
    let prices = market::prices(&catalog, &market_cache, &held).await?;

    debug!(
        took_us = started.elapsed().as_micros() as u64,
        prices = prices.len(),
        "held item prices served"
    );
    Ok(prices)
}

/// Returns every open warframe.market order for one held item and variant, fetched from the
/// market.
#[tauri::command]
#[specta::specta]
pub async fn get_listings(
    item: String,
    variant: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    freshener: tauri::State<'_, Arc<market::Freshener>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<Orders, String> {
    let held = holdings.read().inventory;
    market::listings(&freshener, &catalog, &market_cache, &held, &item, &variant).await
}

/// Declares the item/variant pairs the window shows, and the one opened, so their prices in the
/// market cache are kept current; an empty list releases them.
#[tauri::command]
#[specta::specta]
pub fn set_shown(shown: Vec<Pair>, opened: Option<Pair>, needs: tauri::State<'_, market::Needs>) {
    let mut wanted: Vec<Need> = shown
        .into_iter()
        .map(|pair| need(pair, Interest::Shown))
        .collect();
    wanted.extend(opened.map(|pair| need(pair, Interest::Waiting)));
    needs.declare(Holder::Screen, wanted);
}

fn need(pair: Pair, interest: Interest) -> Need {
    Need {
        slug: pair.slug,
        variant: pair.key,
        interest,
    }
}

/// Returns the price chart series of one item and variant over the last `days`, for the
/// maximum rank when `top_rank` is true.
#[tauri::command]
#[specta::specta]
pub async fn get_curve(
    item: String,
    variant: String,
    days: u32,
    top_rank: bool,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
) -> Result<Curve, String> {
    market::curve(&catalog, &market_cache, &item, &variant, days, top_rank).await
}

/// Returns prices from the local market cache of every set the account owns a part of, by set
/// slug.
#[tauri::command]
#[specta::specta]
pub async fn get_set_prices(
    holdings: tauri::State<'_, SharedHoldings>,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
) -> Result<Vec<Priced>, String> {
    let held = holdings.read().inventory;
    market::set_prices(&catalog, &market_cache, &held).await
}

/// Returns every holding of the account with its equipped flag, plus the step and the digest
/// of the inventory the list was built from.
#[tauri::command]
#[specta::specta]
pub fn get_inventory(holdings: tauri::State<'_, SharedHoldings>) -> Holdings {
    let started = Instant::now();
    let listed = to_holdings(&holdings.read());

    debug!(
        took_us = started.elapsed().as_micros() as u64,
        pieces = listed.pieces.len(),
        "inventory served"
    );
    listed
}

/// Returns items that have a picture, named in `lang`; `limit` 0 returns all.
#[tauri::command]
#[specta::specta]
pub async fn get_icons(
    catalog: tauri::State<'_, Arc<Catalog>>,
    lang: String,
    limit: u32,
) -> Result<Vec<Icon>, String> {
    catalog
        .read(move |connection| icons(connection, &lang, limit))
        .await
        .map_err(|e| e.to_string())
}

/// Returns names of the star chart nodes `keys`; keys missing from the catalog are omitted and
/// written to the gaps file.
#[tauri::command]
#[specta::specta]
pub async fn get_nodes(
    catalog: tauri::State<'_, Arc<Catalog>>,
    gaps: tauri::State<'_, Arc<Gaps>>,
    keys: Vec<String>,
) -> Result<Vec<Node>, String> {
    let wanted = keys.clone();
    let found = catalog
        .read(move |connection| nodes(connection, &wanted))
        .await
        .map_err(|e| e.to_string())?;

    gaps.note("node", &keys, found.iter().map(|node| node.node.as_str()));
    Ok(found)
}

/// Returns catalog goods of `items` in `lang`, one row per requested key, with mastery state;
/// keys missing from the catalog are written to the gaps file.
#[tauri::command]
#[specta::specta]
pub async fn get_goods(
    catalog: tauri::State<'_, Arc<Catalog>>,
    gaps: tauri::State<'_, Arc<Gaps>>,
    holdings: tauri::State<'_, SharedHoldings>,
    lang: String,
    items: Vec<String>,
) -> Result<Vec<Good>, String> {
    let found = goods_of(&catalog, &holdings.read().inventory, lang, items.clone()).await?;

    gaps.note(
        "item",
        &items,
        found
            .iter()
            .filter(|good| good.known)
            .map(|good| good.item.as_str()),
    );
    Ok(found)
}

/// Catalog goods of `items` in `lang`, each with `mastered` set for what it builds into.
pub(crate) async fn goods_of(
    catalog: &Catalog,
    levelled: &median_core::Inventory,
    lang: String,
    items: Vec<String>,
) -> Result<Vec<Good>, String> {
    let asking = items.clone();
    let mut found = catalog
        .read(move |connection| goods(connection, &lang, &items))
        .await
        .map_err(|e| e.to_string())?;

    let builds = catalog
        .read(move |connection| builds(connection, &asking))
        .await
        .map_err(|e| e.to_string())?;
    for good in &mut found {
        good.mastered = mastered(&builds, levelled, &good.item);
    }
    Ok(found)
}

/// Whether the account mastered the item `item` builds into; `None` when it builds nothing
/// masterable or the inventory has no mastery data.
fn mastered(
    builds: &HashMap<String, crate::catalog::Built>,
    levelled: &median_core::Inventory,
    item: &str,
) -> Option<bool> {
    let built = builds.get(item)?;
    let (mastery, cap) = (built.mastery?, built.cap?);
    if levelled.mastery.is_empty() {
        return None;
    }
    let xp = levelled
        .mastery
        .iter()
        .find(|levelled| levelled.item == built.item)
        .map_or(0, |levelled| levelled.xp);
    Some(mastery.rank(cap, xp) == cap)
}

/// Returns the account's mastery rank, its mastery points by source and progress to the next
/// rank, and every masterable item with its stage.
#[tauri::command]
#[specta::specta]
pub async fn get_mastery(
    lang: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    cache: tauri::State<'_, Arc<MasteryCache>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<MasteryRank, String> {
    crate::mastery::standing(&catalog, &cache, &holdings, &lang).await
}

/// Returns every collectible item with the account's owned count, mastery state and whether it
/// can be built now, drops from the account's relics, and is allowed at its mastery rank.
#[tauri::command]
#[specta::specta]
pub async fn get_catalogue(
    lang: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<Vec<Collectible>, String> {
    crate::collection::collectibles(&catalog, &holdings, &lang).await
}

/// Returns the build tree of `item` checked against the account's items, with credits, build
/// time and in-game price of each step; `None` when no recipe builds it.
#[tauri::command]
#[specta::specta]
pub async fn get_tree(
    lang: String,
    item: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<Option<Tree>, String> {
    crate::collection::tree(&catalog, &market_cache, &holdings, &lang, &item).await
}

/// Returns the roadmap of planned items: combined requirements, mastery gain, ready builds,
/// foundry lines and farming areas. Unbuilt lines with no known source go to the gaps file.
#[tauri::command]
#[specta::specta]
pub async fn get_roadmap(
    lang: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    cache: tauri::State<'_, Arc<MasteryCache>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
    history: tauri::State<'_, Arc<account::Store>>,
    gaps: tauri::State<'_, Arc<Gaps>>,
) -> Result<Roadmap, String> {
    let roadmap =
        crate::roadmap::roadmap(&catalog, &cache, &market_cache, &holdings, &history, &lang)
            .await?;
    gaps.note_named(
        "way",
        roadmap
            .total
            .lines
            .iter()
            .filter(|line| line.way.is_none() && !line.built)
            .map(|line| (line.item.as_str(), line.name.as_str())),
    );
    Ok(roadmap)
}

/// Returns every syndicate with known vendors and their offers priced from the market cache:
/// affiliated syndicates first, by spendable standing, highest first; then the rest.
#[tauri::command]
#[specta::specta]
pub async fn get_syndicates(
    lang: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
    needs: tauri::State<'_, market::Needs>,
    shared: tauri::State<'_, SharedState>,
) -> Result<Vec<Syndicate>, String> {
    let policy = from_policy(
        &shared
            .0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .policy,
    );
    crate::syndicates::syndicates(&catalog, &market_cache, &holdings, &needs, &policy, &lang).await
}

/// Declares the roadmap item/variant pairs whose prices are kept current; an empty list
/// releases them.
#[tauri::command]
#[specta::specta]
pub fn set_planned(planned: Vec<Pair>, needs: tauri::State<'_, market::Needs>) {
    let wanted = planned
        .into_iter()
        .map(|pair| need(pair, Interest::Shown))
        .collect();
    needs.declare(Holder::Plan, wanted);
}

/// Marks (`on`) or unmarks an item as a favourite of the current profile.
#[tauri::command]
#[specta::specta]
pub async fn set_favourite(
    item: String,
    on: bool,
    history: tauri::State<'_, Arc<account::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<(), String> {
    history
        .set_favourite(holdings.read().inventory.profile.clone(), item, on)
        .await
        .map_err(|e| e.to_string())
}

/// Returns every item marked as a favourite of the current profile.
#[tauri::command]
#[specta::specta]
pub async fn get_favourites(
    history: tauri::State<'_, Arc<account::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<Vec<String>, String> {
    history
        .favourites(holdings.read().inventory.profile.clone())
        .await
        .map_err(|e| e.to_string())
}

/// Adds (`on`) or removes an item from the current profile's roadmap.
#[tauri::command]
#[specta::specta]
pub async fn set_roadmap(
    item: String,
    on: bool,
    history: tauri::State<'_, Arc<account::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<(), String> {
    history
        .set_roadmap(holdings.read().inventory.profile.clone(), item, on)
        .await
        .map_err(|e| e.to_string())
}

/// Removes every item from the current profile's roadmap.
#[tauri::command]
#[specta::specta]
pub async fn clear_roadmap(
    history: tauri::State<'_, Arc<account::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<(), String> {
    history
        .clear_roadmap(holdings.read().inventory.profile.clone())
        .await
        .map_err(|e| e.to_string())
}

/// Records whether a part of an item in the roadmap is to be bought rather than farmed.
#[tauri::command]
#[specta::specta]
pub async fn set_part_choice(
    item: String,
    part: String,
    buy: bool,
    history: tauri::State<'_, Arc<account::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
) -> Result<(), String> {
    history
        .set_part_choice(
            holdings.read().inventory.profile.clone(),
            account::Choice { item, part, buy },
        )
        .await
        .map_err(|e| e.to_string())
}

/// Returns the item's name in the catalog's other language (English for `ru`, Russian
/// otherwise); `None` when the catalog has no row or no name in that language.
#[tauri::command]
#[specta::specta]
pub async fn get_other_name(
    catalog: tauri::State<'_, Arc<Catalog>>,
    lang: String,
    item: String,
) -> Result<Option<String>, String> {
    catalog
        .read(move |connection| crate::catalog::other_name(connection, &lang, &item))
        .await
        .map_err(|e| e.to_string())
}

/// Returns names and pictures of every currency in the catalog.
#[tauri::command]
#[specta::specta]
pub async fn get_coins(
    catalog: tauri::State<'_, Arc<Catalog>>,
    lang: String,
) -> Result<Vec<Coin>, String> {
    catalog
        .read(move |connection| coins(connection, &lang))
        .await
        .map_err(|e| e.to_string())
}

/// Returns the catalog card of one item (sources, contents, recipes, uses); `None` when the item
/// is not in the catalog.
#[tauri::command]
#[specta::specta]
pub async fn get_card(
    catalog: tauri::State<'_, Arc<Catalog>>,
    lang: String,
    item: String,
) -> Result<Option<Card>, String> {
    catalog
        .read(move |connection| card(connection, &lang, &item))
        .await
        .map_err(|e| e.to_string())
}

/// Returns the grade of `riven` at `rank` against its weapon's good-rolls sheet, with each
/// stat's verdict, labels in `lang`; `None` when the weapon is unknown or a stat cannot roll.
#[tauri::command]
#[specta::specta]
pub async fn get_riven_appraisal(
    catalog: tauri::State<'_, Arc<Catalog>>,
    sheet: tauri::State<'_, Arc<rivens::Sheet>>,
    lang: String,
    riven: Riven,
    rank: u32,
) -> Result<Option<RivenAppraisal>, String> {
    rivens::appraisal(&catalog, &sheet, &lang, riven, rank).await
}

/// Returns the price band (low, middle, high) of rivens similar to `riven`; `None` when the
/// weapon is unknown or warframe.market does not list its rivens.
#[tauri::command]
#[specta::specta]
pub async fn get_riven_band(
    catalog: tauri::State<'_, Arc<Catalog>>,
    market: tauri::State<'_, market::Gateway>,
    slices: tauri::State<'_, Arc<rivens::Slices>>,
    weekly: tauri::State<'_, Arc<rivens::Weekly>>,
    riven: Riven,
) -> Result<Option<RivenBand>, String> {
    rivens::band(&catalog, &market, &slices, &weekly, riven).await
}

/// Returns the highest price of a rerolled riven of the weapon's family in last week's trades,
/// in platinum; `None` when unknown.
#[tauri::command]
#[specta::specta]
pub async fn get_riven_ceiling(
    catalog: tauri::State<'_, Arc<Catalog>>,
    weekly: tauri::State<'_, Arc<rivens::Weekly>>,
    weapon: String,
) -> Result<Option<f64>, String> {
    rivens::ceiling(&catalog, &weekly, weapon).await
}

/// Returns the demand for rivens of the weapon, rerolled or not; `None` before the weekly
/// trades are loaded or when the weapon's riven family is unknown.
#[tauri::command]
#[specta::specta]
pub async fn get_riven_demand(
    catalog: tauri::State<'_, Arc<Catalog>>,
    slices: tauri::State<'_, Arc<rivens::Slices>>,
    weekly: tauri::State<'_, Arc<rivens::Weekly>>,
    weapon: String,
    rerolled: bool,
) -> Result<Option<RivenDemand>, String> {
    rivens::demand(&catalog, &slices, &weekly, weapon, rerolled).await
}

/// Returns the in-game name of each riven, in input order; `None` for a riven whose name cannot
/// be built from the catalog.
#[tauri::command]
#[specta::specta]
pub async fn get_riven_names(
    catalog: tauri::State<'_, Arc<Catalog>>,
    rivens: Vec<Riven>,
) -> Result<Vec<Option<String>>, String> {
    rivens::names(&catalog, rivens).await
}

/// Returns the value of a riven when sold (price band) and when transmuted, with similar
/// auctions; `None` when the weapon is unknown.
#[tauri::command]
#[specta::specta]
pub async fn get_riven_choices(
    catalog: tauri::State<'_, Arc<Catalog>>,
    market: tauri::State<'_, market::Gateway>,
    slices: tauri::State<'_, Arc<rivens::Slices>>,
    weekly: tauri::State<'_, Arc<rivens::Weekly>>,
    lang: String,
    riven: Riven,
) -> Result<Option<RivenChoices>, String> {
    rivens::choices(&catalog, &market, &slices, &weekly, &lang, riven).await
}

/// Returns, for no lock and for each stat locked, the chance that a reroll gives a pricier
/// riven, the expected kuva and the mean price of those rolls; `None` when it cannot be priced.
#[tauri::command]
#[specta::specta]
pub async fn get_riven_rolls(
    catalog: tauri::State<'_, Arc<Catalog>>,
    market: tauri::State<'_, market::Gateway>,
    slices: tauri::State<'_, Arc<rivens::Slices>>,
    weekly: tauri::State<'_, Arc<rivens::Weekly>>,
    riven: Riven,
) -> Result<Option<Vec<RerollOdds>>, String> {
    rivens::rolls(&catalog, &market, &slices, &weekly, riven).await
}

/// Returns the account's riven auctions that are not closed, newest first.
#[tauri::command]
#[specta::specta]
pub async fn get_own_auctions(
    catalog: tauri::State<'_, Arc<Catalog>>,
    market: tauri::State<'_, market::Gateway>,
    tx: tauri::State<'_, InputTx>,
) -> Result<Vec<OwnAuction>, String> {
    let session = session(&market)?;
    let auctions = refused(
        market
            .call_auction(Priority::Person, |pass| session.my_auctions(pass))
            .await,
        &tx,
    )
    .await?;
    rivens::own_auctions(&catalog, auctions).await
}

/// Sets the starting price, buyout price (`None` for none) and visibility of one of the
/// account's riven auctions on warframe.market.
#[tauri::command]
#[specta::specta]
pub async fn change_riven_auction(
    market: tauri::State<'_, market::Gateway>,
    tx: tauri::State<'_, InputTx>,
    id: String,
    starting: u32,
    buyout: Option<u32>,
    visible: bool,
) -> Result<(), String> {
    let session = session(&market)?;
    let change = AuctionChange {
        starting_price: Some(starting),
        buyout_price: buyout,
        visible: Some(visible),
        ..AuctionChange::default()
    };
    refused(
        market
            .call_auction(Priority::Person, |pass| {
                session.change_auction(pass, &id, &change)
            })
            .await,
        &tx,
    )
    .await
    .inspect_err(|why| warn!(auction_id = id, error = %why, "riven auction change refused"))?;
    info!(
        auction_id = id,
        starting, buyout, visible, "riven auction changed"
    );
    Ok(())
}

/// Opens a riven auction's page on warframe.market in the default browser; `id` must be 1 to
/// 40 ASCII letters or digits. Does nothing off Windows.
#[tauri::command]
#[specta::specta]
pub fn open_auction(id: String) -> Result<(), String> {
    if id.is_empty() || id.len() > 40 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(format!("not an auction id: {id}"));
    }
    let page = format!("https://warframe.market/auction/{id}");
    #[cfg(windows)]
    std::process::Command::new("explorer.exe")
        .arg(&page)
        .spawn()
        .map_err(|e| e.to_string())?;
    #[cfg(not(windows))]
    let _ = page;
    Ok(())
}

/// Closes one of the account's riven auctions on warframe.market.
#[tauri::command]
#[specta::specta]
pub async fn close_riven_auction(
    market: tauri::State<'_, market::Gateway>,
    tx: tauri::State<'_, InputTx>,
    id: String,
) -> Result<(), String> {
    let session = session(&market)?;
    refused(
        market
            .call_auction(Priority::Person, |pass| session.close_auction(pass, &id))
            .await,
        &tx,
    )
    .await
    .inspect_err(|why| warn!(auction_id = id, error = %why, "riven auction close failed"))?;
    info!(auction_id = id, "riven auction closed");
    Ok(())
}

/// Creates a warframe.market auction for a riven at `rank`; returns the auction id.
#[tauri::command]
#[specta::specta]
pub async fn put_up_riven(
    catalog: tauri::State<'_, Arc<Catalog>>,
    market: tauri::State<'_, market::Gateway>,
    tx: tauri::State<'_, InputTx>,
    riven: Riven,
    rank: u32,
    starting: u32,
    buyout: Option<u32>,
) -> Result<String, String> {
    let draft = rivens::draft(&catalog, riven, rank, starting, buyout).await?;
    let session = session(&market)?;
    let auction = refused(
        market
            .call_auction(Priority::Person, |pass| session.put_up(pass, &draft))
            .await,
        &tx,
    )
    .await
    .inspect_err(
        |why| warn!(weapon = draft.weapon, error = %why, "riven auction create refused"),
    )?;
    info!(
        weapon = draft.weapon,
        auction_id = auction.id,
        starting,
        buyout,
        "riven auction created"
    );
    Ok(auction.id)
}

/// Returns the median closed-trade price of each item from the local market cache; an item
/// traded only as a set is priced as its set.
#[tauri::command]
#[specta::specta]
pub async fn get_quotes(
    items: Vec<String>,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
) -> Result<Vec<Quote>, String> {
    quotes_of(&catalog, &market_cache, items).await
}

/// Median closed-trade price of each item from the market cache; an item traded only as a set
/// is priced as the set.
pub(crate) async fn quotes_of(
    catalog: &Catalog,
    market_cache: &market::Store,
    items: Vec<String>,
) -> Result<Vec<Quote>, String> {
    let mut quotes: Vec<Quote> = market::quoted(catalog, market_cache, &items)
        .await?
        .into_iter()
        .map(|quote| Quote {
            item: quote.item,
            platinum: quote.platinum,
            pair: Some(Pair {
                slug: quote.slug,
                key: quote.variant,
            }),
        })
        .collect();

    let quoted: HashSet<String> = quotes.iter().map(|quote| quote.item.clone()).collect();
    let unlisted: HashSet<String> = items
        .into_iter()
        .filter(|item| !quoted.contains(item))
        .collect();
    if unlisted.is_empty() {
        return Ok(quotes);
    }
    let built: Vec<(String, String)> = catalog
        .read(move |connection| sets(connection, "en"))
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter_map(|set| {
            set.built
                .filter(|item| unlisted.contains(item))
                .map(|item| (item, set.slug))
        })
        .collect();
    let slugs: Vec<String> = built.iter().map(|(_, slug)| slug.clone()).collect();
    let whole = market::quoted_sets(market_cache, &slugs).await?;
    quotes.extend(built.into_iter().map(|(item, slug)| Quote {
        item,
        platinum: whole.get(&slug).copied(),
        pair: Some(Pair {
            slug,
            key: String::new(),
        }),
    }));
    Ok(quotes)
}

/// Returns the classes and kinds the catalog groups items by.
#[tauri::command]
#[specta::specta]
pub async fn get_taxonomy(
    catalog: tauri::State<'_, Arc<Catalog>>,
    lang: String,
) -> Result<Taxonomy, String> {
    catalog
        .read(move |connection| taxonomy(connection, &lang))
        .await
        .map_err(|e| e.to_string())
}

/// Returns every set in the catalog, named in `lang`.
#[tauri::command]
#[specta::specta]
pub async fn get_sets(
    catalog: tauri::State<'_, Arc<Catalog>>,
    lang: String,
) -> Result<Vec<SetRow>, String> {
    catalog
        .read(move |connection| sets(connection, &lang))
        .await
        .map_err(|e| e.to_string())
}

/// Returns the account's relics with the expected value of each refinement for a squad of
/// `squad` players opening the same relic.
#[tauri::command]
#[specta::specta]
pub async fn get_relics(
    holdings: tauri::State<'_, SharedHoldings>,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    lang: String,
    squad: u32,
) -> Result<Vec<Relic>, String> {
    let started = Instant::now();
    let held = relics::held(&catalog, &market_cache, &holdings, &lang, squad).await?;

    debug!(
        took_us = started.elapsed().as_micros() as u64,
        relics = held.len(),
        squad,
        "relic payback served"
    );
    Ok(held)
}

/// Returns the relics of `items` with the expected value of each refinement for a squad of
/// `squad`.
#[tauri::command]
#[specta::specta]
pub async fn get_relics_of(
    holdings: tauri::State<'_, SharedHoldings>,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    lang: String,
    items: Vec<String>,
    squad: u32,
) -> Result<Vec<Relic>, String> {
    let counts = holdings.counted();
    relics::priced(&catalog, &market_cache, &counts, items, &lang, squad).await
}

/// Returns the keys of items matching `query`, best match first; `limit` 0 returns all.
#[tauri::command]
#[specta::specta]
pub async fn get_search(
    catalog: tauri::State<'_, Arc<Catalog>>,
    lang: String,
    query: String,
    limit: u32,
) -> Result<Vec<String>, String> {
    catalog
        .read(move |connection| search(connection, &lang, &query, limit))
        .await
        .map_err(|e| e.to_string())
}

/// Signs in to warframe.market, saves the token in the system credential store, passes it to
/// the market socket, then reads the account's own orders; a failed orders read is only logged.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn sign_in(
    email: String,
    password: String,
    market: tauri::State<'_, market::Gateway>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    history: tauri::State<'_, Arc<account::Store>>,
    needs: tauri::State<'_, market::Needs>,
    stream: tauri::State<'_, Stream>,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    let session = market
        .call(Priority::Person, |pass| {
            Session::open(Arc::clone(market.client()), pass, &email, &password)
        })
        .await
        .map_err(|e| e.to_string())?;

    Token::write(session.authorization())?;
    stream.ask(Ask::SignIn(session.authorization().to_owned()));

    let name = session.name().map(str::to_owned);
    if let Err(why) = Token::write_name(name.as_deref()) {
        warn!(error = %why, "account name not saved");
    }
    say(&tx, MembershipEvent::SignedIn { name }).await?;

    // After sign-in the orders page has data to show; a failure here concerns the orders
    // read, not the sign-in, which has already succeeded.
    let authorization = session.authorization().to_owned();
    if let Err(why) = market::read_own_orders(
        &market,
        Priority::Person,
        &market_cache,
        &history,
        &needs,
        &tx.0,
        authorization,
    )
    .await
    {
        warn!(error = %why, "own orders read after sign-in failed");
    }
    Ok(())
}

/// Deletes the token from the system credential store and signs out the market socket and the
/// engine.
#[tauri::command]
#[specta::specta]
pub async fn sign_out(
    stream: tauri::State<'_, Stream>,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    Token::clear();
    stream.ask(Ask::SignOut);
    say(&tx, MembershipEvent::SignedOut).await
}

/// Sets the account's warframe.market status by its code and turns off following the game.
#[tauri::command]
#[specta::specta]
pub async fn set_status(
    code: String,
    stream: tauri::State<'_, Stream>,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    let status = Status::parse(&code).ok_or_else(|| format!("no status goes by {code}"))?;
    stream.ask(Ask::Status(status));
    // Following the game and a manual status are exclusive; a manually set status is kept.
    tx.0.send(Input::Command(Event::FollowGameSet(false)))
        .await
        .map_err(|e| e.to_string())
}

/// Turns following the game on or off: the warframe.market status is "in game" while the game
/// runs and "invisible" otherwise.
#[tauri::command]
#[specta::specta]
pub async fn set_status_auto(on: bool, tx: tauri::State<'_, InputTx>) -> Result<(), String> {
    tx.0.send(Input::Command(Event::FollowGameSet(on)))
        .await
        .map_err(|e| e.to_string())
}

/// Sends a membership event to the engine.
async fn say(tx: &tauri::State<'_, InputTx>, event: MembershipEvent) -> Result<(), String> {
    tx.0.send(Input::Command(Event::Membership(event)))
        .await
        .map_err(|e| e.to_string())
}

/// Starts or stops the agent; the new state comes back via `AgentChanged`.
#[tauri::command]
#[specta::specta]
pub async fn set_agent(
    on: bool,
    running: tauri::State<'_, Arc<tokio::sync::Mutex<bool>>>,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    *running.lock().await = on;

    tx.0.send(Input::Command(Event::Agent(AgentEvent::RunningSet(on))))
        .await
        .map_err(|e| e.to_string())
}

/// Sets the policy limits the agent trades within; the new state comes back via `PolicyChanged`.
#[tauri::command]
#[specta::specta]
pub async fn set_policy(
    policy: median_ipc::Policy,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    tx.0.send(Input::Command(Event::PolicySet(from_policy(&policy))))
        .await
        .map_err(|e| e.to_string())
}

/// Returns the in-game trades of the last `days` days (`0` for all), items named in `lang`,
/// with each complete set of parts shown as one set row.
#[tauri::command]
#[specta::specta]
pub async fn get_swaps(
    days: u32,
    lang: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    history: tauri::State<'_, Arc<account::Store>>,
) -> Result<Vec<median_ipc::Swap>, String> {
    let from = match days {
        0 => 0,
        days => median_core::seconds() - i64::from(days) * median_core::DAY,
    };
    let mut swaps = history.swaps_since(from).await.map_err(|e| e.to_string())?;

    let names: Vec<String> = swaps
        .iter()
        .flat_map(|swap| swap.given.iter().chain(swap.taken.iter()))
        .filter(|part| part.kind == median_core::SwapKind::Item)
        .map(|part| part.name.clone())
        .collect();
    let known = crate::catalog::told_by_fold(&catalog, &lang, names).await?;

    let items: Vec<String> = known
        .values()
        .map(|told| told.item.clone())
        .filter(|item| !item.is_empty())
        .collect();
    let kits = kits(&catalog, &lang, items).await?;

    for swap in &mut swaps {
        for side in [&mut swap.given, &mut swap.taken] {
            *side = gathered(std::mem::take(side), &known, &kits);
        }
        for part in swap.given.iter_mut().chain(swap.taken.iter_mut()) {
            if let Some(told) = known.get(&median_core::fold(&part.name)) {
                part.name = told.name.clone();
            }
        }
    }
    Ok(swaps.iter().map(crate::state_sink::to_swap).collect())
}

/// A set that one side of a trade may contain: its parts and its display name.
struct Kit {
    /// Each part and the number of copies one set needs.
    members: Vec<(String, u32)>,
    name: String,
}

/// The sets `items` belong to, each with its parts; sets without a catalog name are skipped.
async fn kits(catalog: &Arc<Catalog>, lang: &str, items: Vec<String>) -> Result<Vec<Kit>, String> {
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let asked = lang.to_owned();
    catalog
        .read(move |connection| {
            let mut members: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
            for part in crate::catalog::kit(connection, &asked, &items)? {
                let of = members.entry(part.slug).or_default();
                if !of.iter().any(|(item, _)| *item == part.item) {
                    of.push((part.item, part.per_set));
                }
            }
            let mut kits = Vec::new();
            for (slug, members) in members {
                let Some(named) = crate::catalog::named(connection, &asked, &slug)? else {
                    continue;
                };
                kits.push(Kit {
                    members,
                    name: named.name,
                });
            }
            Ok(kits)
        })
        .await
        .map_err(|e| e.to_string())
}

/// One side of a trade with every complete set of parts replaced by one row for the set.
fn gathered(
    parts: Vec<median_core::SwapPart>,
    known: &HashMap<String, crate::catalog::Told>,
    kits: &[Kit],
) -> Vec<median_core::SwapPart> {
    let key = |part: &median_core::SwapPart| {
        (part.kind == median_core::SwapKind::Item)
            .then(|| known.get(&median_core::fold(&part.name)))
            .flatten()
            .map(|told| told.item.clone())
    };

    let mut left: HashMap<String, u32> = HashMap::new();
    for part in &parts {
        if let Some(item) = key(part) {
            *left.entry(item).or_default() += part.count;
        }
    }

    let mut whole = Vec::new();
    for kit in kits {
        let made = kit
            .members
            .iter()
            .map(|(item, need)| left.get(item).copied().unwrap_or(0) / (*need).max(1))
            .min()
            .unwrap_or(0);
        if made == 0 {
            continue;
        }
        for (item, need) in &kit.members {
            if let Some(count) = left.get_mut(item) {
                *count -= made * (*need).max(1);
            }
        }
        whole.push(median_core::SwapPart {
            name: kit.name.clone(),
            count: made,
            kind: median_core::SwapKind::Item,
            rank: None,
        });
    }
    if whole.is_empty() {
        return parts;
    }

    // The parts used by the sets are removed in the order the game log lists them, so the
    // remaining rows keep their order and their rank.
    let mut taken: HashMap<String, u32> = left
        .iter()
        .map(|(item, over)| (item.clone(), *over))
        .collect();
    let mut rest = Vec::new();
    for mut part in parts {
        let Some(item) = key(&part) else {
            rest.push(part);
            continue;
        };
        let over = taken.entry(item).or_default();
        let keeps = part.count.min(*over);
        *over -= keeps;
        if keeps == 0 {
            continue;
        }
        part.count = keeps;
        rest.push(part);
    }
    whole.extend(rest);
    whole
}

/// Sets the sound alert settings for game log events; the new state comes back in `AppState`.
#[tauri::command]
#[specta::specta]
pub async fn set_alert(
    alert: median_ipc::Alert,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    tx.0.send(Input::Command(Event::AlertSet(from_alert(&alert))))
        .await
        .map_err(|e| e.to_string())
}

/// Adds (`on`) or removes a world state watch; the new state comes back via `SettingsChanged`.
#[tauri::command]
#[specta::specta]
pub async fn set_watch(
    watch: median_ipc::Watch,
    on: bool,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    tx.0.send(Input::Command(Event::WatchSet {
        watch: from_watch(&watch),
        on,
    }))
    .await
    .map_err(|e| e.to_string())
}

/// Returns the account's own orders as saved in the account store and the market cache, without
/// a warframe.market request.
#[tauri::command]
#[specta::specta]
pub async fn get_orders(
    lang: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    history: tauri::State<'_, Arc<account::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
    shared: tauri::State<'_, SharedState>,
) -> Result<Vec<OwnOrder>, String> {
    let held = holdings.read().inventory;
    let policy = from_policy(
        &shared
            .0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .policy,
    );
    market::own_orders(&catalog, &market_cache, &history, &held, &lang, &policy).await
}

/// Returns the prime parts gained since the haul started, not counting copies bought, received
/// in in-game trades or built.
#[tauri::command]
#[specta::specta]
pub async fn get_haul(
    lang: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
    history: tauri::State<'_, Arc<account::Store>>,
) -> Result<Haul, String> {
    crate::haul::haul(&catalog, &market_cache, &holdings, &history, &lang).await
}

/// Restarts the haul now, with the last inventory read as its baseline.
#[tauri::command]
#[specta::specta]
pub async fn restart_haul(
    holdings: tauri::State<'_, SharedHoldings>,
    history: tauri::State<'_, Arc<account::Store>>,
) -> Result<(), String> {
    crate::haul::restart(&holdings, &history).await
}

/// Returns the open positions (items bought for resale) with purchase price, the lowest in-game
/// sell price and the account's own ask.
#[tauri::command]
#[specta::specta]
pub async fn get_positions(
    lang: String,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    history: tauri::State<'_, Arc<account::Store>>,
) -> Result<Vec<median_ipc::Position>, String> {
    market::positions(&catalog, &market_cache, &history, &lang).await
}

/// Reads the account's own orders from warframe.market and saves them to the account store;
/// `Err` when signed out.
#[tauri::command]
#[specta::specta]
pub async fn refresh_orders(
    market: tauri::State<'_, market::Gateway>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    history: tauri::State<'_, Arc<account::Store>>,
    needs: tauri::State<'_, market::Needs>,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    let authorization = Token::read().ok_or_else(|| "no session".to_owned())?;
    refused(
        market::read_own_orders(
            &market,
            Priority::Person,
            &market_cache,
            &history,
            &needs,
            &tx.0,
            authorization,
        )
        .await,
        &tx,
    )
    .await?;
    Ok(())
}

/// Changes only the given fields (platinum, quantity, visibility) of one own order and returns
/// the updated order; `Err` when no field is given.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn amend_order(
    id: String,
    amend: median_ipc::Amend,
    lang: String,
    market: tauri::State<'_, market::Gateway>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    catalog: tauri::State<'_, Arc<Catalog>>,
    history: tauri::State<'_, Arc<account::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
    shared: tauri::State<'_, SharedState>,
    tx: tauri::State<'_, InputTx>,
) -> Result<OwnOrder, String> {
    let mut change = serde_json::Map::new();
    if let Some(platinum) = amend.platinum {
        change.insert("platinum".into(), platinum.into());
    }
    if let Some(quantity) = amend.quantity {
        change.insert("quantity".into(), quantity.into());
    }
    if let Some(visible) = amend.visible {
        change.insert("visible".into(), visible.into());
    }
    if change.is_empty() {
        return Err("nothing to change".to_owned());
    }

    let standing = amended(&id, change.into(), &market, &market_cache, &history, &tx).await?;
    let held = holdings.read().inventory;
    let positions = history.open_positions().await.map_err(|e| e.to_string())?;
    let basis = positions
        .iter()
        .find(|held| {
            Some(held.slug.as_str()) == standing.slug.as_deref() && held.variant == standing.variant
        })
        .map(account::Position::basis);
    let policy = from_policy(
        &shared
            .0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .policy,
    );
    let mut row = market::one_order(
        &catalog,
        &market_cache,
        &held,
        &lang,
        standing,
        median_core::seconds(),
        &policy,
    )
    .await?;
    row.basis = basis;
    Ok(row)
}

/// Places one order on warframe.market at the user's request; when every order slot is taken,
/// the agent's least promising bid is withdrawn first.
#[tauri::command]
#[specta::specta]
pub async fn place_order(
    posting: Posting,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market: tauri::State<'_, market::Gateway>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    history: tauri::State<'_, Arc<account::Store>>,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    let started = std::time::Instant::now();
    let item = posting.item.clone();
    // A built item traded only as a set is listed under the set, a relic under its refinement.
    let unheld = || async {
        market::unheld(&catalog, &market_cache, &item)
            .await?
            .ok_or_else(|| format!("the market does not list {item}"))
    };
    let slug = unheld().await?.slug;
    if let Err(why) = market::describe(&market, &market_cache, Priority::Person, &slug).await {
        warn!(slug, error = %why, "market item details read failed, using cached");
    }
    let listed = unheld().await?;

    let item_id = market_cache
        .id_of(slug.clone())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("the market cache has no id for {slug}"))?;

    let order = Placing {
        item,
        item_id,
        side: match posting.selling {
            true => Side::Sell,
            false => Side::Buy,
        },
        platinum: posting.platinum,
        quantity: posting.quantity,
        per_trade: market::lots_allowed(&market, &market_cache, Priority::Person, &slug)
            .await?
            .then_some(posting.per_trade.unwrap_or(1).max(1)),
        rank: posting.rank,
        subtype: posting.subtype.or(listed.of.subtype),
        visible: true,
    };

    make_room(&market, &history).await;

    let session = session(&market)?;
    let placed = refused(
        market
            .call(Priority::Person, |pass| session.place_by_hand(pass, &order))
            .await,
        &tx,
    )
    .await
    .inspect_err(|why| warn!(slug, error = %why, "manual order refused"))?;

    let variant = variant_of(&placed);
    history
        .keep_one_order((placed, Some(slug.clone()), variant))
        .await
        .map_err(|e| e.to_string())?;
    info!(
        slug,
        took_ms = started.elapsed().as_millis() as u64,
        "manual order placed"
    );
    Ok(())
}

/// Withdraws the agent's least promising bid when all `SLOTS` order slots are taken; a failure
/// is logged and the caller goes on.
async fn make_room(market: &market::Gateway, history: &Arc<account::Store>) {
    let Ok(standing) = history.standing_orders().await else {
        return;
    };
    if (standing.len() as u32) < median_core::SLOTS {
        return;
    }

    let Some(order) = agent::worst_buy(&standing) else {
        return;
    };

    match market::withdraw_own_order(market, Priority::Person, history, &order.id).await {
        Ok(()) => info!(
            order_id = order.id,
            slug = order.slug,
            "agent bid withdrawn for manual order"
        ),
        Err(why) => {
            warn!(error = %why, order_id = order.id, "agent bid not withdrawn for manual order")
        }
    }
}

/// Records the agent's riven pick `id` as bought at `price` platinum for resale, then removes
/// the pick.
#[tauri::command]
#[specta::specta]
pub async fn record_riven_buy(
    shared: tauri::State<'_, SharedState>,
    history: tauri::State<'_, Arc<account::Store>>,
    tx: tauri::State<'_, InputTx>,
    id: String,
    price: u32,
) -> Result<(), String> {
    let pick = shared
        .0
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .agent
        .picks
        .iter()
        .find(|pick| pick.id == id)
        .cloned()
        .ok_or_else(|| format!("no pick {id}"))?;
    let riven = serde_json::to_string(&pick.riven).map_err(|e| e.to_string())?;
    history
        .keep_riven_buy(account::RivenBought {
            auction: pick.id.clone(),
            weapon: pick.riven.weapon.clone(),
            name: pick.name.clone(),
            riven,
            rank: pick.rank,
            price,
            bought: median_core::seconds(),
            sold: None,
            sale: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    info!(auction_id = id, platinum = price, "riven bought for resale");
    drop_pick(&tx, id).await
}

/// Removes the agent's riven pick `id` without recording a buy.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_riven_pick(tx: tauri::State<'_, InputTx>, id: String) -> Result<(), String> {
    drop_pick(&tx, id).await
}

async fn drop_pick(tx: &tauri::State<'_, InputTx>, id: String) -> Result<(), String> {
    tx.0.send(Input::Command(Event::Agent(AgentEvent::PickDropped(id))))
        .await
        .map_err(|e| e.to_string())
}

/// Records that the resale riven bought from `auction` was sold for `price` platinum.
#[tauri::command]
#[specta::specta]
pub async fn record_riven_sale(
    history: tauri::State<'_, Arc<account::Store>>,
    auction: String,
    price: u32,
) -> Result<(), String> {
    history
        .sell_riven(auction.clone(), price)
        .await
        .map_err(|e| e.to_string())?;
    info!(auction_id = auction, platinum = price, "resale riven sold");
    Ok(())
}

/// Returns every riven bought for resale, newest first.
#[tauri::command]
#[specta::specta]
pub async fn get_riven_buys(
    history: tauri::State<'_, Arc<account::Store>>,
) -> Result<Vec<RivenBuy>, String> {
    let bought = history.rivens_bought().await.map_err(|e| e.to_string())?;
    bought
        .into_iter()
        .map(|riven| {
            Ok(RivenBuy {
                riven: serde_json::from_str(&riven.riven).map_err(|e| e.to_string())?,
                auction: riven.auction,
                name: riven.name,
                rank: riven.rank,
                price: riven.price,
                bought: riven.bought.max(0) as u32,
                sold: riven.sold.map(|at| at.max(0) as u32),
                sale: riven.sale,
            })
        })
        .collect()
}

/// Passes `quantity` owned copies to the agent: records a buy at `platinum` per copy under the
/// copy's warframe.market variant, which opens the position, and wakes the agent. `Err` when
/// either is 0 or the market does not list the copy.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn hand_to_agent(
    item: String,
    variant: String,
    platinum: u32,
    quantity: u32,
    catalog: tauri::State<'_, Arc<Catalog>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    history: tauri::State<'_, Arc<account::Store>>,
    holdings: tauri::State<'_, SharedHoldings>,
    bought: tauri::State<'_, Bought>,
) -> Result<(), String> {
    if platinum == 0 || quantity == 0 {
        return Err("a copy is handed over at a price, one at least".into());
    }
    let held = holdings.read().inventory;
    let listed = market::listed(&catalog, &market_cache, &held)
        .await?
        .into_iter()
        .find(|listed| listed.item == item && listed.variant == variant)
        .ok_or_else(|| format!("{item} is not held as a copy the market lists"))?;

    let slug = listed.slug;
    let made = account::Made {
        item,
        slug: slug.clone(),
        variant: listed.of.key(),
        selling: false,
        platinum,
        quantity,
        origin: Origin::Hand.word().to_owned(),
        own: None,
    };
    history
        .record_exchange(made)
        .await
        .map_err(|e| e.to_string())?;
    bought.0.notify_one();
    info!(slug, platinum, quantity, "copies handed to agent");
    Ok(())
}

/// Passes one of the user's open orders to the agent. A sell order also opens its position at
/// `platinum` per copy (required, above 0); a buy order joins the agent's bids.
#[tauri::command]
#[specta::specta]
pub async fn hand_order_to_agent(
    id: String,
    platinum: Option<u32>,
    catalog: tauri::State<'_, Arc<Catalog>>,
    history: tauri::State<'_, Arc<account::Store>>,
    bought: tauri::State<'_, Bought>,
) -> Result<(), String> {
    let standing = history.standing_orders().await.map_err(|e| e.to_string())?;
    let order = standing
        .into_iter()
        .find(|order| order.id == id)
        .ok_or_else(|| format!("order {id} is not standing"))?;
    if order.by_agent {
        return Err("the order is the agent's already".into());
    }

    if order.selling {
        let platinum = platinum
            .filter(|platinum| *platinum > 0)
            .ok_or("a sale is handed over at what a copy cost, one at least")?;
        let slug = order.slug.clone().ok_or("the order names no item")?;
        let asked = slug.clone();
        let item = catalog
            .read(move |connection| crate::catalog::named(connection, "en", &asked))
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("{slug} is not in the catalog"))?
            .item;
        let made = account::Made {
            item,
            slug,
            variant: order.variant.clone(),
            selling: false,
            platinum,
            quantity: order.quantity,
            origin: Origin::Hand.word().to_owned(),
            own: None,
        };
        history
            .record_exchange(made)
            .await
            .map_err(|e| e.to_string())?;
    }
    history
        .hand_order(order.id.clone())
        .await
        .map_err(|e| e.to_string())?;
    bought.0.notify_one();
    info!(order_id = order.id, "order handed to agent");
    Ok(())
}

/// Takes a position away from the agent; its sell order stays listed as the user's own.
#[tauri::command]
#[specta::specta]
pub async fn release_position(
    slug: String,
    variant: String,
    history: tauri::State<'_, Arc<account::Store>>,
) -> Result<(), String> {
    history
        .release_position(slug.clone(), variant)
        .await
        .map_err(|e| e.to_string())?;
    info!(slug, "position released from agent");
    Ok(())
}

/// Records a closed trade the user confirmed and closes the order it filled; a trade already
/// recorded lately is skipped.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn record_exchange(
    order: Option<String>,
    item: String,
    slug: String,
    variant: String,
    selling: bool,
    platinum: u32,
    quantity: u32,
    market: tauri::State<'_, market::Gateway>,
    history: tauri::State<'_, Arc<account::Store>>,
    tx: tauri::State<'_, InputTx>,
) -> Result<(), String> {
    let started = std::time::Instant::now();
    let made = account::Made {
        item,
        slug,
        variant,
        selling,
        platinum,
        quantity,
        origin: Origin::Hand.word().to_owned(),
        own: order.clone(),
    };
    if history
        .recorded_lately(made.clone())
        .await
        .map_err(|e| e.to_string())?
    {
        info!(order_id = order, "trade already recorded");
        return Ok(());
    }

    // warframe.market is updated first, so a refusal there leaves no partial local write to undo.
    if let Some(id) = &order {
        let standing = history
            .standing_orders()
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|standing| &standing.id == id);

        let closed = match &standing {
            Some(standing) => {
                market::close_own_order(
                    &market,
                    Priority::Person,
                    &history,
                    standing,
                    quantity,
                    platinum,
                )
                .await
            }
            None => {
                let session = session(&market)?;
                market
                    .call(Priority::Person, |pass| session.close(pass, id, quantity))
                    .await
            }
        };
        refused(closed, &tx).await?;
    }

    history
        .record_exchange(made)
        .await
        .map_err(|e| e.to_string())?;
    info!(
        took_ms = started.elapsed().as_millis() as u64,
        "manual trade recorded"
    );
    Ok(())
}

/// Returns the closed trades of the last `days` days (`0` for all) with their margins, newest
/// first.
#[tauri::command]
#[specta::specta]
pub async fn get_figures(
    lang: String,
    days: u32,
    catalog: tauri::State<'_, Arc<Catalog>>,
    history: tauri::State<'_, Arc<account::Store>>,
) -> Result<Figures, String> {
    figures::figures(&catalog, &history, &lang, days).await
}

/// Returns the agent's trading results over the last `days` days.
#[tauri::command]
#[specta::specta]
pub async fn get_scorecard(
    days: u32,
    history: tauri::State<'_, Arc<account::Store>>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
) -> Result<Scorecard, String> {
    agent::scorecard_over(&history, &market_cache, days).await
}

/// Withdraws the orders `ids`, one market call each, and returns how many are gone; an order
/// the market no longer lists counts as gone. A refusal does not stop the rest, but the first
/// refusal is returned as `Err`.
#[tauri::command]
#[specta::specta]
pub async fn withdraw_orders(
    ids: Vec<String>,
    market: tauri::State<'_, market::Gateway>,
    history: tauri::State<'_, Arc<account::Store>>,
    tx: tauri::State<'_, InputTx>,
) -> Result<u32, String> {
    let session = session(&market)?;
    let mut gone = Vec::with_capacity(ids.len());
    let mut refusal = None;

    for id in ids {
        let withdrawn = market
            .call(Priority::Person, |pass| session.withdraw(pass, &id))
            .await;
        match withdrawn {
            Ok(()) => gone.push(id),
            // The market no longer lists it, which is the requested outcome.
            Err(MarketError::NotListed(_)) => gone.push(id),
            // One refusal does not stop the rest; withdrawn orders are still removed locally.
            Err(why) => {
                refusal.get_or_insert(why);
            }
        }
    }

    let took = gone.len() as u32;
    if !gone.is_empty() {
        history.drop_orders(gone).await.map_err(|e| e.to_string())?;
    }
    match refusal {
        Some(why) => refused(Err::<u32, _>(why), &tx).await,
        None => Ok(took),
    }
}

/// Shows or hides every sell (`selling`) or buy order of the user's own, not the agent's;
/// returns how many orders changed.
#[tauri::command]
#[specta::specta]
pub async fn show_orders(
    selling: bool,
    visible: bool,
    market: tauri::State<'_, market::Gateway>,
    market_cache: tauri::State<'_, Arc<market::Store>>,
    history: tauri::State<'_, Arc<account::Store>>,
    tx: tauri::State<'_, InputTx>,
) -> Result<u32, String> {
    let standing = history.standing_orders().await.map_err(|e| e.to_string())?;
    let mut touched = 0;
    for order in standing
        .iter()
        .filter(|order| order.selling == selling && !order.by_agent && order.visible != visible)
    {
        let change = serde_json::json!({ "visible": visible });
        amended(&order.id, change, &market, &market_cache, &history, &tx).await?;
        touched += 1;
    }
    Ok(touched)
}

/// Sends one order change to warframe.market, saves the returned order to the account store
/// and returns it.
async fn amended(
    id: &str,
    change: serde_json::Value,
    market: &market::Gateway,
    market_cache: &Arc<market::Store>,
    history: &Arc<account::Store>,
    tx: &tauri::State<'_, InputTx>,
) -> Result<account::Standing, String> {
    let session = session(market)?;
    let changed = refused(
        market
            .call(Priority::Person, |pass| session.amend(pass, id, change))
            .await,
        tx,
    )
    .await?;

    let slug = match &changed.item_id {
        Some(id) => market_cache
            .slug_of(id.clone())
            .await
            .map_err(|e| e.to_string())?,
        None => None,
    };
    let variant = variant_of(&changed);
    let kept = (changed, slug, variant);
    let standing = account::Standing {
        id: kept.0.id.clone(),
        item_id: kept.0.item_id.clone().unwrap_or_default(),
        slug: kept.1.clone(),
        variant: kept.2.clone(),
        selling: kept.0.side == median_market::Side::Sell,
        platinum: kept.0.platinum,
        quantity: kept.0.quantity,
        per_trade: kept.0.per_trade.unwrap_or(1),
        visible: kept.0.visible,
        by_agent: false,
        promised: None,
        created_at: kept.0.created_at.clone(),
        updated_at: kept.0.updated_at.clone(),
    };
    history
        .keep_one_order(kept)
        .await
        .map_err(|e| e.to_string())?;
    Ok(standing)
}

/// Session resumed from the saved token; `Err` when there is no saved token.
fn session(market: &market::Gateway) -> Result<Session, String> {
    let authorization = Token::read().ok_or_else(|| "no session".to_owned())?;
    Ok(Session::resume(
        Arc::clone(market.client()),
        authorization,
        Token::name(),
    ))
}

/// Converts a market error to text; on `Unauthorized` also clears the token and sends
/// `MembershipEvent::Refused` to the engine.
async fn refused<T>(
    answer: Result<T, MarketError>,
    tx: &tauri::State<'_, InputTx>,
) -> Result<T, String> {
    match answer {
        Ok(value) => Ok(value),
        Err(MarketError::Unauthorized) => {
            Token::clear();
            say(tx, MembershipEvent::Refused).await?;
            Err("the session expired".to_owned())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Variant key of an own order in the market cache's notation.
fn variant_of(order: &Mine) -> String {
    market::Variant::of_mine(order).key()
}

/// Shows the main window, which is created hidden, and closes the launcher; the main window
/// calls this once its page has rendered.
#[tauri::command]
#[specta::specta]
pub async fn window_ready(app: AppHandle) {
    window::main_rendered(&app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use median_core::{SwapKind, SwapPart};

    const BLUEPRINT: &str = "/Lotus/Types/Recipes/Weapons/AkariusPrimeBlueprint";
    const BARREL: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/AkariusPrimeBarrel";
    const LINK: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/AkariusPrimeLink";
    const RECEIVER: &str = "/Lotus/Types/Recipes/Weapons/WeaponParts/AkariusPrimeReceiver";

    fn told() -> HashMap<String, crate::catalog::Told> {
        [
            ("ЧЕРТЁЖ: Акариус Прайм", BLUEPRINT),
            ("Акариус Прайм: Ствол", BARREL),
            ("Акариус Прайм: Связь", LINK),
            ("Акариус Прайм: Приёмник", RECEIVER),
            ("Мистическая зарядка", "/Lotus/Upgrades/Enhancers/Energize"),
        ]
        .into_iter()
        .map(|(name, item)| {
            (
                median_core::fold(name),
                crate::catalog::Told {
                    fold: median_core::fold(name),
                    name: name.to_owned(),
                    item: item.to_owned(),
                },
            )
        })
        .collect()
    }

    fn kits() -> Vec<Kit> {
        vec![Kit {
            members: vec![
                (BLUEPRINT.to_owned(), 1),
                (BARREL.to_owned(), 1),
                (LINK.to_owned(), 1),
                (RECEIVER.to_owned(), 1),
            ],
            name: "Акариус Прайм: Комплект".to_owned(),
        }]
    }

    fn part(name: &str, count: u32) -> SwapPart {
        SwapPart {
            name: name.to_owned(),
            count,
            kind: SwapKind::Item,
            rank: None,
        }
    }

    fn read(parts: &[SwapPart]) -> Vec<(String, u32)> {
        parts
            .iter()
            .map(|part| (part.name.clone(), part.count))
            .collect()
    }

    #[test]
    fn the_parts_of_a_set_handed_over_together_read_as_the_set() {
        let side = vec![
            part("ЧЕРТЁЖ: Акариус Прайм", 1),
            part("Акариус Прайм: Приёмник", 1),
            part("Акариус Прайм: Ствол", 1),
            part("Акариус Прайм: Связь", 1),
        ];

        assert_eq!(
            read(&gathered(side, &told(), &kits())),
            vec![("Акариус Прайм: Комплект".to_owned(), 1)]
        );
    }

    #[test]
    fn what_the_set_did_not_take_is_still_there_to_see() {
        let side = vec![
            part("ЧЕРТЁЖ: Акариус Прайм", 1),
            part("Акариус Прайм: Приёмник", 1),
            part("Акариус Прайм: Приёмник", 1),
            part("Акариус Прайм: Ствол", 1),
            part("Акариус Прайм: Ствол", 1),
            part("Акариус Прайм: Связь", 1),
        ];

        assert_eq!(
            read(&gathered(side, &told(), &kits())),
            vec![
                ("Акариус Прайм: Комплект".to_owned(), 1),
                ("Акариус Прайм: Приёмник".to_owned(), 1),
                ("Акариус Прайм: Ствол".to_owned(), 1),
            ]
        );
    }

    #[test]
    fn a_side_short_of_a_part_is_left_as_the_game_wrote_it() {
        let side = vec![
            part("Акариус Прайм: Ствол", 1),
            part("Акариус Прайм: Связь", 1),
        ];

        assert_eq!(
            read(&gathered(side, &told(), &kits())),
            vec![
                ("Акариус Прайм: Ствол".to_owned(), 1),
                ("Акариус Прайм: Связь".to_owned(), 1),
            ]
        );
    }

    #[test]
    fn two_whole_sets_read_as_two() {
        let side = vec![
            part("ЧЕРТЁЖ: Акариус Прайм", 2),
            part("Акариус Прайм: Приёмник", 2),
            part("Акариус Прайм: Ствол", 2),
            part("Акариус Прайм: Связь", 2),
        ];

        assert_eq!(
            read(&gathered(side, &told(), &kits())),
            vec![("Акариус Прайм: Комплект".to_owned(), 2)]
        );
    }

    #[test]
    fn a_set_that_takes_two_of_a_part_eats_both() {
        let doubled = vec![Kit {
            members: vec![(BLUEPRINT.to_owned(), 1), (BARREL.to_owned(), 2)],
            name: "Акариус Прайм: Комплект".to_owned(),
        }];
        let side = vec![
            part("ЧЕРТЁЖ: Акариус Прайм", 1),
            part("Акариус Прайм: Ствол", 3),
        ];

        assert_eq!(
            read(&gathered(side, &told(), &doubled)),
            vec![
                ("Акариус Прайм: Комплект".to_owned(), 1),
                ("Акариус Прайм: Ствол".to_owned(), 1),
            ]
        );
    }

    /// Platinum and items missing from the catalog pass through unchanged.
    #[test]
    fn what_is_not_a_part_is_not_gathered() {
        let side = vec![
            SwapPart {
                name: "Платина".to_owned(),
                count: 79,
                kind: SwapKind::Platinum,
                rank: None,
            },
            part("Мистическая зарядка", 1),
        ];

        assert_eq!(
            read(&gathered(side, &told(), &kits())),
            vec![
                ("Платина".to_owned(), 79),
                ("Мистическая зарядка".to_owned(), 1)
            ]
        );
    }
}
