use crate::worldstate::{Cycle, Era, Vendor, Visit, Worldstate};

/// World state event the user asked to be notified about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Watch {
    /// A cycle entering a phase; `phase` is the phase key, e.g. `night`.
    Phase { cycle: Cycle, phase: String },
    /// A vendor's new offer: Baro Ki'Teer arriving, or Varzia's offer rotating.
    Arrival { vendor: Vendor },
    /// A fissure opening that matches every given condition: era, mission type code (e.g.
    /// `MT_DEFENSE`), kind; a `None` condition matches any value.
    Fissure {
        era: Option<Era>,
        mission: Option<String>,
        kind: Option<FissureKind>,
    },
}

/// Fissure kind a fissure watch matches: normal, Steel Path or Railjack void storm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FissureKind {
    Plain,
    Hard,
    Storm,
}

/// A watch that fired, with the node it fired for; `None` for a watch without a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alarm {
    pub watch: Watch,
    pub node: Option<String>,
}

const CYCLES: [(Cycle, &str); 5] = [
    (Cycle::Cetus, "cetus"),
    (Cycle::Earth, "earth"),
    (Cycle::Vallis, "vallis"),
    (Cycle::Cambion, "cambion"),
    (Cycle::Duviri, "duviri"),
];

const VENDORS: [(Vendor, &str); 2] = [(Vendor::Baro, "baro"), (Vendor::Varzia, "varzia")];

const ERAS: [(Era, &str); 6] = [
    (Era::Lith, "lith"),
    (Era::Meso, "meso"),
    (Era::Neo, "neo"),
    (Era::Axi, "axi"),
    (Era::Requiem, "requiem"),
    (Era::Omnia, "omnia"),
];

const KINDS: [(FissureKind, &str); 3] = [
    (FissureKind::Plain, "plain"),
    (FissureKind::Hard, "hard"),
    (FissureKind::Storm, "storm"),
];

/// Code of a condition that matches any value.
const ANY: &str = "*";

/// Parses a condition code: `Some(None)` for `*`, `Some(Some(value))` for a valid code, `None`
/// for an invalid one.
fn any_or<T>(code: &str, read: impl Fn(&str) -> Option<T>) -> Option<Option<T>> {
    match code {
        ANY => Some(None),
        _ => read(code).map(Some),
    }
}

fn code_of<T: PartialEq + Copy>(table: &[(T, &'static str)], value: T) -> &'static str {
    table
        .iter()
        .find(|(each, _)| *each == value)
        .map(|(_, code)| *code)
        .expect("every value has a code")
}

fn value_of<T: Copy>(table: &[(T, &str)], code: &str) -> Option<T> {
    table
        .iter()
        .find(|(_, each)| *each == code)
        .map(|(value, _)| *value)
}

impl Watch {
    /// Code the watch is stored under: `phase:cetus:night`, `arrival:baro`,
    /// `fissure:axi:hard:MT_DEFENSE` (`*` for a condition that matches any value).
    pub fn as_code(&self) -> String {
        match self {
            Watch::Phase { cycle, phase } => format!("phase:{}:{phase}", code_of(&CYCLES, *cycle)),
            Watch::Arrival { vendor } => format!("arrival:{}", code_of(&VENDORS, *vendor)),
            Watch::Fissure { era, mission, kind } => format!(
                "fissure:{}:{}:{}",
                era.map_or(ANY, |era| code_of(&ERAS, era)),
                kind.map_or(ANY, |kind| code_of(&KINDS, kind)),
                mission.as_deref().unwrap_or(ANY)
            ),
        }
    }

    /// Watch for a stored code; `None` when the code is invalid.
    pub fn parse(code: &str) -> Option<Watch> {
        let parts: Vec<&str> = code.split(':').collect();
        let watch = match parts.as_slice() {
            ["phase", cycle, phase] if !phase.is_empty() => Watch::Phase {
                cycle: value_of(&CYCLES, cycle)?,
                phase: phase.to_string(),
            },
            ["arrival", vendor] => Watch::Arrival {
                vendor: value_of(&VENDORS, vendor)?,
            },
            ["fissure", era, kind, mission] => Watch::Fissure {
                era: any_or(era, |era| value_of(&ERAS, era))?,
                kind: any_or(kind, |kind| value_of(&KINDS, kind))?,
                mission: any_or(mission, |mission| Some(mission.to_string()))?,
            },
            _ => return None,
        };
        Some(watch)
    }
}

/// Whether the vendor's visit covers `now`.
fn is_here(visit: &Visit, now: i64) -> bool {
    visit.arrives <= now && now < visit.departs
}

/// Watches that fired between two world state readings. Without an earlier reading nothing
/// fires, so conditions already true when the application starts do not notify.
pub fn alarms(watches: &[Watch], before: Option<&Worldstate>, after: &Worldstate) -> Vec<Alarm> {
    let Some(before) = before else {
        return Vec::new();
    };
    let mut rung = Vec::new();
    for watch in watches {
        match watch {
            Watch::Phase { cycle, phase } => {
                let running = |world: &Worldstate| {
                    world
                        .cycles
                        .iter()
                        .find(|state| state.cycle == *cycle)
                        .map(|state| state.cycle.phases()[state.phase].key)
                };
                if running(after) == Some(phase.as_str()) && running(before) != running(after) {
                    rung.push(Alarm {
                        watch: watch.clone(),
                        node: None,
                    });
                }
            }
            Watch::Arrival { vendor } => {
                let visit = |world: &Worldstate| {
                    world
                        .visits
                        .iter()
                        .find(|visit| visit.vendor == *vendor)
                        .cloned()
                };
                if let Some(now) = visit(after)
                    && is_here(&now, after.at)
                {
                    let fresh = match visit(before) {
                        Some(then) => !is_here(&then, before.at) || then.arrives != now.arrives,
                        None => true,
                    };
                    if fresh {
                        rung.push(Alarm {
                            watch: watch.clone(),
                            node: Some(now.node),
                        });
                    }
                }
            }
            Watch::Fissure { era, mission, kind } => {
                let matches = |fissure: &crate::worldstate::Fissure| {
                    let of = match (fissure.storm, fissure.hard) {
                        (true, _) => FissureKind::Storm,
                        (false, true) => FissureKind::Hard,
                        (false, false) => FissureKind::Plain,
                    };
                    era.is_none_or(|era| fissure.era == era)
                        && kind.is_none_or(|kind| of == kind)
                        && mission
                            .as_ref()
                            .is_none_or(|mission| fissure.mission.as_ref() == Some(mission))
                };
                for fissure in after.fissures.iter().filter(|fissure| matches(fissure)) {
                    let known = before.fissures.iter().any(|then| {
                        then.node == fissure.node
                            && then.hard == fissure.hard
                            && then.storm == fissure.storm
                    });
                    if !known {
                        rung.push(Alarm {
                            watch: watch.clone(),
                            node: Some(fissure.node.clone()),
                        });
                    }
                }
            }
        }
    }
    rung
}

#[cfg(test)]
mod tests {
    use super::{Alarm, FissureKind, Watch, alarms};
    use crate::worldstate::{Cycle, CycleState, Era, Fissure, Vendor, Visit, Worldstate};

    fn world(at: i64) -> Worldstate {
        Worldstate {
            at,
            cycles: Vec::new(),
            fissures: Vec::new(),
            deadlines: Vec::new(),
            visits: Vec::new(),
        }
    }

    fn cetus(phase: usize) -> CycleState {
        CycleState {
            cycle: Cycle::Cetus,
            phase,
            ends: 0,
        }
    }

    fn fissure(node: &str, era: Era, hard: bool, mission: &str) -> Fissure {
        Fissure {
            node: node.into(),
            era,
            hard,
            storm: false,
            mission: Some(mission.into()),
            expires: 10_000,
        }
    }

    #[test]
    fn every_watch_survives_its_code() {
        for watch in [
            Watch::Phase {
                cycle: Cycle::Cetus,
                phase: "night".into(),
            },
            Watch::Arrival {
                vendor: Vendor::Varzia,
            },
            Watch::Fissure {
                era: Some(Era::Axi),
                mission: Some("MT_DEFENSE".into()),
                kind: Some(FissureKind::Hard),
            },
            Watch::Fissure {
                era: None,
                mission: None,
                kind: Some(FissureKind::Hard),
            },
        ] {
            assert_eq!(Watch::parse(&watch.as_code()), Some(watch));
        }
        assert_eq!(Watch::parse("phase:mars:night"), None);
        assert_eq!(Watch::parse("arrival"), None);
    }

    #[test]
    fn the_first_reading_sets_nothing_off() {
        let watches = [Watch::Arrival {
            vendor: Vendor::Baro,
        }];
        let mut after = world(100);
        after.visits.push(Visit {
            vendor: Vendor::Baro,
            node: "SaturnHUB".into(),
            arrives: 50,
            departs: 500,
        });
        assert!(alarms(&watches, None, &after).is_empty());
    }

    #[test]
    fn a_phase_rings_once_as_it_begins() {
        let watches = [Watch::Phase {
            cycle: Cycle::Cetus,
            phase: "night".into(),
        }];
        let mut day = world(100);
        day.cycles.push(cetus(0));
        let mut night = world(130);
        night.cycles.push(cetus(1));

        assert_eq!(alarms(&watches, Some(&day), &night).len(), 1);
        assert!(alarms(&watches, Some(&night), &night).is_empty());
        assert!(alarms(&watches, Some(&night), &day).is_empty());
    }

    #[test]
    fn baro_rings_on_arrival_at_his_relay() {
        let watches = [Watch::Arrival {
            vendor: Vendor::Baro,
        }];
        let visit = Visit {
            vendor: Vendor::Baro,
            node: "SaturnHUB".into(),
            arrives: 200,
            departs: 500,
        };
        let mut away = world(150);
        away.visits.push(visit.clone());
        let mut here = world(210);
        here.visits.push(visit);

        assert_eq!(
            alarms(&watches, Some(&away), &here),
            [Alarm {
                watch: watches[0].clone(),
                node: Some("SaturnHUB".into()),
            }]
        );
        assert!(alarms(&watches, Some(&here), &here).is_empty());
    }

    #[test]
    fn a_new_fissure_rings_when_it_meets_every_condition() {
        let watches = [Watch::Fissure {
            era: None,
            mission: Some("MT_DEFENSE".into()),
            kind: Some(FissureKind::Hard),
        }];
        let mut before = world(100);
        before
            .fissures
            .push(fissure("SolNode1", Era::Axi, true, "MT_DEFENSE"));
        let mut after = world(130);
        after
            .fissures
            .push(fissure("SolNode1", Era::Axi, true, "MT_DEFENSE"));
        after
            .fissures
            .push(fissure("SolNode2", Era::Lith, true, "MT_DEFENSE"));
        after
            .fissures
            .push(fissure("SolNode3", Era::Axi, false, "MT_DEFENSE"));
        after
            .fissures
            .push(fissure("SolNode4", Era::Axi, true, "MT_SURVIVAL"));

        let rung = alarms(&watches, Some(&before), &after);
        assert_eq!(rung.len(), 1);
        assert_eq!(rung[0].node.as_deref(), Some("SolNode2"));
    }

    #[test]
    fn a_fissure_watch_with_no_conditions_takes_any_new_fissure() {
        let watches = [Watch::Fissure {
            era: None,
            mission: None,
            kind: None,
        }];
        let mut after = world(130);
        after
            .fissures
            .push(fissure("SolNode9", Era::Neo, false, "MT_RESCUE"));
        assert_eq!(alarms(&watches, Some(&world(100)), &after).len(), 1);
    }
}
