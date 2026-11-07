use crate::time::Instant;

/// Timed cycle of an open zone: day and night, warm and cold, Fass and Vome, or Duviri's moods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cycle {
    Cetus,
    Earth,
    Vallis,
    Cambion,
    Duviri,
}

/// One phase of a cycle and its duration in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phase {
    pub key: &'static str,
    pub seconds: i64,
}

/// Current phase of a cycle and the instant it ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleState {
    pub cycle: Cycle,
    /// Index into the cycle's phases.
    pub phase: usize,
    pub ends: Instant,
}

const CETUS_PHASES: [Phase; 2] = [
    Phase {
        key: "day",
        seconds: 6000,
    },
    Phase {
        key: "night",
        seconds: 3000,
    },
];

const CAMBION_PHASES: [Phase; 2] = [
    Phase {
        key: "fass",
        seconds: 6000,
    },
    Phase {
        key: "vome",
        seconds: 3000,
    },
];

const EARTH_PHASES: [Phase; 2] = [
    Phase {
        key: "day",
        seconds: 14400,
    },
    Phase {
        key: "night",
        seconds: 14400,
    },
];

const VALLIS_PHASES: [Phase; 2] = [
    Phase {
        key: "warm",
        seconds: 400,
    },
    Phase {
        key: "cold",
        seconds: 1200,
    },
];

const DUVIRI_PHASES: [Phase; 5] = [
    Phase {
        key: "sorrow",
        seconds: 7200,
    },
    Phase {
        key: "fear",
        seconds: 7200,
    },
    Phase {
        key: "joy",
        seconds: 7200,
    },
    Phase {
        key: "anger",
        seconds: 7200,
    },
    Phase {
        key: "envy",
        seconds: 7200,
    },
];

/// Length of the Cetus day-night cycle in seconds; the last 3000 seconds are night.
const CETUS_LOOP: i64 = 9000;

/// Start of a warm phase on Orb Vallis, taken from the cycle observed on 2026-07-28.
const VALLIS_ANCHOR: Instant = 1_785_240_808;

impl Cycle {
    const ALL: [Cycle; 5] = [
        Cycle::Cetus,
        Cycle::Earth,
        Cycle::Vallis,
        Cycle::Cambion,
        Cycle::Duviri,
    ];

    /// Phases in the order they run.
    pub fn phases(self) -> &'static [Phase] {
        match self {
            Cycle::Cetus => &CETUS_PHASES,
            Cycle::Earth => &EARTH_PHASES,
            Cycle::Vallis => &VALLIS_PHASES,
            Cycle::Cambion => &CAMBION_PHASES,
            Cycle::Duviri => &DUVIRI_PHASES,
        }
    }

    /// A past instant at which the first phase began: one Cetus cycle before `ostron_ends` for
    /// Cetus and Cambion, the Unix epoch for Earth and Duviri, `VALLIS_ANCHOR` for Orb Vallis.
    fn anchor(self, ostron_ends: Instant) -> Instant {
        match self {
            Cycle::Cetus | Cycle::Cambion => ostron_ends - CETUS_LOOP,
            Cycle::Earth | Cycle::Duviri => 0,
            Cycle::Vallis => VALLIS_ANCHOR,
        }
    }

    fn at(self, now: Instant, ostron_ends: Instant) -> CycleState {
        let phases = self.phases();
        let turn: i64 = phases.iter().map(|phase| phase.seconds).sum();
        let mut into = (now - self.anchor(ostron_ends)).rem_euclid(turn);

        for (index, phase) in phases.iter().enumerate() {
            if into < phase.seconds {
                return CycleState {
                    cycle: self,
                    phase: index,
                    ends: now + phase.seconds - into,
                };
            }
            into -= phase.seconds;
        }

        unreachable!("phases cover the whole turn")
    }
}

/// State of every cycle at `now`, given the instant the current Cetus day-night cycle ends.
pub fn cycles(now: Instant, ostron_ends: Instant) -> Vec<CycleState> {
    Cycle::ALL
        .iter()
        .map(|cycle| cycle.at(now, ostron_ends))
        .collect()
}

/// Relic era of a void fissure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Era {
    Lith,
    Meso,
    Neo,
    Axi,
    Requiem,
    Omnia,
}

/// One open void fissure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fissure {
    /// Node key as in the catalog's regions table.
    pub node: String,
    pub era: Era,
    /// Whether it is a Steel Path fissure.
    pub hard: bool,
    /// Whether it is a Railjack void storm rather than a ground mission.
    pub storm: bool,
    /// Mission type code from the world state, e.g. `MT_DEFENSE`; `None` for void storms.
    pub mission: Option<String>,
    pub expires: Instant,
}

/// Game-wide reset timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reset {
    Daily,
    Sortie,
    Weekly,
}

/// A reset and the instant it next happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deadline {
    pub reset: Reset,
    pub at: Instant,
}

/// Vendor that visits for a limited time and then leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    Baro,
    Varzia,
}

/// One visit of a rotating vendor to a relay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Visit {
    pub vendor: Vendor,
    /// Relay key as in the catalog's regions table.
    pub node: String,
    pub arrives: Instant,
    pub departs: Instant,
}

/// Snapshot of timed world content: cycles, fissures, resets and vendor visits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Worldstate {
    /// Instant the source world state document was generated.
    pub at: Instant,
    pub cycles: Vec<CycleState>,
    /// Open fissures, soonest to expire first.
    pub fissures: Vec<Fissure>,
    /// Game-wide resets, soonest first.
    pub deadlines: Vec<Deadline>,
    pub visits: Vec<Visit>,
}

#[cfg(test)]
mod tests {
    use super::{Cycle, cycles};

    /// End of a Cetus day-night cycle, 2026-07-28T12:57:58Z.
    const OSTRON_ENDS: i64 = 1_785_243_478;

    #[test]
    fn cetus_turns_to_night_fifty_minutes_before_the_window_closes() {
        let night = Cycle::Cetus.at(OSTRON_ENDS - 2999, OSTRON_ENDS);
        assert_eq!(night.phase, 1);
        assert_eq!(night.ends, OSTRON_ENDS);

        let day = Cycle::Cetus.at(OSTRON_ENDS - 3001, OSTRON_ENDS);
        assert_eq!(day.phase, 0);
        assert_eq!(day.ends, OSTRON_ENDS - 3000);
    }

    #[test]
    fn cambion_follows_the_ostron_window() {
        let now = OSTRON_ENDS - 1000;

        assert_eq!(
            Cycle::Cambion.at(now, OSTRON_ENDS).phase,
            Cycle::Cetus.at(now, OSTRON_ENDS).phase
        );
    }

    #[test]
    fn vallis_is_cold_for_twenty_minutes_of_every_loop() {
        let warm = Cycle::Vallis.at(1_785_240_808, OSTRON_ENDS);
        assert_eq!(warm.phase, 0);
        assert_eq!(warm.ends, 1_785_241_208);

        let cold = Cycle::Vallis.at(1_785_241_208, OSTRON_ENDS);
        assert_eq!(cold.phase, 1);
        assert_eq!(cold.ends, 1_785_242_408);
    }

    #[test]
    fn earth_and_duviri_ride_on_the_epoch() {
        let earth = Cycle::Earth.at(1_785_240_441, OSTRON_ENDS);
        assert_eq!(earth.phase, 1);
        assert_eq!(earth.ends, 1_785_254_400);

        let duviri = Cycle::Duviri.at(1_785_240_441, OSTRON_ENDS);
        assert_eq!(duviri.phase, 0);
        assert_eq!(duviri.ends, 1_785_247_200);
    }

    #[test]
    fn every_cycle_lands_inside_a_phase_that_is_still_ahead() {
        let now = 1_785_240_441;

        for state in cycles(now, OSTRON_ENDS) {
            assert!(state.phase < state.cycle.phases().len());
            assert!(state.ends > now);
        }
    }
}
