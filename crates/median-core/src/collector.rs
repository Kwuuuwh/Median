use crate::time::Instant;

/// Connection state of the collector.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Collector {
    /// No envelope received within the server's timeout.
    #[default]
    Offline,
    /// Last envelope received at `heard_at`; `feeds` is what the collector last reported about
    /// Overwolf's game-event feeds.
    Live { heard_at: Instant, feeds: Feeds },
    /// The connected collector uses another protocol version, `speaks`.
    Outdated { speaks: u32 },
}

/// Overwolf's game-event feeds (GEP) for Warframe, as the collector reports them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Feeds {
    /// Not reported yet, or requested and not up yet: the feeds attach to the game some time after
    /// it starts.
    #[default]
    Waiting,
    /// Up: game data flows.
    Up,
    /// Overwolf refused them with `reason`; the collector keeps asking.
    Refused { reason: String },
}

/// Change in the collector connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectorEvent {
    /// An envelope was received at `at`.
    Received { at: Instant },
    /// The collector reported the state of the game-event feeds.
    Feeds(Feeds),
    /// A collector using another protocol version, `speaks`, connected.
    Outdated { speaks: u32 },
    /// No envelope received within the server's timeout.
    Silent,
}

/// Collector state after `event`. A live connection keeps its feeds across envelopes; a new
/// connection starts at `Feeds::Waiting`.
pub(crate) fn update(collector: &Collector, event: CollectorEvent) -> Collector {
    match event {
        CollectorEvent::Received { at } => Collector::Live {
            heard_at: at,
            feeds: match collector {
                Collector::Live { feeds, .. } => feeds.clone(),
                _ => Feeds::Waiting,
            },
        },
        CollectorEvent::Feeds(feeds) => match collector {
            Collector::Live { heard_at, .. } => Collector::Live {
                heard_at: *heard_at,
                feeds,
            },
            other => other.clone(),
        },
        CollectorEvent::Outdated { speaks } => Collector::Outdated { speaks },
        CollectorEvent::Silent => Collector::Offline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn live(heard_at: Instant, feeds: Feeds) -> Collector {
        Collector::Live { heard_at, feeds }
    }

    #[test]
    fn an_envelope_makes_the_link_live() {
        assert_eq!(
            update(
                &Collector::Offline,
                CollectorEvent::Received { at: 1_785_240_441 }
            ),
            live(1_785_240_441, Feeds::Waiting)
        );
    }

    #[test]
    fn a_later_envelope_moves_the_instant_and_keeps_the_feeds() {
        let first = live(1_785_240_441, Feeds::Up);

        let next = update(&first, CollectorEvent::Received { at: 1_785_240_456 });

        assert_eq!(next, live(1_785_240_456, Feeds::Up));
    }

    #[test]
    fn a_feeds_report_changes_only_the_feeds() {
        let refused = Feeds::Refused {
            reason: "not supported".into(),
        };

        let next = update(
            &live(1_785_240_441, Feeds::Waiting),
            CollectorEvent::Feeds(refused.clone()),
        );

        assert_eq!(next, live(1_785_240_441, refused));
    }

    #[test]
    fn a_feeds_report_without_a_live_link_changes_nothing() {
        assert_eq!(
            update(&Collector::Offline, CollectorEvent::Feeds(Feeds::Up)),
            Collector::Offline
        );
    }

    #[test]
    fn silence_takes_the_link_offline_and_drops_the_feeds() {
        let gone = update(&live(1, Feeds::Up), CollectorEvent::Silent);
        assert_eq!(gone, Collector::Offline);

        let back = update(&gone, CollectorEvent::Received { at: 2 });
        assert_eq!(back, live(2, Feeds::Waiting));
    }

    #[test]
    fn a_collector_speaking_another_version_is_not_live() {
        assert_eq!(
            update(&Collector::Offline, CollectorEvent::Outdated { speaks: 2 }),
            Collector::Outdated { speaks: 2 }
        );
    }
}
