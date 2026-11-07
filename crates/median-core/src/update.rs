use std::sync::Arc;

use crate::collector::Collector;
use crate::domain::{Changed, Domain};
use crate::effect::Effect;
use crate::event::{Event, Progress};
use crate::game::{Cycling, Rewards};
use crate::membership::{Standing, Status};
use crate::state::{State, World};
use crate::watch::alarms;
use crate::{account, agent, catalog, collector, game, market, membership, tasks};

/// Applies `event` to `state`; returns the next state, the domains that changed and the effects
/// to run.
pub fn update(state: &State, event: Event) -> (State, Changed, Vec<Effect>) {
    let mut next = state.clone();
    let (changed, effects) = match event {
        Event::LocaleSet(locale) => {
            if locale == state.settings.locale {
                (Changed::NOTHING, Vec::new())
            } else {
                next.settings.locale = locale;
                persisted(&next)
            }
        }
        Event::AccentSet(accent) => {
            if accent == state.settings.accent {
                (Changed::NOTHING, Vec::new())
            } else {
                next.settings.accent = accent;
                persisted(&next)
            }
        }
        Event::WorldReceived {
            worldstate,
            shelves,
        } => {
            let rung = alarms(
                &state.settings.watches,
                state.world.worldstate.as_ref(),
                &worldstate,
            );
            let world = World {
                worldstate: Some(worldstate),
                shelves,
            };
            let effects = match rung.is_empty() {
                true => Vec::new(),
                false => vec![
                    Effect::Ring(rung),
                    Effect::Chime(state.settings.alert.volume),
                ],
            };
            (moved(&mut next.world, world, Domain::World), effects)
        }
        Event::InputQuarantined => {
            next.quarantined += 1;
            (Changed::of(Domain::Quarantine), Vec::new())
        }
        Event::Progressed(progress) => (progressed(&mut next, progress), Vec::new()),
        Event::Catalog(event) => {
            let (catalog, effects) = catalog::update(&state.catalog, event);
            (moved(&mut next.catalog, catalog, Domain::Catalog), effects)
        }
        Event::Collector(event) => {
            next.collector = collector::update(&state.collector, event);
            let changed = match next.collector == state.collector {
                true => Changed::NOTHING,
                false => Changed::of(Domain::Collector),
            };
            // Every envelope updates the collector state, so the market status is recomputed only
            // when the game starts or stops.
            let effects = match running(&state.collector) == running(&next.collector) {
                true => Vec::new(),
                false => shown(&next),
            };
            (changed, effects)
        }
        Event::TaskReported(report) => {
            let tasks = tasks::update(&state.tasks, report);
            (moved(&mut next.tasks, tasks, Domain::Tasks), Vec::new())
        }
        Event::Account(event) => {
            let (account, held, effects) = account::update(&state.account, event);
            next.account = account;
            let mut changed = match next.account == state.account {
                true => Changed::NOTHING,
                false => Changed::of(Domain::Account),
            };
            if let Some(held) = held {
                next.inventory = held;
                changed |= Changed::of(Domain::Inventory);
                // The dump reports the current purse, which already includes the trades counted
                // since the previous dump, so those counters restart from zero.
                if next.game.spent != 0 || next.game.platinum_moved != 0 {
                    next.game.spent = 0;
                    next.game.platinum_moved = 0;
                    changed |= Changed::of(Domain::Game);
                }
            }
            (changed, effects)
        }
        Event::Market(event) => {
            next.market = market::update(&state.market, event);
            let changed = match next.market == state.market {
                true => Changed::NOTHING,
                false => Changed::of(Domain::Market),
            };
            (changed, Vec::new())
        }
        Event::Membership(event) => {
            next.membership = membership::update(&state.membership, event);
            let changed = match next.membership == state.membership {
                true => Changed::NOTHING,
                false => Changed::of(Domain::Membership),
            };
            (changed, shown(&next))
        }
        Event::Agent(event) => {
            let effects = match event {
                agent::AgentEvent::Picked(_) => vec![Effect::Chime(state.settings.alert.volume)],
                _ => Vec::new(),
            };
            next.agent = agent::update(&state.agent, event);
            let changed = match next.agent == state.agent {
                true => Changed::NOTHING,
                false => Changed::of(Domain::Agent),
            };
            (changed, effects)
        }
        Event::WatchSet { watch, on } => {
            let held = state.settings.watches.contains(&watch);
            if held == on {
                (Changed::NOTHING, Vec::new())
            } else {
                match on {
                    true => next.settings.watches.push(watch),
                    false => next.settings.watches.retain(|each| *each != watch),
                }
                persisted(&next)
            }
        }
        Event::FollowGameSet(follow) => {
            if follow == state.settings.follow_game {
                (Changed::NOTHING, Vec::new())
            } else {
                next.settings.follow_game = follow;
                let (changed, mut effects) = persisted(&next);
                effects.extend(shown(&next));
                (changed | Changed::of(Domain::Membership), effects)
            }
        }
        Event::AlertSet(alert) => {
            if alert == state.settings.alert {
                (Changed::NOTHING, Vec::new())
            } else {
                next.settings.alert = alert;
                persisted(&next)
            }
        }
        Event::UpdateChannelSet(channel) => {
            if Some(channel) == state.settings.update_channel {
                (Changed::NOTHING, Vec::new())
            } else {
                next.settings.update_channel = Some(channel);
                persisted(&next)
            }
        }
        Event::Game(event) => {
            next.game = game::update(&state.game, event);
            let changed = match next.game == state.game {
                true => Changed::NOTHING,
                false => Changed::of(Domain::Game),
            };
            let mut effects = spent(&next);
            let standing =
                |game: &game::Game| game.rewards != Rewards::Gone || game.cycling != Cycling::Gone;
            let standing_now = standing(&next.game);
            if standing_now != standing(&state.game) {
                effects.push(Effect::ShowOverlay(standing_now));
            }
            (changed, effects)
        }
        Event::PolicySet(policy) => {
            if policy == state.settings.policy {
                (Changed::NOTHING, Vec::new())
            } else {
                next.settings.policy = policy;
                (
                    Changed::of(Domain::Policy),
                    vec![Effect::PersistSettings(next.settings.clone())],
                )
            }
        }
    };
    (next, changed, effects)
}

/// Effect that sets the market status to invisible once no daily trades are left, while signed
/// in: an order that cannot be traded today only wastes other players' time.
fn spent(next: &State) -> Vec<Effect> {
    let left = game::exchanges_left(next.inventory.purse.trades_left, &next.game);
    let hide = left == Some(0)
        && next.membership.status != Status::Invisible
        && matches!(next.membership.standing, Standing::In { .. });

    match hide {
        true => vec![Effect::ShowStatus(Status::Invisible)],
        false => Vec::new(),
    }
}

/// Effect that sets the market status the game state calls for; none when the status already
/// matches or does not follow the game.
fn shown(next: &State) -> Vec<Effect> {
    match membership::follows(
        &next.membership,
        next.settings.follow_game,
        running(&next.collector),
    ) {
        Some(status) => vec![Effect::ShowStatus(status)],
        None => Vec::new(),
    }
}

/// Whether the game is running, judged by the collector being live.
fn running(collector: &Collector) -> bool {
    matches!(collector, Collector::Live { .. })
}

/// Changed domains and the effect that saves the settings after a settings change.
fn persisted(next: &State) -> (Changed, Vec<Effect>) {
    (
        Changed::of(Domain::Settings),
        vec![Effect::PersistSettings(next.settings.clone())],
    )
}

/// Passes download progress to the domain doing the download; only `Catalog` uses it.
fn progressed(next: &mut State, progress: Progress) -> Changed {
    match progress.domain {
        Domain::Catalog => {
            let catalog = catalog::progressed(&next.catalog, progress.done, progress.total);
            moved(&mut next.catalog, catalog, Domain::Catalog)
        }
        Domain::Settings
        | Domain::World
        | Domain::Quarantine
        | Domain::Collector
        | Domain::Tasks
        | Domain::Account
        | Domain::Inventory
        | Domain::Market
        | Domain::Membership
        | Domain::Policy
        | Domain::Agent
        | Domain::Game => Changed::NOTHING,
    }
}

/// Stores `value` in `slot` when it differs; returns `domain` as changed, or nothing.
fn moved<T: PartialEq>(slot: &mut Arc<T>, value: T, domain: Domain) -> Changed {
    if **slot == value {
        return Changed::NOTHING;
    }
    *slot = Arc::new(value);
    Changed::of(domain)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::update;
    use crate::account::{AccountEvent, Holding, Inventory, Profile, Purse, Variant};
    use crate::catalog::{Activity, CatalogEvent};
    use crate::domain::{Changed, Domain};
    use crate::effect::Effect;
    use crate::event::{Event, Progress};
    use crate::game::{Cycling, GameEvent, Outcome, Rewards, Swap};
    use crate::membership::{MembershipEvent, Status};
    use crate::settings::UpdateChannel;
    use crate::state::State;
    use crate::trade::{Currency, Offer, Price, Shelf};
    use crate::watch::Watch;
    use crate::worldstate::{Vendor, Visit, Worldstate};

    fn worldstate(at: i64) -> Worldstate {
        Worldstate {
            at,
            cycles: Vec::new(),
            fissures: Vec::new(),
            deadlines: Vec::new(),
            visits: Vec::new(),
        }
    }

    fn received(at: i64) -> Event {
        Event::WorldReceived {
            worldstate: worldstate(at),
            shelves: Vec::new(),
        }
    }

    #[test]
    fn a_watch_is_taken_up_and_dropped_and_saved_each_time() {
        let watch = Watch::Arrival {
            vendor: Vendor::Baro,
        };
        let (on, _, effects) = update(
            &State::default(),
            Event::WatchSet {
                watch: watch.clone(),
                on: true,
            },
        );
        assert_eq!(on.settings.watches, std::slice::from_ref(&watch));
        assert!(matches!(effects.as_slice(), [Effect::PersistSettings(_)]));

        let (again, changed, _) = update(
            &on,
            Event::WatchSet {
                watch: watch.clone(),
                on: true,
            },
        );
        assert_eq!(again.settings.watches.len(), 1);
        assert!(changed.is_empty());

        let (off, _, _) = update(&on, Event::WatchSet { watch, on: false });
        assert!(off.settings.watches.is_empty());
    }

    #[test]
    fn a_chosen_channel_is_saved_once() {
        let (chosen, changed, effects) = update(
            &State::default(),
            Event::UpdateChannelSet(UpdateChannel::Preview),
        );
        assert_eq!(chosen.settings.update_channel, Some(UpdateChannel::Preview));
        assert!(!changed.is_empty());
        assert!(matches!(effects.as_slice(), [Effect::PersistSettings(_)]));

        let (_, again, effects) = update(&chosen, Event::UpdateChannelSet(UpdateChannel::Preview));
        assert!(again.is_empty());
        assert!(effects.is_empty());
    }

    #[test]
    fn a_world_that_sets_a_watch_off_rings_it() {
        let mut watching = State::default();
        watching.settings.watches.push(Watch::Arrival {
            vendor: Vendor::Baro,
        });
        let baro = |at: i64| {
            let mut world = worldstate(at);
            world.visits.push(Visit {
                vendor: Vendor::Baro,
                node: "SaturnHUB".into(),
                arrives: 120,
                departs: 500,
            });
            world
        };
        let (watching, _, _) = update(
            &watching,
            Event::WorldReceived {
                worldstate: baro(100),
                shelves: Vec::new(),
            },
        );

        let (_, _, effects) = update(
            &watching,
            Event::WorldReceived {
                worldstate: baro(130),
                shelves: Vec::new(),
            },
        );

        assert!(matches!(
            effects.as_slice(),
            [Effect::Ring(rung), Effect::Chime(_)] if rung.len() == 1
        ));
    }

    #[test]
    fn the_world_updates_without_effects() {
        let (next, changed, effects) = update(&State::default(), received(1_785_240_441));

        assert_eq!(next.world.worldstate, Some(worldstate(1_785_240_441)));
        assert_eq!(changed, Changed::of(Domain::World));
        assert!(effects.is_empty());
    }

    #[test]
    fn the_same_world_is_a_noop() {
        let (state, _, _) = update(&State::default(), received(1_785_240_441));

        let (next, changed, effects) = update(&state, received(1_785_240_441));

        assert_eq!(next, state);
        assert!(changed.is_empty());
        assert!(effects.is_empty());
    }

    #[test]
    fn a_fresh_shelf_lands_even_when_the_clocks_have_not_moved() {
        let (state, _, _) = update(&State::default(), received(1_785_240_441));
        let shelf = Shelf {
            vendor: Vendor::Baro,
            offers: vec![Offer {
                item: "/Lotus/Weapons/VoidTrader/PrismaGrakata".into(),
                prices: vec![Price {
                    currency: Currency::Ducats,
                    amount: 610,
                }],
            }],
        };

        let (next, changed, _) = update(
            &state,
            Event::WorldReceived {
                worldstate: worldstate(1_785_240_441),
                shelves: vec![shelf.clone()],
            },
        );

        assert_eq!(next.world.shelves, vec![shelf]);
        assert_eq!(changed, Changed::of(Domain::World));
    }

    #[test]
    fn the_layer_stands_from_the_screen_opening_until_it_goes() {
        let game = |rewards| Event::Game(GameEvent::Rewards(rewards));

        let (reading, _, effects) = update(&State::default(), game(Rewards::Reading));
        assert_eq!(effects, vec![Effect::ShowOverlay(true)]);

        let read = Rewards::Read {
            cards: Vec::new(),
            fresh: false,
        };
        let (read, _, effects) = update(&reading, game(read));
        assert!(effects.is_empty(), "{effects:?}");

        let (_, _, effects) = update(&read, game(Rewards::Gone));
        assert_eq!(effects, vec![Effect::ShowOverlay(false)]);
    }

    #[test]
    fn the_layer_stands_while_the_cycling_screen_is_open() {
        let game = |cycling| Event::Game(GameEvent::Cycling(cycling));
        let open = Cycling::Open {
            cards: Vec::new(),
            rolls: Vec::new(),
            reading: true,
            unread: None,
        };

        let (open, _, effects) = update(&State::default(), game(open));
        assert_eq!(effects, vec![Effect::ShowOverlay(true)]);

        let (_, _, effects) = update(&open, game(Cycling::Gone));
        assert_eq!(effects, vec![Effect::ShowOverlay(false)]);
    }

    #[test]
    fn quarantine_increments_the_counter() {
        let (next, changed, effects) = update(&State::default(), Event::InputQuarantined);

        assert_eq!(next.quarantined, 1);
        assert_eq!(changed, Changed::of(Domain::Quarantine));
        assert!(effects.is_empty());
    }

    #[test]
    fn a_catalog_event_reaches_the_catalog() {
        let (next, changed, effects) = update(
            &State::default(),
            Event::Catalog(CatalogEvent::SyncRequested),
        );

        assert_eq!(next.catalog.activity, Activity::Checking);
        assert_eq!(changed, Changed::of(Domain::Catalog));
        assert_eq!(effects, vec![Effect::CheckCatalog]);
    }

    #[test]
    fn progress_of_a_domain_that_is_doing_nothing_is_a_noop() {
        let state = State::default();

        let (next, changed, _) = update(
            &state,
            Event::Progressed(Progress {
                domain: Domain::Catalog,
                done: 2048,
                total: 4096,
            }),
        );

        assert_eq!(next, state);
        assert!(changed.is_empty());
    }

    #[test]
    fn an_untouched_domain_keeps_the_slice_it_had() {
        let (state, _, _) = update(&State::default(), received(1_785_240_441));

        let (next, _, _) = update(&state, Event::InputQuarantined);

        assert!(Arc::ptr_eq(&state.world, &next.world));
    }

    /// An inventory reading with one item.
    fn reading(count: u32) -> Arc<Inventory> {
        Arc::new(Inventory {
            profile: Some(Profile {
                created: 1,
                name: "Tenno".into(),
            }),
            holdings: vec![Holding {
                item: "/Lotus/Types/Items/MiscItems/Rubedo".into(),
                variant: Variant::default(),
                detail: None,
                count,
            }],
            synced: None,
            purse: Purse::default(),
            ..Inventory::default()
        })
    }

    #[test]
    fn a_dump_moves_the_holdings_and_says_so() {
        let held = reading(1);

        let (next, changed, _) = update(
            &State::default(),
            Event::Account(AccountEvent::Read(Arc::clone(&held))),
        );

        assert_eq!(next.inventory, held);
        assert!(changed.has(Domain::Inventory));
        assert!(changed.has(Domain::Account));
    }

    #[test]
    fn a_dump_that_holds_the_same_leaves_the_slice_where_it_was() {
        let (state, _, _) = update(
            &State::default(),
            Event::Account(AccountEvent::Read(reading(1))),
        );

        let (next, changed, _) = update(&state, Event::Account(AccountEvent::Read(reading(1))));

        assert!(Arc::ptr_eq(&state.inventory, &next.inventory));
        assert!(!changed.has(Domain::Inventory));
    }
    #[test]
    fn following_the_game_is_saved_and_shows_the_account_in_game_while_it_runs() {
        let signed = update(
            &State::default(),
            Event::Membership(MembershipEvent::SignedIn {
                name: Some("Tenno".into()),
            }),
        )
        .0;
        let running = State {
            collector: crate::collector::Collector::Live {
                heard_at: 10,
                feeds: crate::collector::Feeds::Waiting,
            },
            ..signed
        };

        let (next, changed, effects) = update(&running, Event::FollowGameSet(true));

        assert!(next.settings.follow_game);
        assert!(changed.has(Domain::Settings) && changed.has(Domain::Membership));
        assert!(matches!(effects.as_slice(), [
            Effect::PersistSettings(saved),
            Effect::ShowStatus(Status::Ingame),
        ] if saved.follow_game));
    }

    #[test]
    fn the_last_exchange_of_the_day_hides_the_account() {
        let signed = update(
            &State::default(),
            Event::Membership(MembershipEvent::SignedIn {
                name: Some("Tenno".into()),
            }),
        )
        .0;
        let signed = update(
            &signed,
            Event::Membership(MembershipEvent::StatusSet(Status::Ingame)),
        )
        .0;
        let held = Arc::new(Inventory {
            purse: Purse {
                trades_left: Some(1),
                ..Purse::default()
            },
            ..Inventory::default()
        });
        let state = State {
            inventory: held,
            ..signed
        };

        let (next, _, effects) = update(
            &state,
            Event::Game(GameEvent::Traded(Box::new(Swap {
                at: 10,
                partner: "kafasi".into(),
                given: Vec::new(),
                taken: Vec::new(),
                outcome: Outcome::Done,
            }))),
        );

        assert_eq!(next.game.spent, 1);
        assert_eq!(effects, vec![Effect::ShowStatus(Status::Invisible)]);
    }
}
