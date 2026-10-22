//! Thin JSON-in/JSON-out wasm-bindgen exports of `rewire-core`.
//! Bad input returns a JS `Error` (an `Err(JsValue)`), never a panic.

use rewire_core::{
    CityEvent, ProgressState, StepRecord, apply, check_mission, due_concepts, explain_go_errors,
    help_level, level, parse_city_output,
};
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

fn err(msg: impl std::fmt::Display) -> JsValue {
    // JsError becomes a real JS Error without pulling in js-sys.
    JsError::new(&msg.to_string()).into()
}

fn parse<T: DeserializeOwned>(what: &str, json: &str) -> Result<T, JsValue> {
    serde_json::from_str(json).map_err(|e| err(format!("invalid {what} JSON: {e}")))
}

fn to_json<T: Serialize>(v: &T) -> Result<String, JsValue> {
    serde_json::to_string(v).map_err(err)
}

/// Native-callable core of `apply_step`: `{state, rewards}` as JSON.
pub fn apply_step_json(state_json: &str, rec_json: &str) -> Result<String, String> {
    let state: ProgressState = serde_json::from_str(state_json).map_err(|e| format!("invalid state JSON: {e}"))?;
    let rec: StepRecord = serde_json::from_str(rec_json).map_err(|e| format!("invalid record JSON: {e}"))?;
    let (state, rewards) = apply(&state, &rec);
    serde_json::to_string(&serde_json::json!({ "state": state, "rewards": rewards })).map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn apply_step(state_json: &str, rec_json: &str) -> Result<String, JsValue> {
    apply_step_json(state_json, rec_json).map_err(err)
}

#[wasm_bindgen]
pub fn due_concepts_json(state_json: &str, today: &str) -> Result<String, JsValue> {
    let state: ProgressState = parse("state", state_json)?;
    let today = today.parse().map_err(|e| err(format!("invalid date: {e}")))?;
    to_json(&due_concepts(&state, today))
}

#[wasm_bindgen]
pub fn help_level_json(state_json: &str, mission_id: &str) -> Result<String, JsValue> {
    let state: ProgressState = parse("state", state_json)?;
    to_json(&help_level(&state, mission_id))
}

#[wasm_bindgen]
pub fn level_of(xp: f64) -> u32 {
    // JS numbers are f64; negative or NaN saturate to 0.
    level(xp as u64)
}

#[wasm_bindgen]
pub fn explain_errors_json(stderr: &str, line_offset: u32, timed_out: bool, truncated: bool) -> Result<String, JsValue> {
    to_json(&explain_go_errors(stderr, line_offset, timed_out, truncated))
}

#[wasm_bindgen]
pub fn parse_city_output_json(stdout: &str) -> Result<String, JsValue> {
    let (text, events) = parse_city_output(stdout);
    to_json(&serde_json::json!({ "stdout": text, "events": events }))
}

#[wasm_bindgen]
pub fn check_mission_json(expected_json: &str, actual_json: &str) -> Result<String, JsValue> {
    let expected: Vec<CityEvent> = parse("expected events", expected_json)?;
    let actual: Vec<CityEvent> = parse("actual events", actual_json)?;
    to_json(&check_mission(&expected, &actual))
}
