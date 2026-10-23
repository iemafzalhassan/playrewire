//! The page API (wasm-bindgen), wrapped by `apps/web/src/world/world.ts`.
//!
//! Every call is a message into the [`Mailbox`]; `pollSignals` hands back what the street
//! said since the last poll (see `mailbox.rs` for why this polls).

use std::sync::OnceLock;

use bevy::prelude::*;
use rewire_core::CityEvent;
use wasm_bindgen::prelude::*;

use crate::{
    mailbox::{Mailbox, Target, WorldCommand},
    render::ScreenLabels,
};

static MAILBOX: OnceLock<Mailbox> = OnceLock::new();
static LABELS: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

fn mailbox() -> &'static Mailbox {
    MAILBOX.get_or_init(Mailbox::default)
}

fn copy_labels(labels: Res<ScreenLabels>) {
    if let Ok(json) = serde_json::to_string(&*labels) {
        *LABELS.lock().unwrap() = json;
    }
}

/// Starts the street in `<canvas id=canvas_id>`. Call once per page: winit owns the
/// canvas from then on.
#[wasm_bindgen]
pub fn start(canvas_id: String) -> Result<(), JsValue> {
    if STARTED.set(()).is_err() {
        return Err(JsValue::from_str("the street is already running"));
    }
    let mut app = crate::build_app(Some(format!("#{canvas_id}")));
    app.insert_resource(mailbox().clone())
        .add_systems(Last, copy_labels);
    app.run();
    Ok(())
}

static STARTED: OnceLock<()> = OnceLock::new();

#[wasm_bindgen(js_name = loadMission)]
pub fn load_mission(mission_id: String) {
    mailbox().send(WorldCommand::LoadMission(mission_id));
}

/// `eventsJson`: the `CityEvent` array from `parseCityOutput`.
#[wasm_bindgen(js_name = applyEvents)]
pub fn apply_events(events_json: String) -> Result<(), JsValue> {
    let events: Vec<CityEvent> =
        serde_json::from_str(&events_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    mailbox().send(WorldCommand::ApplyEvents(events));
    Ok(())
}

#[wasm_bindgen(js_name = setReducedMotion)]
pub fn set_reduced_motion(on: bool) {
    mailbox().send(WorldCommand::SetReducedMotion(on));
}

#[wasm_bindgen(js_name = setInputEnabled)]
pub fn set_input_enabled(on: bool) {
    mailbox().send(WorldCommand::SetInputEnabled(on));
}

#[wasm_bindgen(js_name = goToTerminal)]
pub fn go_to_terminal() {
    mailbox().send(WorldCommand::GoToTerminal);
}

/// `targetJson`: `{"kind":"light","i":3}`, `{"kind":"lamp","i":5}`, or `null` to clear.
#[wasm_bindgen]
pub fn highlight(target_json: String) -> Result<(), JsValue> {
    let target: Option<Target> =
        serde_json::from_str(&target_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    mailbox().send(WorldCommand::Highlight(target));
    Ok(())
}

/// JSON array of `WorldSignal`s since the last call.
#[wasm_bindgen(js_name = pollSignals)]
pub fn poll_signals() -> String {
    serde_json::to_string(&mailbox().take_signals()).unwrap_or_else(|_| "[]".into())
}

/// JSON `ScreenLabels`: where lights, lamps, the terminal and the ring are (CSS px).
#[wasm_bindgen]
pub fn labels() -> String {
    LABELS.lock().unwrap().clone()
}
