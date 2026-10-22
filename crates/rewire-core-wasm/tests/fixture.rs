//! Produces `tests/fixtures/apply_step.json` from native `apply`; the Vitest
//! suite asserts the wasm build returns the same thing. Regenerate with
//! `UPDATE_FIXTURE=1 cargo test -p rewire-core-wasm`.

use chrono::NaiveDate;
use rewire_core::{ProgressState, Step, StepRecord, apply};
use serde_json::{Value, json};

fn build() -> Value {
    let d = |day| NaiveDate::from_ymd_opt(2026, 3, day).unwrap();
    let rec = |chunk: &str, day, first_try| StepRecord {
        chunk_id: chunk.into(),
        concept: "loops".into(),
        step: Step::Drill,
        mission_id: None,
        success: true,
        first_try,
        hints_used: 0,
        local_date: d(day),
    };
    // A non-trivial starting state: one earlier drill, so the streak and box move.
    let (state, _) = apply(&ProgressState::default(), &rec("go/loops-1", 1, true));
    let record = rec("go/loops-2", 2, true);
    let (expected_state, expected_rewards) = apply(&state, &record);
    json!({
        "state": state,
        "record": record,
        "expected_state": expected_state,
        "expected_rewards": expected_rewards,
    })
}

#[test]
fn apply_step_fixture_is_not_stale() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/apply_step.json");
    let fresh = serde_json::to_string_pretty(&build()).unwrap() + "\n";
    if std::env::var_os("UPDATE_FIXTURE").is_some() {
        std::fs::write(path, &fresh).unwrap();
    }
    let committed = std::fs::read_to_string(path).expect("fixture missing; run with UPDATE_FIXTURE=1");
    assert_eq!(committed, fresh, "stale fixture; run UPDATE_FIXTURE=1 cargo test -p rewire-core-wasm");
}

#[test]
fn bad_json_is_an_error_not_a_panic() {
    assert!(rewire_core_wasm::apply_step_json("{nope", "{}").is_err());
    assert!(rewire_core_wasm::apply_step_json("{}", "[]").is_err());
}
