use chrono::{Days, NaiveDate};

use crate::types::{ConceptState, StepRecord};

/// Leitner intervals in days for boxes 0..=5 (the MVP stand-in for FSRS).
const INTERVALS: [u64; 6] = [1, 2, 4, 7, 14, 30];
const MAX_BOX: u8 = 5;

/// Update one concept after a Drill or Use outcome.
/// Clean success (first try, no hints) moves up a box; other success stays; fail goes to box 0.
pub(crate) fn update_concept(prev: &ConceptState, rec: &StepRecord) -> ConceptState {
    let mut c = prev.clone();
    // Saved JSON can hold any u8; clamp before it indexes INTERVALS.
    c.box_level = c.box_level.min(MAX_BOX);
    if rec.success {
        c.successes += 1;
        if rec.first_try && rec.hints_used == 0 {
            c.box_level = c.box_level.saturating_add(1).min(MAX_BOX);
        }
    } else {
        c.fails += 1;
        c.box_level = 0;
    }
    c.due = due_date(rec.local_date, c.box_level);
    c
}

fn due_date(day: NaiveDate, box_level: u8) -> NaiveDate {
    day + Days::new(INTERVALS[box_level as usize])
}
