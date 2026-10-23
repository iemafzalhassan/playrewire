//! The one seam between the page and the street.
//!
//! **Choice: polling, not stored JS callbacks.** The page pushes [`WorldCommand`]s into
//! the inbox (each wasm export is one `send`), a system drains them at the start of every
//! frame, and systems push [`WorldSignal`]s into the outbox. `world.ts` drains the outbox
//! once per animation frame (`pollSignals`) and calls the `onTerminal` / `onEventsDone`
//! callbacks itself. So nothing JS-shaped lives in the Bevy world, the same queue works
//! in the headless tests, and a callback that throws can never unwind through Bevy.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use rewire_core::CityEvent;
use serde::Serialize;

/// What the page asks the street to do.
#[derive(Debug, Clone, PartialEq)]
pub enum WorldCommand {
    /// Make one mission's terminal live and put the player on that part of the street.
    LoadMission(String),
    /// Reset the lights and lamps, then play these events (green delays scaled by 0.25).
    ApplyEvents(Vec<CityEvent>),
    /// Jump to end states, no camera moves, no bobbing.
    SetReducedMotion(bool),
    /// Off while the editor overlay is open: keys typed there never move the player.
    SetInputEnabled(bool),
    /// Walk the player to the live terminal and open it (keyboard-only / assistive path).
    GoToTerminal,
    /// Ring one light or lamp with an ink circle (`None` clears it).
    Highlight(Option<Target>),
}

/// A thing in the street that can be ringed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    Light { i: u32 },
    Lamp { i: u32 },
}

/// What the street tells the page.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorldSignal {
    /// The first frame ran: the street is drawn and listening.
    Ready,
    /// The player walked into (Some) or out of (None) the live terminal's 2 m range.
    Near { mission: Option<String> },
    /// The player used the live terminal (E, Enter, a tap, or "go to the terminal").
    Terminal { mission: String },
    /// Every event of the last `ApplyEvents` has played.
    EventsDone,
}

#[derive(Default)]
struct Queues {
    inbox: Vec<WorldCommand>,
    outbox: Vec<WorldSignal>,
}

/// Shared inbox/outbox. Cloning shares the same queues.
#[derive(Resource, Clone, Default)]
pub struct Mailbox(Arc<Mutex<Queues>>);

impl Mailbox {
    pub fn send(&self, cmd: WorldCommand) {
        self.0.lock().unwrap().inbox.push(cmd);
    }

    pub fn take_commands(&self) -> Vec<WorldCommand> {
        std::mem::take(&mut self.0.lock().unwrap().inbox)
    }

    pub fn emit(&self, sig: WorldSignal) {
        self.0.lock().unwrap().outbox.push(sig);
    }

    pub fn take_signals(&self) -> Vec<WorldSignal> {
        std::mem::take(&mut self.0.lock().unwrap().outbox)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signals_serialize_for_world_ts() {
        let j = |s: WorldSignal| serde_json::to_string(&s).unwrap();
        assert_eq!(j(WorldSignal::Ready), r#"{"kind":"ready"}"#);
        assert_eq!(
            j(WorldSignal::Terminal { mission: "green-wave".into() }),
            r#"{"kind":"terminal","mission":"green-wave"}"#
        );
        assert_eq!(j(WorldSignal::Near { mission: None }), r#"{"kind":"near","mission":null}"#);
        assert_eq!(j(WorldSignal::EventsDone), r#"{"kind":"events_done"}"#);
    }

    #[test]
    fn targets_parse_from_world_ts() {
        let t: Target = serde_json::from_str(r#"{"kind":"lamp","i":5}"#).unwrap();
        assert_eq!(t, Target::Lamp { i: 5 });
    }
}
