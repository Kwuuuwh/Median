use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use median_core::{
    Account as CoreAccount, Catalog as CoreCatalog, CatalogEvent, Event, GameEvent, Inventory,
    Settings, State, seconds,
};
use median_engine::{Engine, Input, Ports};
use median_market::{Budget, Client as MarketClient, DayFiles, Gate, Host};
use tauri::{App, Manager};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::account;
use crate::agent;
use crate::catalog::{self, Catalog, Driver, Layout, Schedule, Source as CatalogSource, adopt};
use crate::collector;
use crate::commands::{Bought, InputTx};
use crate::effects::AppEffects;
use crate::gamelog;
use crate::gaps::Gaps;
use crate::layer::Layer;
use crate::market;
use crate::quarantine::FileQuarantine;
use crate::rivens;
use crate::schema::OpenError;
use crate::settings::SettingsStore;
use crate::state_sink::{Presence, SharedHoldings, SharedState, TauriStateSink, to_ipc};
use crate::tasks::{self, Restart};
use crate::way::MasteryCache;
use crate::worldstate;

/// Delay before a stopped long-lived task is restarted.
const RESTART: Duration = Duration::from_secs(5);

/// Self-imposed warframe.market request rate, in requests per second, evenly spaced.
const MARKET_RATE: f64 = 2.8;

/// Consecutive market request failures after which requests pause for the gate's cooldown.
const MARKET_FAILURES: u32 = 3;

/// Initial engine state: the installed catalog, the saved settings and the inventory last
/// saved in the account store.
pub(crate) fn starting_state(
    catalog: &Catalog,
    layout: &Layout,
    settings: Settings,
    inventory: Arc<Inventory>,
    standing: median_core::Standing,
) -> State {
    let account = match inventory.profile.clone() {
        Some(profile) => CoreAccount {
            profile: Some(profile),
            positions: inventory.holdings.len() as u32,
            digest: Some(inventory.digest()),
            ..CoreAccount::default()
        },
        None => CoreAccount::default(),
    };
    State {
        catalog: Arc::new(CoreCatalog {
            installed: catalog.version(),
            pack: catalog::version(layout),
            ..CoreCatalog::default()
        }),
        settings,
        account,
        inventory,
        membership: median_core::Membership {
            standing,
            ..median_core::Membership::default()
        },
        ..State::default()
    }
}

/// Wires the engine and its ports, then spawns the runtime tasks.
pub fn init(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();

    let root = handle.path().app_local_data_dir()?;
    market::Token::keep_under(&handle.config().identifier);
    std::fs::create_dir_all(&root).ok();
    if let Err(why) = crate::logfile::open(&root) {
        warn!(error = %why, "log file not opened, logging to stderr only");
    }

    let store = Arc::new(SettingsStore::new(root.join("settings.json")));
    let layout = Arc::new(Layout::create(root.join("data"))?);
    catalog::prune(&layout);

    let catalog = Arc::new(Catalog::empty());
    if let Some(open) = adopt(&layout) {
        catalog.install(open);
    }

    let none = || "none".to_owned();
    info!(
        revision = crate::revision::REVISION,
        hash = crate::revision::HASH,
        channel = crate::revision::CHANNEL,
        updates = crate::update::enabled(&handle),
        build = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        windows = crate::logfile::windows_version(),
        catalog = catalog
            .version()
            .map_or_else(none, |version| version.to_string()),
        pictures = catalog::version(&layout).map_or_else(none, |version| version.to_string()),
        collector_protocol = crate::collector::PROTOCOL,
        screen_capture = crate::screen::capture().map_or("none", crate::screen::Capture::as_code),
        "median started"
    );

    // Places the collector shipped with this build.
    let collector_folder = root.join("collector");
    let placed = handle
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())
        .and_then(|resources| collector::place(&resources.join("collector"), &collector_folder));
    match placed {
        Ok(collector::Placed::Replaced { was, now }) => info!(
            was = was.as_deref().unwrap_or("none"),
            now,
            folder = %collector_folder.display(),
            "collector placed"
        ),
        Ok(collector::Placed::Current(version)) => debug!(version, "collector current"),
        Ok(collector::Placed::Unshipped) => debug!("collector not shipped with this build"),
        Err(why) => warn!(error = %why, "collector not placed, overwolf keeps its copy"),
    }
    app.manage(collector::CollectorFolder(collector_folder));
    app.manage(crate::commands::DataFolder(root.clone()));

    let history = match account::Store::under(&root) {
        Ok(history) => Arc::new(history),
        Err(account::StoreError::Open(OpenError::TooNew { found, version })) => {
            error!(
                version = found,
                build_version = version,
                "account data written by a later build, median stops"
            );
            refuse_later_data(store.load().locale.as_code());
            std::process::exit(1);
        }
        Err(why) => return Err(why.into()),
    };
    let kept = market::Token::read();
    let market_cache = Arc::new(market::Store::under(&root)?);
    let started = Instant::now();
    let inventory = Arc::new(tauri::async_runtime::block_on(history.restore()).unwrap_or_default());

    let initial = starting_state(
        &catalog,
        &layout,
        store.load(),
        Arc::clone(&inventory),
        market::Token::standing(),
    );
    let data_bare = initial.catalog.is_bare();
    let account = &initial.account;

    info!(
        took_us = started.elapsed().as_micros() as u64,
        positions = account.positions,
        profile = account
            .profile
            .as_ref()
            .map(|profile| profile.name.as_str())
            .unwrap_or("none"),
        digest = account
            .digest
            .map(|digest| digest.short())
            .unwrap_or_default(),
        "account restored"
    );

    let shared = SharedState(Arc::new(RwLock::new(to_ipc(&initial))));
    let presence = Presence::default();
    let holdings = SharedHoldings::new(inventory);
    let checks = shared.clone();
    app.manage(shared.clone());
    app.manage(holdings.clone());
    app.manage(Arc::clone(&layout));
    app.manage(Arc::clone(&catalog));
    app.manage(Arc::new(MasteryCache::default()));
    app.manage(Arc::new(Gaps::new(root.join("gaps.log"))));
    app.manage(Arc::clone(&market_cache));
    app.manage(Arc::clone(&history));

    let (tx, rx) = mpsc::channel::<Input>(64);
    app.manage(InputTx(tx.clone()));

    let needs = market::Needs::default();
    app.manage(needs.clone());

    let stirred = market::Stirred::default();
    let books = market::Books::default();
    let heat = market::Heat::default();
    let answers = market::Answers::default();
    let bought = Arc::new(tokio::sync::Notify::new());
    app.manage(Bought(Arc::clone(&bought)));

    let (stream, asks) = market::stream::channel();
    let asks = Arc::new(tokio::sync::Mutex::new(asks));
    let bearer = Arc::new(tokio::sync::Mutex::new(kept));
    let running = Arc::new(tokio::sync::Mutex::new(false));
    let (flow, carried) = mpsc::channel::<median_market::Order>(256);
    let carried = Arc::new(tokio::sync::Mutex::new(carried));
    let (judging, orders) = mpsc::channel::<median_market::Order>(256);
    let orders = Arc::new(tokio::sync::Mutex::new(orders));
    app.manage(stream.clone());
    app.manage(Arc::clone(&running));

    let driver = Arc::new(Driver::new(
        CatalogSource::default(),
        layout,
        Arc::clone(&catalog),
        tx.clone(),
    ));

    let layer = Layer::new(app.handle().clone());
    layer.watch({
        let tx = tx.clone();
        move |running| {
            let event = match running {
                true => GameEvent::Started { at: seconds() },
                false => GameEvent::Stopped,
            };
            info!(running, "game window state changed");
            // The watcher runs on its own thread, outside the async runtime.
            let _ = tx.blocking_send(Input::Command(Event::Game(event)));
        }
    });

    let ports = Ports {
        quarantine: Arc::new(FileQuarantine::new(root.join("quarantine.log"))),
        effects: Arc::new(AppEffects::new(
            store,
            driver,
            Arc::clone(&history),
            stream.clone(),
            layer,
            app.handle().clone(),
        )),
        sink: Arc::new(TauriStateSink::new(
            handle,
            shared.clone(),
            holdings.clone(),
            presence.clone(),
        )),
    };

    let mut engine = Some((Engine::with_state(initial, ports), rx));
    tasks::spawn("engine", tx.clone(), Restart::Never, move || {
        let started = engine.take();
        async move {
            if let Some((engine, rx)) = started {
                engine.run(rx).await;
            }
            Ok(())
        }
    });

    tasks::spawn("worldstate", tx.clone(), Restart::After(RESTART), {
        let tx = tx.clone();
        let source = worldstate::Source::default();
        move || worldstate::poll(tx.clone(), source.clone())
    });

    tasks::spawn("catalog", tx.clone(), Restart::After(RESTART), {
        let tx = tx.clone();
        let schedule = Schedule::default();
        move || catalog::watch(tx.clone(), checks.clone(), schedule)
    });

    let dumps = Arc::new(account::Reader::new(
        tx.clone(),
        Arc::new(account::Journal::under(&root)),
    ));

    tasks::spawn("collector", tx.clone(), Restart::After(RESTART), {
        let tx = tx.clone();
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, collector::PORT));
        move || collector::serve(tx.clone(), Arc::clone(&dumps), address)
    });

    let (gate, doorman) = Gate::open(
        Budget {
            per_second: MARKET_RATE,
            burst: MARKET_RATE,
            failures_before_open: MARKET_FAILURES,
            cooldown: Duration::from_secs(60),
        },
        Instant::now(),
    );
    let doorman = Arc::new(doorman);
    let market = market::Gateway::new(Arc::new(MarketClient::new(
        gate,
        Host::default(),
        crate::http::USER_AGENT,
    )?));
    app.manage(market.clone());
    let slices = Arc::new(rivens::Slices::new(Arc::clone(&market_cache)));
    app.manage(Arc::clone(&slices));
    let weekly = Arc::new(rivens::Weekly::default());
    app.manage(Arc::clone(&weekly));
    let sheet = Arc::new(rivens::Sheet::default());
    app.manage(Arc::clone(&sheet));
    let fresh = Arc::new(market::Fresh::default());

    let freshener = Arc::new(tauri::async_runtime::block_on(market::Freshener::new(
        app.handle().clone(),
        market.clone(),
        Arc::clone(&market_cache),
        needs.clone(),
        stirred.clone(),
        books.clone(),
        answers.clone(),
        Arc::clone(&running),
    ))?);
    app.manage(Arc::clone(&freshener));

    let day_files = Arc::new(DayFiles::new(crate::http::USER_AGENT)?);

    tasks::spawn("days", tx.clone(), Restart::After(RESTART), {
        let market_cache = Arc::clone(&market_cache);
        let day_files = Arc::clone(&day_files);
        move || market::tend_days(Arc::clone(&market_cache), Arc::clone(&day_files))
    });

    tasks::spawn("heat", tx.clone(), Restart::After(RESTART), {
        let market_cache = Arc::clone(&market_cache);
        let heat = heat.clone();
        let shared = shared.clone();
        let needs = needs.clone();
        move || {
            market::tend_heat(
                Arc::clone(&market_cache),
                heat.clone(),
                shared.clone(),
                needs.clone(),
            )
        }
    });

    tasks::spawn("gateway", tx.clone(), Restart::After(RESTART), {
        let gateway = market.clone();
        let doorman = Arc::clone(&doorman);
        let freshener = Arc::clone(&freshener);
        move || {
            market::tend_gateway(
                gateway.clone(),
                Arc::clone(&doorman),
                Arc::clone(&freshener),
            )
        }
    });

    tasks::spawn("inventory-prices", tx.clone(), Restart::After(RESTART), {
        let freshener = Arc::clone(&freshener);
        let catalog = Arc::clone(&catalog);
        let holdings = holdings.clone();
        move || Arc::clone(&freshener).tend(Arc::clone(&catalog), holdings.clone())
    });

    tasks::spawn("riven-trades", tx.clone(), Restart::After(RESTART), {
        let weekly = Arc::clone(&weekly);
        move || rivens::tend_weekly(Arc::clone(&weekly))
    });

    tasks::spawn("riven-sweeps", tx.clone(), Restart::After(RESTART), {
        let catalog = Arc::clone(&catalog);
        let gateway = market.clone();
        let slices = Arc::clone(&slices);
        let holdings = holdings.clone();
        move || {
            rivens::tend_sweeps(
                Arc::clone(&catalog),
                gateway.clone(),
                Arc::clone(&slices),
                holdings.clone(),
            )
        }
    });

    tasks::spawn("good-rolls", tx.clone(), Restart::After(RESTART), {
        let sheet = Arc::clone(&sheet);
        let cache = root.join("good-rolls");
        move || rivens::tend_sheet(Arc::clone(&sheet), cache.clone())
    });

    tasks::spawn("riven-feed", tx.clone(), Restart::After(RESTART), {
        let gateway = market.clone();
        let market_cache = Arc::clone(&market_cache);
        let fresh = Arc::clone(&fresh);
        move || {
            market::tend_feed(
                gateway.clone(),
                Arc::clone(&market_cache),
                Arc::clone(&fresh),
            )
        }
    });

    tasks::spawn("market-health", tx.clone(), Restart::After(RESTART), {
        let client = Arc::clone(market.client());
        let tx = tx.clone();
        move || market::tend_health(Arc::clone(&client), tx.clone())
    });

    tasks::spawn("orders", tx.clone(), Restart::After(RESTART), {
        let gateway = market.clone();
        let market_cache = Arc::clone(&market_cache);
        let history = Arc::clone(&history);
        let needs = needs.clone();
        let tx = tx.clone();
        move || {
            market::tend_own_orders(
                gateway.clone(),
                Arc::clone(&market_cache),
                Arc::clone(&history),
                needs.clone(),
                tx.clone(),
            )
        }
    });

    tasks::spawn("stream", tx.clone(), Restart::After(RESTART), {
        let tx = tx.clone();
        let asks = Arc::clone(&asks);
        let bearer = Arc::clone(&bearer);
        let flow = flow.clone();
        move || {
            market::stream::tend(
                tx.clone(),
                Arc::clone(&asks),
                Arc::clone(&bearer),
                flow.clone(),
            )
        }
    });

    tasks::spawn("flow", tx.clone(), Restart::After(RESTART), {
        let market_cache = Arc::clone(&market_cache);
        let carried = Arc::clone(&carried);
        let judging = judging.clone();
        let stirred = stirred.clone();
        move || {
            market::tend_flow(
                Arc::clone(&market_cache),
                Arc::clone(&carried),
                judging.clone(),
                stirred.clone(),
            )
        }
    });

    let gear = Arc::new(agent::Gear::new(
        tx.clone(),
        shared.clone(),
        presence.clone(),
        Arc::clone(&market_cache),
        Arc::clone(&catalog),
        Arc::clone(&history),
        market.clone(),
        holdings.clone(),
        needs.clone(),
        books.clone(),
    ));

    tasks::spawn("agent", tx.clone(), Restart::After(RESTART), {
        let gear = Arc::clone(&gear);
        let orders = Arc::clone(&orders);
        let running = Arc::clone(&running);
        move || agent::tend_socket(Arc::clone(&gear), Arc::clone(&orders), Arc::clone(&running))
    });

    tasks::spawn("answers", tx.clone(), Restart::After(RESTART), {
        let gear = Arc::clone(&gear);
        let answers = answers.clone();
        let freshener = Arc::clone(&freshener);
        let running = Arc::clone(&running);
        let heat = heat.clone();
        move || {
            agent::tend_answers(
                Arc::clone(&gear),
                answers.clone(),
                Arc::clone(&freshener),
                Arc::clone(&running),
                heat.clone(),
            )
        }
    });

    tasks::spawn("prowl", tx.clone(), Restart::After(RESTART), {
        let gear = Arc::clone(&gear);
        let running = Arc::clone(&running);
        move || agent::tend_prowl(Arc::clone(&gear), Arc::clone(&running))
    });

    tasks::spawn("riven-picker", tx.clone(), Restart::After(RESTART), {
        let sources = Arc::new(agent::RivenSources {
            gear: Arc::clone(&gear),
            slices: Arc::clone(&slices),
            weekly: Arc::clone(&weekly),
            fresh: Arc::clone(&fresh),
        });
        let running = Arc::clone(&running);
        move || agent::tend_rivens(Arc::clone(&sources), Arc::clone(&running))
    });

    tasks::spawn("bids", tx.clone(), Restart::After(RESTART), {
        let gear = Arc::clone(&gear);
        let running = Arc::clone(&running);
        move || agent::tend_bids(Arc::clone(&gear), Arc::clone(&running))
    });

    tasks::spawn("scorecard", tx.clone(), Restart::After(RESTART), {
        let gear = Arc::clone(&gear);
        move || agent::tend_scorecard(Arc::clone(&gear))
    });

    tasks::spawn("presence", tx.clone(), Restart::After(RESTART), {
        let presence = presence.clone();
        let history = Arc::clone(&history);
        move || account::tend_presence(presence.clone(), Arc::clone(&history))
    });

    tasks::spawn("gamelog", tx.clone(), Restart::After(RESTART), {
        let tx = tx.clone();
        let shared = shared.clone();
        let history = Arc::clone(&history);
        let catalog = Arc::clone(&catalog);
        let gateway = market.clone();
        let bought = Arc::clone(&bought);
        move || {
            gamelog::tend(
                tx.clone(),
                shared.clone(),
                gamelog::GameLogContext {
                    history: Arc::clone(&history),
                    catalog: Arc::clone(&catalog),
                    gateway: gateway.clone(),
                    bought: Arc::clone(&bought),
                    market_cache: Arc::clone(&market_cache),
                    holdings: holdings.clone(),
                    needs: needs.clone(),
                },
            )
        }
    });

    tasks::spawn("offers", tx.clone(), Restart::After(RESTART), {
        let gear = Arc::clone(&gear);
        let running = Arc::clone(&running);
        let bought = Arc::clone(&bought);
        move || agent::tend_offers(Arc::clone(&gear), Arc::clone(&running), Arc::clone(&bought))
    });

    tauri::async_runtime::spawn(async move {
        let _ = tx
            .send(Input::Command(Event::Catalog(CatalogEvent::SyncRequested)))
            .await;
    });

    crate::window::start(app.handle(), !data_bare);
    Ok(())
}

/// Tells the person, in a system message box in `locale`, that the account data comes from a
/// later build; returns when the box is closed.
fn refuse_later_data(locale: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};

    let said = match locale.starts_with("ru") {
        true => "Данные median записаны более новой версией.",
        false => "median data was written by a newer version.",
    };
    let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (text, title) = (wide(said), wide("Median"));
    // SAFETY: both buffers are NUL-terminated UTF-16 that outlive the call, and a null owner
    // window is allowed.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
