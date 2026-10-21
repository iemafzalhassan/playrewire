//! `@city {json}` lines printed by learner programs become city events.

use serde::{Deserialize, Serialize};

const PREFIX: &str = "@city ";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum CityEvent {
    Green { i: u32, delay: u32 },
    Lamp { i: u32, on: bool },
    Say { text: String },
}

/// Splits `stdout` into the text without `@city` lines and the parsed events,
/// in order. A line that starts with `@city ` but is not valid stays in the
/// text. Line endings of kept lines are preserved.
pub fn parse_city_output(stdout: &str) -> (String, Vec<CityEvent>) {
    let mut kept = String::with_capacity(stdout.len());
    let mut events = Vec::new();
    for line in stdout.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);
        let body = body.strip_suffix('\r').unwrap_or(body);
        let parsed = body
            .strip_prefix(PREFIX)
            .and_then(|json| serde_json::from_str::<CityEvent>(json).ok());
        match parsed {
            Some(ev) => events.push(ev),
            None => kept.push_str(line),
        }
    }
    (kept, events)
}
