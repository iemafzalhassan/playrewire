use chrono::{Days, NaiveDate};

use crate::types::{Reward, StreakState};

const FREEZE_EVERY: u32 = 7;
const MAX_FREEZES: u8 = 2;

/// Advance the streak for a qualifying day `day` (a successful Drill or Use).
///
/// Rules, checked in this order:
/// - an earlier `day` than `last_day` (clock went backwards): no change, never decrease
/// - no `last_day`: current = 1
/// - same day: no change
/// - next day: current + 1
/// - two days on and a freeze is held: spend it, then current + 1
/// - any other gap: current = 1
///
/// Reaching a multiple of 7 earns one freeze, holding at most 2. `best` tracks the max.
fn apply_streak(s: &StreakState, day: NaiveDate) -> (StreakState, Vec<Reward>) {
    let mut n = s.clone();
    let mut rewards = Vec::new();
    match s.last_day {
        Some(last) if day < last => return (n, rewards),
        Some(last) if day == last => return (n, rewards),
        Some(last) if Some(day) == last.checked_add_days(Days::new(1)) => n.current += 1,
        Some(last) if Some(day) == last.checked_add_days(Days::new(2)) && s.freezes > 0 => {
            n.freezes -= 1;
            rewards.push(Reward::FreezeUsed);
            n.current += 1;
        }
        Some(_) => n.current = 1,
        None => n.current = 1,
    }
    n.last_day = Some(day);
    n.best = n.best.max(n.current);
    if n.current > s.current {
        rewards.push(Reward::StreakExtended(n.current));
        if n.current.is_multiple_of(FREEZE_EVERY) && n.freezes < MAX_FREEZES {
            n.freezes += 1;
            rewards.push(Reward::FreezeEarned);
        }
    }
    (n, rewards)
}

/// Public to the crate so `apply` can call it.
pub(crate) fn advance(s: &StreakState, day: NaiveDate) -> (StreakState, Vec<Reward>) {
    apply_streak(s, day)
}
