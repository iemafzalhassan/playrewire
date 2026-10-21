use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// The four steps of a chunk, in play order.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    #[default]
    See,
    Type,
    Drill,
    Use,
}

impl Step {
    /// Same spelling as the serde form; used in `completed` keys.
    pub fn as_str(self) -> &'static str {
        match self {
            Step::See => "see",
            Step::Type => "type",
            Step::Drill => "drill",
            Step::Use => "use",
        }
    }
}

/// One finished attempt at a step. `local_date` is the player's local day.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StepRecord {
    pub chunk_id: String,
    pub concept: String,
    pub step: Step,
    pub mission_id: Option<String>,
    pub success: bool,
    pub first_try: bool,
    pub hints_used: u32,
    pub local_date: NaiveDate,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(default)]
pub struct StreakState {
    pub current: u32,
    pub best: u32,
    pub last_day: Option<NaiveDate>,
    pub freezes: u8,
}

/// Leitner box for one concept.
///
/// `Default` is hand-written: `due` defaults to `NaiveDate::MIN` (always due).
/// Real entries are created by `apply` with an explicit date.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct ConceptState {
    pub box_level: u8,
    pub due: NaiveDate,
    pub successes: u32,
    pub fails: u32,
}

impl Default for ConceptState {
    fn default() -> Self {
        ConceptState {
            box_level: 0,
            due: NaiveDate::MIN,
            successes: 0,
            fails: 0,
        }
    }
}

/// The persistence format on desktop and web. Every field has a serde default
/// so older or partial JSON still loads.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(default)]
pub struct ProgressState {
    pub xp: u64,
    pub streak: StreakState,
    pub concepts: BTreeMap<String, ConceptState>,
    /// `"{chunk_id}:{step}"`, inserted only on success.
    pub completed: BTreeSet<String>,
    pub mission_encounters: BTreeMap<String, u32>,
}

/// Serialized as `{"kind":"xp","value":20}` or `{"kind":"freeze_used"}`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Reward {
    Xp(u64),
    StreakExtended(u32),
    FreezeUsed,
    FreezeEarned,
    LevelUp(u32),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum HelpLevel {
    #[default]
    FillBlank,
    HalfFunction,
    BlankWithHints,
}
