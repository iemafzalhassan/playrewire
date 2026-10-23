//! Headless tests for the street's logic: `MinimalPlugins` + [`WorldLogicPlugin`], no
//! window and no GPU. Time is stepped by hand so the pacing is exact.

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use rewire_core::CityEvent;
use rewire_world::{
    BridgeLamp, CameraFocus, Intent, LampLit, Mailbox, Player, Signal, TrafficLight,
    WorldCommand, WorldLogicPlugin, WorldSignal, scene,
};

const STEP: f32 = 0.05;

fn app() -> (App, Mailbox) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, WorldLogicPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(STEP)));
    let mailbox = app.world().resource::<Mailbox>().clone();
    // Startup spawns the street; the first frames have no delta.
    app.update();
    app.update();
    (app, mailbox)
}

/// Runs frames until `seconds` of world time have passed.
fn run_for(app: &mut App, seconds: f32) {
    let frames = (seconds / STEP).round() as usize;
    for _ in 0..frames {
        app.update();
    }
}

fn green(i: u32, delay: u32) -> CityEvent {
    CityEvent::Green { i, delay }
}

fn lamp(i: u32, on: bool) -> CityEvent {
    CityEvent::Lamp { i, on }
}

fn lights(app: &mut App) -> Vec<Signal> {
    let mut q = app.world_mut().query::<(&TrafficLight, &Signal)>();
    let mut v: Vec<_> = q.iter(app.world()).map(|(l, s)| (l.i, *s)).collect();
    v.sort_by_key(|(i, _)| *i);
    v.into_iter().map(|(_, s)| s).collect()
}

fn lamps(app: &mut App) -> Vec<bool> {
    let mut q = app.world_mut().query::<(&BridgeLamp, &LampLit)>();
    let mut v: Vec<_> = q.iter(app.world()).map(|(l, s)| (l.i, s.0)).collect();
    v.sort_by_key(|(i, _)| *i);
    v.into_iter().map(|(_, on)| on).collect()
}

fn greens(app: &mut App) -> usize {
    lights(app).iter().filter(|s| **s == Signal::Green).count()
}

fn signals(mailbox: &Mailbox) -> Vec<WorldSignal> {
    mailbox.take_signals()
}

fn done(sig: &[WorldSignal]) -> bool {
    sig.contains(&WorldSignal::EventsDone)
}

fn place_player(app: &mut App, at: Vec3) {
    let mut q = app.world_mut().query_filtered::<&mut Transform, With<Player>>();
    q.single_mut(app.world_mut()).unwrap().translation = at;
}

#[test]
fn apply_green_events_sets_light_states_in_order() {
    let (mut app, mailbox) = app();
    assert_eq!(lights(&mut app), vec![Signal::Red; scene::LIGHTS as usize]);
    mailbox.send(WorldCommand::LoadMission("green-wave".into()));
    mailbox.send(WorldCommand::ApplyEvents(
        (0..5).map(|i| green(i, i * 2)).collect(),
    ));

    // Delays are scaled by 0.25: light i goes green at i * 0.5 s.
    let mut turned_at = [None; 5];
    for frame in 0..80 {
        app.update();
        for (i, s) in lights(&mut app).iter().enumerate() {
            if *s == Signal::Green && turned_at[i].is_none() {
                turned_at[i] = Some(frame);
            }
        }
    }
    let turned: Vec<usize> = turned_at.iter().map(|t| t.expect("every light went green")).collect();
    assert!(turned.windows(2).all(|w| w[0] < w[1]), "a wave, one light after another: {turned:?}");
    // 0.5 s apart at 0.05 s a frame is 10 frames.
    for w in turned.windows(2) {
        assert!((9..=11).contains(&(w[1] - w[0])), "paced at delay * 0.25: {turned:?}");
    }
    assert!(done(&signals(&mailbox)), "onEventsDone fires after the wave");
}

#[test]
fn every_run_starts_from_red() {
    let (mut app, mailbox) = app();
    mailbox.send(WorldCommand::SetReducedMotion(true));
    mailbox.send(WorldCommand::ApplyEvents(vec![green(1, 0), green(2, 0)]));
    app.update();
    assert_eq!(greens(&mut app), 2);
    mailbox.send(WorldCommand::ApplyEvents(vec![green(4, 0)]));
    app.update();
    assert_eq!(
        lights(&mut app),
        vec![Signal::Red, Signal::Red, Signal::Red, Signal::Red, Signal::Green]
    );
}

#[test]
fn lamp_events_set_bridge_lamps() {
    let (mut app, mailbox) = app();
    assert_eq!(lamps(&mut app), vec![false; scene::LAMPS as usize]);
    mailbox.send(WorldCommand::LoadMission("binary-bridge".into()));
    let thirteen = [false, false, false, false, true, true, false, true];
    mailbox.send(WorldCommand::ApplyEvents(
        thirteen.iter().enumerate().map(|(i, on)| lamp(i as u32, *on)).collect(),
    ));
    // Lamps light one after another, left to right, not all at once.
    app.update();
    run_for(&mut app, 0.7);
    let partway = lamps(&mut app);
    assert!(partway[4] && !partway[7], "lamp 4 is on before lamp 7: {partway:?}");
    run_for(&mut app, 3.0);
    assert_eq!(lamps(&mut app), thirteen.to_vec(), "13 = 00001101");
    assert!(done(&signals(&mailbox)));

    // Out-of-range lamps and lights are ignored, not a crash.
    mailbox.send(WorldCommand::ApplyEvents(vec![lamp(42, true), green(9, 0)]));
    run_for(&mut app, 2.0);
    assert_eq!(lamps(&mut app), vec![false; 8]);
}

#[test]
fn terminal_trigger_only_within_2m() {
    let (mut app, mailbox) = app();
    mailbox.send(WorldCommand::LoadMission("green-wave".into()));
    app.update();
    let terminal = scene::terminal_for("green-wave").expect("dock st terminal");
    let spot = terminal.position();
    signals(&mailbox);

    // 2.5 m away: E does nothing.
    place_player(&mut app, spot + Vec3::new(2.5, 0.0, 0.0));
    app.world_mut().resource_mut::<Intent>().interact = true;
    app.update();
    let far = signals(&mailbox);
    assert!(
        !far.iter().any(|s| matches!(s, WorldSignal::Terminal { .. })),
        "too far: {far:?}"
    );
    assert!(
        far.iter().all(|s| !matches!(s, WorldSignal::Near { mission: Some(_) })),
        "no prompt that far: {far:?}"
    );

    // 1.5 m away: the prompt shows, and E opens the terminal.
    place_player(&mut app, spot + Vec3::new(1.5, 0.0, 0.0));
    app.update();
    let near = signals(&mailbox);
    assert!(
        near.contains(&WorldSignal::Near { mission: Some("green-wave".into()) }),
        "{near:?}"
    );
    app.world_mut().resource_mut::<Intent>().interact = true;
    app.update();
    assert!(signals(&mailbox).contains(&WorldSignal::Terminal { mission: "green-wave".into() }));

    // The other mission's terminal stays shut while Dock St is loaded.
    let bridge = scene::terminal_for("binary-bridge").unwrap().position();
    place_player(&mut app, bridge + Vec3::new(1.0, 0.0, 0.0));
    app.update();
    app.world_mut().resource_mut::<Intent>().interact = true;
    app.update();
    assert!(!signals(&mailbox).iter().any(|s| matches!(s, WorldSignal::Terminal { .. })));
}

#[test]
fn go_to_terminal_walks_there_and_opens_it() {
    let (mut app, mailbox) = app();
    mailbox.send(WorldCommand::LoadMission("binary-bridge".into()));
    mailbox.send(WorldCommand::GoToTerminal);
    run_for(&mut app, 12.0);
    assert!(signals(&mailbox).contains(&WorldSignal::Terminal { mission: "binary-bridge".into() }));
}

#[test]
fn input_disabled_while_the_editor_is_open() {
    let (mut app, mailbox) = app();
    mailbox.send(WorldCommand::SetInputEnabled(false));
    app.update();
    let before = {
        let mut q = app.world_mut().query_filtered::<&Transform, With<Player>>();
        q.single(app.world()).unwrap().translation
    };
    for _ in 0..10 {
        app.world_mut().resource_mut::<Intent>().move_dir = Vec2::new(1.0, 0.0);
        app.update();
    }
    let mut q = app.world_mut().query_filtered::<&Transform, With<Player>>();
    assert_eq!(q.single(app.world()).unwrap().translation, before);
}

#[test]
fn reduced_motion_applies_end_state_immediately() {
    let (mut app, mailbox) = app();
    mailbox.send(WorldCommand::LoadMission("green-wave".into()));
    mailbox.send(WorldCommand::SetReducedMotion(true));
    mailbox.send(WorldCommand::ApplyEvents(
        (0..5).map(|i| green(i, i * 2)).collect(),
    ));
    app.update();
    assert_eq!(greens(&mut app), 5, "the end state, right away");
    assert!(done(&signals(&mailbox)));
    assert!(
        app.world().resource::<CameraFocus>().0.is_none(),
        "no camera move with reduced motion"
    );

    // With motion on, the camera does turn to watch the wave.
    mailbox.send(WorldCommand::SetReducedMotion(false));
    mailbox.send(WorldCommand::ApplyEvents(vec![green(0, 4)]));
    app.update();
    assert!(app.world().resource::<CameraFocus>().0.is_some());
}
