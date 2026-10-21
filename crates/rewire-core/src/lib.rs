//! Pure rules for PlayRewire. No I/O, no clock: dates arrive on `StepRecord`.

mod city;
mod explain;
mod mission;
mod progress;
mod schedule;
mod streak;
mod types;

pub use city::{CityEvent, parse_city_output};
pub use explain::{FriendlyError, explain_go_errors};
pub use mission::{MissionVerdict, check_mission};
pub use progress::{apply, due_concepts, help_level, level};
pub use types::{ConceptState, HelpLevel, ProgressState, Reward, Step, StepRecord, StreakState};
