use chrono::NaiveDate;

use crate::schedule::update_concept;
use crate::streak::advance;
use crate::types::{HelpLevel, ProgressState, Reward, Step, StepRecord};

const REPEAT_XP: u64 = 2;
const FIRST_TRY_BONUS: u64 = 10;
const XP_PER_LEVEL: u64 = 250;

fn base_xp(step: Step) -> u64 {
    match step {
        Step::See => 5,
        Step::Type => 10,
        Step::Drill => 20,
        Step::Use => 30,
    }
}

/// Level for a total XP: 1 + xp / 250.
pub fn level(xp: u64) -> u32 {
    1 + (xp / XP_PER_LEVEL) as u32
}

/// Apply one finished step. Pure: returns the new state and the rewards earned.
/// Order: XP, streak, schedule, help encounters, level-up.
pub fn apply(state: &ProgressState, rec: &StepRecord) -> (ProgressState, Vec<Reward>) {
    let mut next = state.clone();
    let mut rewards = Vec::new();
    let scored = matches!(rec.step, Step::Drill | Step::Use);

    // XP and completion.
    if rec.success {
        let key = format!("{}:{}", rec.chunk_id, rec.step.as_str());
        let first = next.completed.insert(key);
        let mut xp = if first { base_xp(rec.step) } else { REPEAT_XP };
        if first && rec.first_try && scored {
            xp += FIRST_TRY_BONUS;
        }
        next.xp += xp;
        rewards.push(Reward::Xp(xp));
    }

    // Streak: only a successful Drill or Use qualifies the day.
    if rec.success && scored {
        let (streak, r) = advance(&next.streak, rec.local_date);
        next.streak = streak;
        rewards.extend(r);
    }

    // Schedule: Drill and Use outcomes, success or fail.
    if scored {
        let prev = next.concepts.get(&rec.concept).cloned().unwrap_or_default();
        next.concepts
            .insert(rec.concept.clone(), update_concept(&prev, rec));
    }

    // Help fades with each successful Use of a mission.
    if let (true, Step::Use, Some(m)) = (rec.success, rec.step, &rec.mission_id) {
        *next.mission_encounters.entry(m.clone()).or_insert(0) += 1;
    }

    let new_level = level(next.xp);
    if new_level > level(state.xp) {
        rewards.push(Reward::LevelUp(new_level));
    }
    (next, rewards)
}

/// Concepts due on or before `today`, sorted by due date then id.
pub fn due_concepts(state: &ProgressState, today: NaiveDate) -> Vec<String> {
    let mut due: Vec<(NaiveDate, &String)> = state
        .concepts
        .iter()
        .filter(|(_, c)| c.due <= today)
        .map(|(id, c)| (c.due, id))
        .collect();
    due.sort();
    due.into_iter().map(|(_, id)| id.clone()).collect()
}

/// 0 encounters: fill one blank. 1: finish half the function. 2 or more: blank editor with hints.
pub fn help_level(state: &ProgressState, mission_id: &str) -> HelpLevel {
    match state
        .mission_encounters
        .get(mission_id)
        .copied()
        .unwrap_or(0)
    {
        0 => HelpLevel::FillBlank,
        1 => HelpLevel::HalfFunction,
        _ => HelpLevel::BlankWithHints,
    }
}

#[cfg(test)]
use crate::types::*;
#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
