use chrono::{Days, NaiveDate};

use super::*;

fn day(n: u64) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 1, 1).unwrap() + Days::new(n)
}

fn rec(chunk: &str, step: Step, success: bool, first_try: bool, date: NaiveDate) -> StepRecord {
    StepRecord {
        chunk_id: chunk.into(),
        concept: "loops".into(),
        step,
        mission_id: None,
        success,
        first_try,
        hints_used: 0,
        local_date: date,
    }
}

fn xp_of(rewards: &[Reward]) -> u64 {
    rewards
        .iter()
        .find_map(|r| {
            if let Reward::Xp(n) = r {
                Some(*n)
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn drill_on(s: &ProgressState, n: u64) -> (ProgressState, Vec<Reward>) {
    // A fresh chunk each call so completion state never interferes.
    apply(s, &rec(&format!("c{n}"), Step::Drill, true, false, day(n)))
}

#[test]
fn xp_first_completion_values() {
    let s = ProgressState::default();
    for (step, want) in [
        (Step::See, 5),
        (Step::Type, 10),
        (Step::Drill, 20),
        (Step::Use, 30),
    ] {
        let (_, rw) = apply(&s, &rec("c", step, true, false, day(0)));
        assert_eq!(xp_of(&rw), want, "{step:?}");
    }
}

#[test]
fn xp_repeat_is_two() {
    let r = rec("c", Step::Drill, true, true, day(0));
    let (s1, _) = apply(&ProgressState::default(), &r);
    let (s2, rw) = apply(&s1, &r);
    assert_eq!(xp_of(&rw), 2);
    assert_eq!(s2.xp, s1.xp + 2);
}

#[test]
fn first_try_bonus_only_drill_and_use() {
    let s = ProgressState::default();
    for (step, want) in [
        (Step::See, 5),
        (Step::Type, 10),
        (Step::Drill, 30),
        (Step::Use, 40),
    ] {
        let (_, rw) = apply(&s, &rec("c", step, true, true, day(0)));
        assert_eq!(xp_of(&rw), want, "{step:?}");
    }
}

#[test]
fn failure_gives_zero_xp_and_no_completion() {
    let (s, rw) = apply(
        &ProgressState::default(),
        &rec("c", Step::Drill, false, true, day(0)),
    );
    assert_eq!(s.xp, 0);
    assert!(s.completed.is_empty());
    assert!(rw.is_empty());
    assert_eq!(s.streak, StreakState::default());
}

#[test]
fn completed_records_chunk_and_step() {
    let (s, _) = apply(
        &ProgressState::default(),
        &rec("go/loops", Step::Use, true, false, day(0)),
    );
    assert!(s.completed.contains("go/loops:use"));
}

#[test]
fn streak_consecutive_days() {
    let mut s = ProgressState::default();
    for n in 0..3 {
        s = drill_on(&s, n).0;
    }
    assert_eq!(s.streak.current, 3);
    assert_eq!(s.streak.best, 3);
    assert_eq!(s.streak.last_day, Some(day(2)));
}

#[test]
fn streak_extended_reward_emitted() {
    let (s, rw) = drill_on(&ProgressState::default(), 0);
    assert!(rw.contains(&Reward::StreakExtended(1)));
    let (_, rw) = drill_on(&s, 1);
    assert!(rw.contains(&Reward::StreakExtended(2)));
}

#[test]
fn streak_same_day_no_change() {
    let (s1, _) = drill_on(&ProgressState::default(), 0);
    let (s2, rw) = apply(&s1, &rec("other", Step::Use, true, false, day(0)));
    assert_eq!(s2.streak, s1.streak);
    assert!(!rw.iter().any(|r| matches!(r, Reward::StreakExtended(_))));
}

#[test]
fn see_type_and_failures_never_touch_streak() {
    let s = ProgressState::default();
    for (step, ok) in [
        (Step::See, true),
        (Step::Type, true),
        (Step::Drill, false),
        (Step::Use, false),
    ] {
        let (n, _) = apply(&s, &rec("c", step, ok, true, day(0)));
        assert_eq!(n.streak, StreakState::default(), "{step:?} {ok}");
    }
}

#[test]
fn streak_gap_uses_freeze() {
    let s = ProgressState {
        streak: StreakState {
            current: 3,
            best: 3,
            last_day: Some(day(0)),
            freezes: 1,
        },
        ..Default::default()
    };
    let (n, rw) = drill_on(&s, 2);
    assert_eq!(n.streak.current, 4);
    assert_eq!(n.streak.freezes, 0);
    assert!(rw.contains(&Reward::FreezeUsed));
    assert!(rw.contains(&Reward::StreakExtended(4)));
}

#[test]
fn streak_gap_without_freeze_resets() {
    let mut s = ProgressState {
        streak: StreakState {
            current: 5,
            best: 5,
            last_day: Some(day(0)),
            freezes: 0,
        },
        ..Default::default()
    };
    let (n, _) = drill_on(&s, 2);
    assert_eq!(n.streak.current, 1);
    assert_eq!(n.streak.best, 5);
    // A three-day gap resets even with a freeze in hand.
    s.streak.freezes = 2;
    let (n, rw) = drill_on(&s, 3);
    assert_eq!(n.streak.current, 1);
    assert_eq!(n.streak.freezes, 2);
    assert!(!rw.contains(&Reward::FreezeUsed));
}

#[test]
fn streak_clock_backwards_never_decreases() {
    let mut s = ProgressState::default();
    for n in 0..5 {
        s = drill_on(&s, n).0;
    }
    assert_eq!(s.streak.current, 5);
    // Clock jumps back three days: nothing changes, not even last_day.
    let (back, rw) = drill_on(&s, 1);
    assert_eq!(back.streak, s.streak);
    assert!(
        !rw.iter()
            .any(|r| matches!(r, Reward::StreakExtended(_) | Reward::FreezeUsed))
    );
    // The next normal day after the original last day still extends.
    let (next, _) = drill_on(&back, 5);
    assert_eq!(next.streak.current, 6);
    assert_eq!(next.streak.last_day, Some(day(5)));
}

#[test]
fn freeze_earned_at_7_and_capped_at_2() {
    let mut s = ProgressState::default();
    let mut earned = 0;
    for n in 0..21 {
        let (ns, rw) = drill_on(&s, n);
        s = ns;
        let got = rw.contains(&Reward::FreezeEarned);
        if got {
            earned += 1;
        }
        match n {
            6 | 13 => assert!(got, "freeze expected on day {n}"),
            _ => assert!(!got, "no freeze expected on day {n}"),
        }
        match n {
            0..=5 => assert_eq!(s.streak.freezes, 0),
            6..=12 => assert_eq!(s.streak.freezes, 1),
            _ => assert_eq!(s.streak.freezes, 2),
        }
    }
    assert_eq!(earned, 2);
    assert_eq!(s.streak.current, 21);
    assert_eq!(s.streak.freezes, 2, "cap holds at 21");
}

#[test]
fn schedule_box_moves() {
    let mut s = ProgressState::default();
    let intervals = [2, 4, 7, 14, 30, 30];
    for (i, want) in intervals.iter().enumerate() {
        s = apply(&s, &rec(&format!("c{i}"), Step::Drill, true, true, day(0))).0;
        let c = &s.concepts["loops"];
        assert_eq!(c.box_level as usize, (i + 1).min(5));
        assert_eq!(c.due, day(0) + Days::new(*want));
    }
    // Success with hints holds the box.
    let mut r = rec("h", Step::Use, true, true, day(0));
    r.hints_used = 1;
    s = apply(&s, &r).0;
    assert_eq!(s.concepts["loops"].box_level, 5);
    // Success not on first try holds the box.
    s = apply(&s, &rec("n", Step::Drill, true, false, day(0))).0;
    assert_eq!(s.concepts["loops"].box_level, 5);
    // Failure drops to box 0, due in 1 day.
    s = apply(&s, &rec("f", Step::Drill, false, true, day(3))).0;
    let c = &s.concepts["loops"];
    assert_eq!((c.box_level, c.due), (0, day(4)));
    assert_eq!((c.successes, c.fails), (8, 1));
}

#[test]
fn see_and_type_do_not_touch_schedule() {
    let s = ProgressState::default();
    for step in [Step::See, Step::Type] {
        assert!(
            apply(&s, &rec("c", step, true, true, day(0)))
                .0
                .concepts
                .is_empty()
        );
    }
}

#[test]
fn due_concepts_sorted() {
    let mut s = ProgressState::default();
    let mk = |due| ConceptState {
        due,
        ..Default::default()
    };
    s.concepts.insert("b".into(), mk(day(2)));
    s.concepts.insert("a".into(), mk(day(2)));
    s.concepts.insert("z".into(), mk(day(1)));
    s.concepts.insert("future".into(), mk(day(9)));
    assert_eq!(due_concepts(&s, day(5)), vec!["z", "a", "b"]);
    assert!(due_concepts(&s, day(0)).is_empty());
}

#[test]
fn help_level_fades_after_encounters() {
    let mut s = ProgressState::default();
    assert_eq!(help_level(&s, "m1"), HelpLevel::FillBlank);
    let mut r = rec("c", Step::Use, true, false, day(0));
    r.mission_id = Some("m1".into());
    s = apply(&s, &r).0;
    assert_eq!(help_level(&s, "m1"), HelpLevel::HalfFunction);
    s = apply(&s, &r).0;
    assert_eq!(help_level(&s, "m1"), HelpLevel::BlankWithHints);
    s = apply(&s, &r).0;
    assert_eq!(help_level(&s, "m1"), HelpLevel::BlankWithHints);
    assert_eq!(help_level(&s, "other"), HelpLevel::FillBlank);
}

#[test]
fn help_encounters_only_count_successful_use() {
    let mut r = rec("c", Step::Use, false, false, day(0));
    r.mission_id = Some("m1".into());
    let (s, _) = apply(&ProgressState::default(), &r);
    assert!(s.mission_encounters.is_empty());
    r.success = true;
    r.step = Step::Drill;
    let (s, _) = apply(&s, &r);
    assert!(s.mission_encounters.is_empty());
}

#[test]
fn level_formula() {
    assert_eq!(level(0), 1);
    assert_eq!(level(249), 1);
    assert_eq!(level(250), 2);
    assert_eq!(level(500), 3);
}

#[test]
fn level_up_reward_emitted_once() {
    let s = ProgressState {
        xp: 240,
        ..Default::default()
    };
    let (n, rw) = apply(&s, &rec("c", Step::Type, true, false, day(0)));
    assert_eq!(n.xp, 250);
    assert!(rw.contains(&Reward::LevelUp(2)));
    let (_, rw) = apply(&n, &rec("d", Step::See, true, false, day(0)));
    assert!(!rw.iter().any(|r| matches!(r, Reward::LevelUp(_))));
}

#[test]
fn apply_is_pure() {
    let s = ProgressState::default();
    let _ = apply(&s, &rec("c", Step::Drill, true, true, day(0)));
    assert_eq!(s, ProgressState::default());
}

#[test]
fn state_json_roundtrip() {
    let mut s = ProgressState::default();
    let mut r = rec("c", Step::Use, true, true, day(0));
    r.mission_id = Some("m1".into());
    s = apply(&s, &r).0;
    let json = serde_json::to_string(&s).unwrap();
    let back: ProgressState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
}

#[test]
fn json_shape_is_pinned() {
    let v = serde_json::to_value(Reward::Xp(20)).unwrap();
    assert_eq!(v, serde_json::json!({"kind": "xp", "value": 20}));
    let v = serde_json::to_value(Reward::FreezeUsed).unwrap();
    assert_eq!(v, serde_json::json!({"kind": "freeze_used"}));
    let v = serde_json::to_value(Reward::LevelUp(3)).unwrap();
    assert_eq!(v, serde_json::json!({"kind": "level_up", "value": 3}));
    assert_eq!(serde_json::to_value(Step::Use).unwrap(), "use");
    assert_eq!(
        serde_json::to_value(HelpLevel::BlankWithHints).unwrap(),
        "blank_with_hints"
    );

    let s = ProgressState {
        streak: StreakState {
            last_day: Some(day(0)),
            ..Default::default()
        },
        ..Default::default()
    };
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["streak"]["last_day"], "2026-01-01");
    assert!(v["mission_encounters"].is_object());
}

#[test]
fn old_or_partial_json_loads() {
    let s: ProgressState = serde_json::from_str("{}").unwrap();
    assert_eq!(s, ProgressState::default());
    let s: ProgressState = serde_json::from_str(r#"{"xp":12,"streak":{"current":2}}"#).unwrap();
    assert_eq!((s.xp, s.streak.current, s.streak.freezes), (12, 2, 0));
}
