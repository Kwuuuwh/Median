use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use median_core::{
    Ask, DAY, Fetch, Holder, Instant, Interest, Inventory, Kept, Need, ask_queue, seconds,
};
use median_ipc::{BookMoved, PricesChanged, QuotesChanged};
use median_market::{Book, ItemMeta, MarketError, Pass};
use tauri::AppHandle;
use tauri_specta::Event;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::Notify;
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::catalog::{self, Catalog};
use crate::market::ask::is_described;
use crate::market::books::Books;
use crate::market::flow::Stirred;
use crate::market::gateway::{Filler, Gateway, Priority};
use crate::market::needs::Needs;
use crate::market::prices::{Listed, listed, prices_of, set_price};
use crate::market::store::{BOOK, ITEM, LISTED, STATISTICS, Standing, Store};
use crate::market::variant::Variant;
use crate::state_sink::SharedHoldings;

/// Items with a newly fetched book, each with the variants of its latest book, queued until
/// taken.
#[derive(Clone, Default)]
pub struct Answers(Arc<AnswersInner>);

#[derive(Default)]
struct AnswersInner {
    waiting: Mutex<Waiting>,
    put: Notify,
}

/// Items in first-arrival order, with the latest variants of each.
#[derive(Default)]
struct Waiting {
    order: VecDeque<String>,
    latest: HashMap<String, Vec<Variant>>,
}

impl Answers {
    pub fn put(&self, slug: &str, looked: Vec<Variant>) {
        {
            let mut waiting = self
                .0
                .waiting
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if waiting.latest.insert(slug.to_owned(), looked).is_none() {
                waiting.order.push_back(slug.to_owned());
            }
        }
        self.0.put.notify_one();
    }

    /// Removes and returns the oldest queued item, waiting until there is one.
    pub async fn next(&self) -> (String, Vec<Variant>) {
        loop {
            {
                let mut waiting = self
                    .0
                    .waiting
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                if let Some(slug) = waiting.order.pop_front() {
                    let looked = waiting.latest.remove(&slug).unwrap_or_default();
                    return (slug, looked);
                }
            }
            self.0.put.notified().await;
        }
    }
}

/// Interval between declarations of the inventory watch's needs.
const DECLARE: Duration = Duration::from_secs(60);

/// Refresh interval of the market item list.
const LIST_KEEP: Instant = 7 * DAY;

/// Holdings that have a market price, and the sets they form.
#[derive(Default)]
struct InventoryWatch {
    holdings: Vec<Listed>,
    sets: BTreeSet<String>,
}

/// Scheduler that keeps the market cache fresh and passes every fetched book to the market
/// cache, the inventory watch, the agent and the window.
pub struct Freshener {
    app: AppHandle,
    gateway: Gateway,
    store: Arc<Store>,
    needs: Needs,
    stirred: Stirred,
    books: Books,
    answers: Answers,
    running: Arc<AsyncMutex<bool>>,
    kept: Mutex<HashMap<String, Kept>>,
    flying: Mutex<HashSet<(String, Fetch)>>,
    inventory_watch: RwLock<Arc<InventoryWatch>>,
    /// Items to describe ahead of any need: every item a relic drops, since the reward screen
    /// shows them without warning.
    ahead: RwLock<Arc<Vec<String>>>,
    /// Woken once the market item list is saved or found current.
    listed: Arc<Notify>,
}

impl Freshener {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        app: AppHandle,
        gateway: Gateway,
        store: Arc<Store>,
        needs: Needs,
        stirred: Stirred,
        books: Books,
        answers: Answers,
        running: Arc<AsyncMutex<bool>>,
        listed: Arc<Notify>,
    ) -> Result<Self, String> {
        let mut kept: HashMap<String, Kept> = HashMap::new();
        for (slug, at) in store.stamps(BOOK).await.map_err(|e| e.to_string())? {
            kept.entry(slug).or_default().book = Some(at);
        }
        for (slug, at) in store.stamps(STATISTICS).await.map_err(|e| e.to_string())? {
            kept.entry(slug).or_default().trades = Some(at);
        }
        for (slug, at) in store.stamps(ITEM).await.map_err(|e| e.to_string())? {
            kept.entry(slug).or_default().described = Some(at);
        }
        Ok(Self {
            app,
            gateway,
            store,
            needs,
            stirred,
            books,
            answers,
            running,
            kept: Mutex::new(kept),
            flying: Mutex::new(HashSet::new()),
            inventory_watch: RwLock::new(Arc::default()),
            ahead: RwLock::new(Arc::default()),
            listed,
        })
    }

    /// Fetches the book of one item at user priority; `of` is the variant the user is viewing.
    pub async fn book_now(&self, slug: &str, of: &Variant) -> Result<Book, String> {
        self.describe(Priority::Person, slug).await?;
        let book = self
            .gateway
            .call(Priority::Person, |pass| {
                self.gateway.client().book(pass, slug)
            })
            .await
            .map_err(|e| e.to_string())?;
        self.took(slug, &book, std::slice::from_ref(of)).await?;
        Ok(book)
    }

    /// Fetches the item's variants and lot rule unless the market cache has a current
    /// description.
    async fn describe(&self, priority: Priority, slug: &str) -> Result<(), String> {
        if self.described(slug).await {
            return Ok(());
        }
        super::describe(&self.gateway, &self.store, priority, slug).await
    }

    async fn described(&self, slug: &str) -> bool {
        matches!(
            (
                self.store.variants_asked(slug).await,
                self.store.lots_allowed(slug).await,
            ),
            (Ok(Some(_)), Ok(Some(_)))
        ) && is_described(&self.store, slug).await
    }

    /// Saves one book to the market cache and passes it to the inventory watch, the agent and
    /// the window.
    async fn took(&self, slug: &str, book: &Book, also: &[Variant]) -> Result<(), String> {
        let now = seconds();
        let needs = self.needs.all();
        let mut variants = self
            .store
            .variants_asked(slug)
            .await
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        let wanted = needs
            .iter()
            .filter(|need| need.slug == slug)
            .map(|need| Variant::parse(&need.variant))
            .chain(also.iter().cloned());
        for variant in wanted {
            if !variants.contains(&variant) {
                variants.push(variant);
            }
        }

        let looks = self
            .store
            .keep_look(slug, book, &variants)
            .await
            .map_err(|e| e.to_string())?;
        self.store
            .stamp(slug, BOOK)
            .await
            .map_err(|e| e.to_string())?;
        self.stirred.settle(slug);
        self.kept().entry(slug.to_owned()).or_default().book = Some(now);

        if traded(&needs, slug) {
            self.books.keep(slug, book.clone(), now);
            let _ = BookMoved {
                slug: slug.to_owned(),
            }
            .emit(&self.app);
        }
        if let Err(why) = self.tell(slug, now).await {
            warn!(error = %why, slug, "price update not sent to window");
        }
        let looked = looks.into_iter().map(|(variant, _)| variant).collect();
        self.answers.put(slug, looked);
        Ok(())
    }

    /// Sends the window the inventory watch's new prices for one item.
    async fn tell(&self, slug: &str, now: Instant) -> Result<(), String> {
        let watch = Arc::clone(
            &self
                .inventory_watch
                .read()
                .unwrap_or_else(PoisonError::into_inner),
        );
        let held = prices_of(&self.store, &watch.holdings, slug, now).await?;
        let sets = match watch.sets.contains(slug) {
            true => vec![set_price(&self.store, slug, now).await?],
            false => Vec::new(),
        };
        if !held.is_empty() || !sets.is_empty() {
            let _ = PricesChanged { held, sets }.emit(&self.app);
        }
        Ok(())
    }

    /// (items whose book is within the `Watched` freshness limit, all items).
    pub async fn warmth(&self, items: &[String]) -> (u32, u32) {
        let keeps = Interest::Watched.keeps(*self.running.lock().await);
        let now = seconds();
        let kept = self.kept();
        let warm = items
            .iter()
            .filter(|slug| {
                kept.get(slug.as_str())
                    .and_then(|known| known.book)
                    .is_some_and(|at| now - at <= keeps)
            })
            .count();
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        (count(warm), count(items.len()))
    }

    fn kept(&self) -> std::sync::MutexGuard<'_, HashMap<String, Kept>> {
        self.kept.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn flying(&self) -> std::sync::MutexGuard<'_, HashSet<(String, Fetch)>> {
        self.flying.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Requests due now, most urgent first, excluding those in flight.
    async fn asks(&self) -> Vec<Ask> {
        let agent_on = *self.running.lock().await;
        let needs = self.needs.all();
        let ahead = Arc::clone(&self.ahead.read().unwrap_or_else(PoisonError::into_inner));
        let asks = {
            let kept = self.kept();
            let known = |slug: &str| Kept {
                stirred: self.stirred.moved(slug),
                ..kept.get(slug).copied().unwrap_or_default()
            };
            ask_queue(&needs, &ahead, &known, agent_on, seconds())
        };
        let flying = self.flying();
        asks.into_iter()
            .filter(|ask| !flying.contains(&(ask.slug.clone(), ask.fetch)))
            .collect()
    }

    fn board(&self, ask: Ask) -> Ask {
        self.flying().insert((ask.slug.clone(), ask.fetch));
        ask
    }

    /// Makes the one market request for `ask`; a book request for an item without a current
    /// description fetches the description instead.
    async fn fetch(&self, ask: &Ask, pass: Pass) -> Result<Made, MarketError> {
        let client = self.gateway.client();
        let slug = ask.slug.as_str();
        match ask.fetch {
            Fetch::Description => {
                let meta = client.item(pass, slug).await?;
                self.keep_description(slug, &meta).await;
                return Ok(Made::Description);
            }
            Fetch::Book if !self.described(slug).await => {
                let meta = client.item(pass, slug).await?;
                self.keep_description(slug, &meta).await;
                return Ok(Made::Description);
            }
            Fetch::Book => {
                let book = client.book(pass, slug).await?;
                note(self.took(slug, &book, &[]).await, slug);
            }
            Fetch::Trades => {
                let statistics = client.statistics(pass, slug).await?;
                note(self.store.keep_closed(slug, &statistics).await, slug);
                let standing = statistics
                    .live_daily
                    .iter()
                    .filter_map(Standing::of_live)
                    .collect();
                note(
                    self.store
                        .keep_standing(vec![(slug.to_owned(), standing)])
                        .await,
                    slug,
                );
                note(self.store.stamp(slug, STATISTICS).await, slug);
                self.quotes_moved(slug);
            }
        }
        Ok(Made::Request)
    }

    /// Saves an item's description; tells the window when the item is ranked, since its prices
    /// are read by rank from then on.
    async fn keep_description(&self, slug: &str, meta: &ItemMeta) {
        note(self.store.keep_item(meta).await, slug);
        note(self.store.stamp(slug, ITEM).await, slug);
        if meta.max_rank.is_some() {
            self.quotes_moved(slug);
        }
    }

    /// Tells the window that prices built on the item's closed trades or description may differ.
    fn quotes_moved(&self, slug: &str) {
        let _ = QuotesChanged {
            slugs: Some(vec![slug.to_owned()]),
        }
        .emit(&self.app);
    }

    /// Records a request as made, whether or not it succeeded.
    fn asked(&self, ask: &Ask, made: Made) {
        self.flying().remove(&(ask.slug.clone(), ask.fetch));
        let mut kept = self.kept();
        record(
            kept.entry(ask.slug.clone()).or_default(),
            ask.fetch,
            made,
            seconds(),
        );
    }

    /// Every `DECLARE`, refreshes the item list when due, declares the inventory watch's needs and
    /// reads the items to describe ahead.
    pub async fn tend(
        self: Arc<Self>,
        catalog: Arc<Catalog>,
        holdings: SharedHoldings,
    ) -> Result<(), String> {
        loop {
            self.list().await;
            match drops(&catalog).await {
                Ok(slugs) => {
                    *self.ahead.write().unwrap_or_else(PoisonError::into_inner) = Arc::new(slugs);
                }
                Err(why) => warn!(error = %why, "relic drops to describe ahead not read"),
            }
            let held = holdings.read().inventory;
            match shown(&catalog, &self.store, &held).await {
                Ok(watch) => {
                    self.needs
                        .declare(Holder::Inventory, inventory_needs(&watch));
                    *self
                        .inventory_watch
                        .write()
                        .unwrap_or_else(PoisonError::into_inner) = Arc::new(watch);
                }
                Err(why) => warn!(error = %why, "inventory price watch list read failed"),
            }
            sleep(DECLARE).await;
        }
    }

    /// Fetches the market item list once the stored one is older than `LIST_KEEP`; wakes
    /// `listed` while a list is stored.
    async fn list(&self) {
        let asked = self.store.stamped("", LISTED).await.unwrap_or(None);
        if asked.is_some_and(|at| seconds() - at < LIST_KEEP) {
            self.listed.notify_one();
            return;
        }
        let listed = self
            .gateway
            .call(Priority::Background, |pass| {
                self.gateway.client().items(pass)
            })
            .await;
        match listed {
            Ok(listed) => match self.store.keep_listed(listed).await {
                Ok(count) => {
                    info!(items = count, "market item list read");
                    let _ = self.store.stamp("", LISTED).await;
                    self.listed.notify_one();
                }
                Err(e) => warn!(error = %e, "market item list not saved"),
            },
            Err(e) => warn!(error = %e, "market item list read failed"),
        }
    }
}

#[async_trait]
impl Filler for Freshener {
    type Work = Ask;

    async fn pressing(&self) -> Option<Ask> {
        let first = self.asks().await.into_iter().next()?;
        (first.waiting && first.overdue >= 1.0).then(|| self.board(first))
    }

    async fn pick(&self) -> Option<Ask> {
        let first = self.asks().await.into_iter().next()?;
        Some(self.board(first))
    }

    async fn run(&self, ask: Ask, pass: Pass) {
        let outcome = self.fetch(&ask, pass).await;
        self.asked(&ask, outcome.as_ref().map_or(Made::Request, |made| *made));
        match outcome {
            Ok(_) => {}
            Err(trouble) if self.gateway.client().gate().failing_since().is_some() => {
                debug!(slug = ask.slug, error = %trouble, "market answer unusable");
            }
            Err(trouble) => warn!(slug = ask.slug, error = %trouble, "market answer unusable"),
        }
    }
}

/// Whether a need marks the item as traded, so its full book is kept in memory.
fn traded(needs: &[Need], slug: &str) -> bool {
    needs.iter().any(|need| {
        need.slug == slug
            && matches!(
                need.interest,
                Interest::Committed | Interest::Glimpsed | Interest::Watched | Interest::Sampled
            )
    })
}

fn note<T, E: std::fmt::Display>(written: Result<T, E>, slug: &str) {
    if let Err(why) = written {
        warn!(error = %why, slug, "market answer not saved");
    }
}

/// Market slugs of every item a relic can drop, sorted.
async fn drops(catalog: &Catalog) -> Result<Vec<String>, String> {
    catalog
        .read(|connection| {
            let rewards: Vec<String> = catalog::reward_names(connection, "en")?
                .into_iter()
                .map(|(item, _)| item)
                .collect();
            let slugs: BTreeSet<String> = catalog::slugs(connection, &rewards)?
                .into_iter()
                .map(|(_, slug)| slug)
                .collect();
            Ok(slugs.into_iter().collect())
        })
        .await
        .map_err(|e| e.to_string())
}

/// Every holding that has a market price, and every set the holdings form.
async fn shown(
    catalog: &Catalog,
    store: &Store,
    held: &Inventory,
) -> Result<InventoryWatch, String> {
    let holdings = listed(catalog, store, held).await?;
    let items = held.items();
    let sets = catalog
        .read(move |connection| catalog::set_slugs(connection, &items))
        .await
        .map_err(|e| e.to_string())?;

    Ok(InventoryWatch {
        holdings,
        sets: sets.into_iter().collect(),
    })
}

fn inventory_needs(watch: &InventoryWatch) -> Vec<Need> {
    let pairs: BTreeSet<(String, String)> = watch
        .holdings
        .iter()
        .map(|holding| (holding.slug.clone(), holding.of.key()))
        .chain(watch.sets.iter().map(|slug| (slug.clone(), String::new())))
        .collect();
    pairs
        .into_iter()
        .map(|(slug, variant)| Need {
            slug,
            variant,
            interest: Interest::Held,
        })
        .collect()
}

/// What one market request was spent on.
#[derive(Debug, Clone, Copy)]
enum Made {
    /// The item's description, fetched in place of its book.
    Description,
    /// The request asked for, whether or not it succeeded.
    Request,
}

/// Marks `fetch` of an item as asked at `now` in `kept`; a description fetched in place of the
/// book leaves the book due.
fn record(kept: &mut Kept, fetch: Fetch, made: Made, now: Instant) {
    match (fetch, made) {
        (Fetch::Description, _) | (_, Made::Description) => kept.described = Some(now),
        (Fetch::Book, Made::Request) => kept.book = Some(kept.book.map_or(now, |at| at.max(now))),
        (Fetch::Trades, Made::Request) => kept.trades = Some(now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_in_place_of_a_book_leaves_the_book_due() {
        let mut kept = Kept::default();
        record(&mut kept, Fetch::Book, Made::Description, 100);
        assert_eq!(kept.book, None);
        assert_eq!(kept.described, Some(100));
        record(&mut kept, Fetch::Book, Made::Request, 101);
        assert_eq!(kept.book, Some(101));
    }

    #[test]
    fn a_description_asked_ahead_is_recorded_even_when_it_failed() {
        let mut kept = Kept::default();
        record(&mut kept, Fetch::Description, Made::Request, 100);
        assert_eq!(kept.described, Some(100));
        assert_eq!((kept.book, kept.trades), (None, None));
    }

    #[test]
    fn a_request_is_recorded_whatever_it_brought() {
        let mut kept = Kept::default();
        record(&mut kept, Fetch::Trades, Made::Request, 100);
        assert_eq!(kept.trades, Some(100));
        record(&mut kept, Fetch::Book, Made::Request, 90);
        record(&mut kept, Fetch::Book, Made::Request, 80);
        assert_eq!(kept.book, Some(90));
    }

    #[tokio::test]
    async fn answers_keep_the_latest_of_each_item_in_the_order_they_first_came() {
        let answers = Answers::default();
        answers.put("ash_prime_set", vec![]);
        answers.put("serration", vec![Variant::default()]);
        answers.put("ash_prime_set", vec![Variant::default()]);

        assert_eq!(
            answers.next().await,
            ("ash_prime_set".to_owned(), vec![Variant::default()])
        );
        assert_eq!(answers.next().await.0, "serration");
    }
}
