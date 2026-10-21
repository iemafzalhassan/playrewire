//! Compares the events a program produced with what a mission expects.

use crate::city::CityEvent;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MissionVerdict {
    Pass,
    WrongEvents { first_mismatch: usize },
    NoEvents,
}

pub fn check_mission(expected: &[CityEvent], actual: &[CityEvent]) -> MissionVerdict {
    if actual.is_empty() && !expected.is_empty() {
        return MissionVerdict::NoEvents;
    }
    if expected == actual {
        return MissionVerdict::Pass;
    }
    let first_mismatch = expected
        .iter()
        .zip(actual)
        .position(|(e, a)| e != a)
        .unwrap_or(expected.len().min(actual.len()));
    MissionVerdict::WrongEvents { first_mismatch }
}
