use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;

use median_core::World;
use median_engine::parse_worldstate;
use serde::Serialize;
use serde_json::{Value, json};

use crate::account;
use crate::catalog::{Catalog, Layout, adopt, card, coins, nodes, picture, sets, taxonomy};
use crate::commands::{build, goods_of, quotes_of};
use crate::market;
use crate::settings::SettingsStore;
use crate::state_sink::{SharedHoldings, Shown, to_holdings, to_ipc};
use crate::way::MasteryCache;
use crate::wiring::starting_state;

/// Languages the window can request.
const LANGS: [&str; 2] = ["ru", "en"];

/// Squad size the relic expected values of `getRelicsOf` are recorded for.
const SQUAD: u32 = 4;

/// Squad sizes the relics page offers: one to four players who open the same relic.
const SQUADS: [u32; 4] = [1, 2, 3, 4];

/// Periods the price chart offers, in days.
const CURVE_DAYS: [u32; 4] = [30, 90, 180, 365];

/// Periods of the trade figures and scorecard, in days: 1 for the agent's day, then the tab's
/// periods; 0 counts every exchange.
const FIGURE_DAYS: [u32; 4] = [1, 7, 30, 0];

/// Periods the item card's chart offers, in days.
const CARD_DAYS: [u32; 3] = [7, 30, 90];

/// Records the answers of the IPC commands for the data under `root` and an optional world-state
/// file: one JSON file per command in `out`, plus the pictures the answers reference.
pub async fn record(root: &Path, out: &Path, world: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let layout = Layout::create(root.join("data"))?;
    let catalog = Catalog::empty();
    catalog.install(adopt(&layout).ok_or("no catalog under the data folder")?);
    let history = account::Store::under(root)?;
    let market_cache = market::Store::under(root)?;
    let settings = SettingsStore::new(root.join("settings.json")).load();
    let policy = settings.policy.clone();
    let inventory = Arc::new(history.restore().await.unwrap_or_default());
    let profile = inventory.profile.clone();
    let counts = SharedHoldings::new(Arc::clone(&inventory)).counted();

    let mut calls = Calls::default();
    let mut state = starting_state(
        &catalog,
        &layout,
        settings,
        Arc::clone(&inventory),
        market::Token::standing(),
    );
    if let Some(world) = world {
        let (worldstate, shelves) = parse_worldstate(&fs::read_to_string(world)?)?;
        state.world = Arc::new(World {
            worldstate: Some(worldstate),
            shelves,
        });
    }
    let shown = to_ipc(&state);
    calls.note("getState", json!([]), &shown);
    // A recording stands for a local build, which takes no updates.
    calls.note("getBuild", json!([]), &build(false));
    calls.note(
        "getInventory",
        json!([]),
        &to_holdings(&Shown {
            step: 0,
            inventory: Arc::clone(&inventory),
        }),
    );
    calls.note(
        "getFavourites",
        json!([]),
        &history.favourites(profile).await?,
    );

    let prices = market::prices(&catalog, &market_cache, &inventory)
        .await
        .map_err(io::Error::other)?;
    calls.note("getPrices", json!([]), &prices);
    let set_prices = market::set_prices(&catalog, &market_cache, &inventory)
        .await
        .map_err(io::Error::other)?;
    calls.note("getSetPrices", json!([]), &set_prices);

    let mut items: BTreeSet<String> = inventory
        .holdings
        .iter()
        .map(|held| held.item.clone())
        .collect();
    items.extend(
        shown
            .shelves
            .iter()
            .flat_map(|shelf| shelf.offers.iter().map(|offer| offer.item.clone())),
    );
    let mut pictures = BTreeSet::new();
    let cache = MasteryCache::default();
    for lang in LANGS {
        let known = catalog
            .read(move |connection| sets(connection, lang))
            .await?;
        for set in &known {
            items.extend(set.members.iter().cloned());
            items.extend(set.built.iter().cloned());
        }
        calls.note("getSets", json!([lang]), &known);

        let grouped = catalog
            .read(move |connection| taxonomy(connection, lang))
            .await?;
        calls.note("getTaxonomy", json!([lang]), &grouped);

        for _ in 0..5 {
            let timer = std::time::Instant::now();
            let _ =
                market::own_orders(&catalog, &market_cache, &history, &inventory, lang, &policy)
                    .await;
            eprintln!("TIMING orders {:?}", timer.elapsed());
        }
        if std::env::var("MEDIAN_PROBE_ONLY").is_ok() {
            std::process::exit(0);
        }
        let own = market::own_orders(&catalog, &market_cache, &history, &inventory, lang, &policy)
            .await
            .map_err(io::Error::other)?;
        calls.note("getOrders", json!([lang]), &own);

        let standing = crate::syndicates::syndicates(
            &catalog,
            &market_cache,
            &SharedHoldings::new(Arc::clone(&inventory)),
            &market::Needs::default(),
            &policy,
            lang,
        )
        .await
        .map_err(io::Error::other)?;
        pictures.extend(
            standing
                .iter()
                .flat_map(|syndicate| &syndicate.offers)
                .filter_map(|offer| offer.image.clone()),
        );
        calls.note("getSyndicates", json!([lang]), &standing);

        let holdings = SharedHoldings::new(Arc::clone(&inventory));
        let mastery = crate::mastery::standing(&catalog, &cache, &holdings, lang)
            .await
            .map_err(io::Error::other)?;
        pictures.extend(
            mastery
                .masterable
                .iter()
                .filter_map(|item| item.image.clone()),
        );
        calls.note("getMastery", json!([lang]), &mastery);
        let roadmap =
            crate::roadmap::roadmap(&catalog, &cache, &market_cache, &holdings, &history, lang)
                .await
                .map_err(io::Error::other)?;
        pictures.extend(
            roadmap
                .total
                .lines
                .iter()
                .filter_map(|line| line.image.clone())
                .chain(roadmap.ready.iter().filter_map(|ready| ready.image.clone()))
                .chain(
                    roadmap
                        .planned
                        .iter()
                        .filter_map(|planned| planned.image.clone()),
                ),
        );
        calls.note("getRoadmap", json!([lang]), &roadmap);

        for days in FIGURE_DAYS {
            let figures = crate::figures::figures(&catalog, &history, lang, days)
                .await
                .map_err(io::Error::other)?;
            calls.note("getFigures", json!([lang, days]), &figures);
            let card = crate::agent::scorecard_over(&history, &market_cache, days)
                .await
                .map_err(io::Error::other)?;
            calls.note("getScorecard", json!([days]), &card);
        }

        let mut held: Vec<String> = counts.keys().cloned().collect();
        held.sort();
        for squad in SQUADS {
            let relics =
                crate::relics::priced(&catalog, &market_cache, &counts, held.clone(), lang, squad)
                    .await
                    .map_err(io::Error::other)?;
            calls.note("getRelics", json!([lang, squad]), &relics);
        }
    }
    for lang in LANGS {
        let wanted: Vec<String> = items.iter().cloned().collect();
        let goods = goods_of(&catalog, &inventory, lang.to_string(), wanted.clone())
            .await
            .map_err(io::Error::other)?;
        pictures.extend(goods.iter().filter_map(|good| good.image.clone()));
        calls.note("getGoods", json!([lang, wanted]), &goods);
    }

    let mut shelf_items: Vec<String> = shown
        .shelves
        .iter()
        .flat_map(|shelf| shelf.offers.iter().map(|offer| offer.item.clone()))
        .collect();
    shelf_items.sort();
    shelf_items.dedup();
    for lang in LANGS {
        let relics = crate::relics::priced(
            &catalog,
            &market_cache,
            &counts,
            shelf_items.clone(),
            lang,
            SQUAD,
        )
        .await
        .map_err(io::Error::other)?;
        pictures.extend(relics.iter().filter_map(|relic| relic.image.clone()));
        pictures.extend(
            relics
                .iter()
                .flat_map(|relic| relic.holds.iter().filter_map(|slot| slot.image.clone())),
        );
        calls.note("getRelicsOf", json!([lang, shelf_items, SQUAD]), &relics);
    }

    for lang in LANGS {
        let known = catalog
            .read(move |connection| coins(connection, lang))
            .await?;
        pictures.extend(known.iter().filter_map(|coin| coin.image.clone()));
        calls.note("getCoins", json!([lang]), &known);
    }

    let mut opened = shelf_items.clone();
    opened.extend(inventory.holdings.iter().map(|held| held.item.clone()));
    let masterables = cache
        .masterables(&catalog, "en")
        .await
        .map_err(io::Error::other)?;
    opened.extend(masterables.listed.iter().map(|item| item.item.clone()));
    let carded = card_items(&catalog, &opened).await?;
    for lang in LANGS {
        for item in &carded {
            let found = catalog
                .read({
                    let item = item.clone();
                    move |connection| card(connection, lang, &item)
                })
                .await?;
            if let Some(found) = &found {
                pictures.extend(found.good.image.iter().cloned());
                pictures.extend(
                    found
                        .inside
                        .iter()
                        .filter_map(|reward| reward.image.clone()),
                );
                pictures.extend(found.found.iter().filter_map(|reward| reward.image.clone()));
                pictures.extend(
                    found.recipes.iter().flat_map(|recipe| {
                        recipe.parts.iter().filter_map(|part| part.image.clone())
                    }),
                );
            }
            calls.note("getCard", json!([lang, item]), &found);
        }
    }

    let mut quoted: Vec<String> = shelf_items.iter().chain(&carded).cloned().collect();
    quoted.sort();
    quoted.dedup();
    let quotes = quotes_of(&catalog, &market_cache, quoted.clone())
        .await
        .map_err(io::Error::other)?;
    calls.note("getQuotes", json!([quoted]), &quotes);
    for quote in quotes.iter().filter(|quote| quote.platinum.is_some()) {
        for days in CARD_DAYS {
            for top_rank in [false, true] {
                let curve = market::curve(&catalog, &market_cache, &quote.item, "", days, top_rank)
                    .await
                    .map_err(io::Error::other)?;
                calls.note("getCurve", json!([quote.item, "", days, top_rank]), &curve);
            }
        }
    }

    let every_node = catalog.read(node_keys).await?;
    let places = catalog
        .read({
            let every_node = every_node.clone();
            move |connection| nodes(connection, &every_node)
        })
        .await?;
    calls.note("getNodes", json!([every_node]), &places);

    for price in prices.iter().filter(|price| price.asked.is_some()) {
        for days in CURVE_DAYS {
            let curve = market::curve(
                &catalog,
                &market_cache,
                &price.item,
                &price.variant,
                days,
                false,
            )
            .await
            .map_err(io::Error::other)?;
            calls.note(
                "getCurve",
                json!([price.item, price.variant, days, false]),
                &curve,
            );
        }
    }

    calls.write(out)?;
    write_pictures(&layout, &pictures, &out.join("pictures"))?;
    Ok(())
}

/// Recipe input class of parts that are built from their own blueprint.
const COMPONENT: &str = "component";

/// Items whose card can be opened starting from `shelf`: those items, their sets and set parts,
/// the relics that drop them and the contents they hold, their recipe blueprints and components,
/// and the blueprints of those.
async fn card_items(
    catalog: &Catalog,
    shelf: &[String],
) -> Result<BTreeSet<String>, Box<dyn Error>> {
    let mut items: BTreeSet<String> = shelf.iter().cloned().collect();
    let known = catalog
        .read(move |connection| sets(connection, "en"))
        .await?;
    for set in &known {
        let offered = set
            .built
            .iter()
            .chain(&set.members)
            .any(|item| items.contains(item));
        if offered {
            items.extend(set.built.iter().cloned());
            items.extend(set.members.iter().cloned());
        }
    }
    let mut more = BTreeSet::new();
    for item in &items {
        let found = catalog
            .read({
                let item = item.clone();
                move |connection| card(connection, "en", &item)
            })
            .await?;
        if let Some(found) = found {
            more.extend(found.found.iter().map(|reward| reward.item.clone()));
            more.extend(found.inside.iter().map(|reward| reward.item.clone()));
            for recipe in &found.recipes {
                more.insert(recipe.blueprint.clone());
                more.extend(
                    recipe
                        .parts
                        .iter()
                        .filter(|part| part.class == COMPONENT)
                        .map(|part| part.item.clone()),
                );
            }
        }
    }
    // A component is built from its own blueprint, which the component's card falls back on.
    let mut blueprints = BTreeSet::new();
    for item in &more {
        let found = catalog
            .read({
                let item = item.clone();
                move |connection| card(connection, "en", &item)
            })
            .await?;
        if let Some(found) = found {
            blueprints.extend(found.recipes.iter().map(|recipe| recipe.blueprint.clone()));
        }
    }
    items.extend(more);
    items.extend(blueprints);
    Ok(items)
}

/// Keys of every star chart node in the catalog's `regions` table.
fn node_keys(connection: &rusqlite::Connection) -> Result<Vec<String>, String> {
    let mut statement = connection
        .prepare("SELECT node FROM regions")
        .map_err(|e| e.to_string())?;
    statement
        .query_map([], |row| row.get(0))
        .and_then(Iterator::collect)
        .map_err(|e| e.to_string())
}

/// Recorded calls, by command.
#[derive(Default)]
struct Calls(BTreeMap<&'static str, Vec<Value>>);

impl Calls {
    fn note(&mut self, command: &'static str, args: Value, answer: &impl Serialize) {
        let answer = serde_json::to_value(answer).expect("an IPC answer serializes");
        self.0
            .entry(command)
            .or_default()
            .push(json!({ "args": args, "answer": answer }));
    }

    fn write(&self, out: &Path) -> io::Result<()> {
        fs::create_dir_all(out)?;
        for (command, calls) in &self.0 {
            let file = out.join(format!("{command}.json"));
            fs::write(file, serde_json::to_vec(calls)?)?;
        }
        Ok(())
    }
}

fn write_pictures(layout: &Layout, ids: &BTreeSet<String>, out: &Path) -> io::Result<()> {
    fs::create_dir_all(out)?;
    for id in ids {
        if let Some(bytes) = picture(layout, id) {
            fs::write(out.join(format!("{id}.webp")), bytes)?;
        }
    }
    Ok(())
}
