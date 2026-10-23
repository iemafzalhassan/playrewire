//! City events from the learner's program, played back in the street.
//!
//! `SetGreen(i, s)` turns light i green after s seconds; the street plays that at a
//! quarter of the time (`GREEN_DELAY_SCALE`), so the 0-2-4-6-8 s wave takes 2 s to
//! watch. `Lamp(i, on)` events light one after another, left to right, a short beat
//! apart. Every run starts from all-red, all-off. Reduced motion skips straight to the
//! end state.

use bevy::prelude::*;
use rewire_core::CityEvent;

use crate::{
    mailbox::{Mailbox, Target, WorldSignal},
    scene,
};

/// Green delays are played at this fraction of real seconds.
pub const GREEN_DELAY_SCALE: f32 = 0.25;
/// The beat between two lamp events.
pub const LAMP_STAGGER: f32 = 0.15;
/// No event waits longer than this, whatever delay the program asked for.
pub const MAX_WAIT: f32 = 8.0;
/// After the last event, how long the street holds before saying it is done.
pub const SETTLE: f32 = 0.6;

#[derive(Component, Debug, Clone, Copy)]
pub struct TrafficLight {
    pub i: u32,
}

/// A traffic light's state.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Signal {
    #[default]
    Red,
    Green,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct BridgeLamp {
    pub i: u32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LampLit(pub bool);

/// When (in `Time::elapsed_secs`) a light or lamp last changed; drives its little pop.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ChangedAt(pub f32);

/// The light or lamp ringed in ink after a wrong run.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct Highlight(pub Option<Target>);

/// Where the camera should look instead of following the player (`None` = follow), and
/// how many metres across it should fit.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct CameraFocus(pub Option<Vec3>, pub f32);

/// Reduced motion: end states at once, no camera moves, no bobbing.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Motion {
    pub reduced: bool,
}

/// Events waiting to play.
#[derive(Resource, Debug, Default)]
pub struct Playback {
    clock: f32,
    queue: Vec<(f32, CityEvent)>,
    running: bool,
    started_this_frame: bool,
    last_at: f32,
}

/// When each event plays, in seconds from the start, sorted (ties keep program order).
pub fn schedule(events: &[CityEvent]) -> Vec<(f32, CityEvent)> {
    let mut lamp_beat = 0;
    let mut out: Vec<(f32, CityEvent)> = events
        .iter()
        .map(|e| {
            let at = match e {
                CityEvent::Green { delay, .. } => *delay as f32 * GREEN_DELAY_SCALE,
                CityEvent::Lamp { .. } => {
                    lamp_beat += 1;
                    (lamp_beat - 1) as f32 * LAMP_STAGGER
                }
                CityEvent::Say { .. } => 0.0,
            };
            (at.min(MAX_WAIT), e.clone())
        })
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}


fn play(event: &CityEvent, now: f32, lights: &mut Query<(&TrafficLight, &mut Signal, &mut ChangedAt)>, lamps: &mut Query<(&BridgeLamp, &mut LampLit, &mut ChangedAt), Without<TrafficLight>>) {
    match event {
        CityEvent::Green { i, .. } => {
            for (light, mut signal, mut at) in lights.iter_mut() {
                if light.i == *i && *signal != Signal::Green {
                    *signal = Signal::Green;
                    at.0 = now;
                }
            }
        }
        CityEvent::Lamp { i, on } => {
            for (lamp, mut lit, mut at) in lamps.iter_mut() {
                if lamp.i == *i && lit.0 != *on {
                    lit.0 = *on;
                    at.0 = now;
                }
            }
        }
        // The page shows what the city says.
        CityEvent::Say { .. } => {}
    }
}

/// Resets the street and starts playing `events`. Reduced motion plays all of them now.
pub(crate) fn start_playback(
    events: Vec<CityEvent>,
    now: f32,
    motion: Motion,
    playback: &mut Playback,
    focus: &mut CameraFocus,
    highlight: &mut Highlight,
    lights: &mut Query<(&TrafficLight, &mut Signal, &mut ChangedAt)>,
    lamps: &mut Query<(&BridgeLamp, &mut LampLit, &mut ChangedAt), Without<TrafficLight>>,
    mailbox: &Mailbox,
) {
    for (_, mut signal, mut at) in lights.iter_mut() {
        if *signal != Signal::Red {
            *signal = Signal::Red;
            at.0 = now;
        }
    }
    for (_, mut lit, mut at) in lamps.iter_mut() {
        if lit.0 {
            lit.0 = false;
            at.0 = now;
        }
    }
    highlight.0 = None;

    let queue = schedule(&events);
    if motion.reduced {
        for (_, e) in &queue {
            play(e, now, lights, lamps);
        }
        *playback = Playback::default();
        mailbox.emit(WorldSignal::EventsDone);
        return;
    }
    let shows_lights = events.iter().any(|e| matches!(e, CityEvent::Green { .. }));
    let shows_lamps = events.iter().any(|e| matches!(e, CityEvent::Lamp { .. }));
    if shows_lights {
        *focus = CameraFocus(Some(scene::lights_center()), 19.0);
    } else if shows_lamps {
        *focus = CameraFocus(Some(scene::lamps_center()), 9.5);
    }
    *playback = Playback {
        clock: 0.0,
        last_at: queue.last().map_or(0.0, |(t, _)| *t),
        queue,
        running: true,
        started_this_frame: true,
    };
    fire_due(playback, now, lights, lamps);
}

fn fire_due(playback: &mut Playback, now: f32, lights: &mut Query<(&TrafficLight, &mut Signal, &mut ChangedAt)>, lamps: &mut Query<(&BridgeLamp, &mut LampLit, &mut ChangedAt), Without<TrafficLight>>) {
    let due = playback.queue.iter().take_while(|(t, _)| *t <= playback.clock + 1e-4).count();
    for (_, e) in playback.queue.drain(..due) {
        play(&e, now, lights, lamps);
    }
}

pub(crate) fn tick_playback(
    time: Res<Time>,
    mailbox: Res<Mailbox>,
    mut playback: ResMut<Playback>,
    mut lights: Query<(&TrafficLight, &mut Signal, &mut ChangedAt)>,
    mut lamps: Query<(&BridgeLamp, &mut LampLit, &mut ChangedAt), Without<TrafficLight>>,
) {
    if !playback.running {
        return;
    }
    if playback.started_this_frame {
        playback.started_this_frame = false;
        return;
    }
    playback.clock += time.delta_secs();
    let now = time.elapsed_secs();
    fire_due(&mut playback, now, &mut lights, &mut lamps);
    if playback.queue.is_empty() && playback.clock >= playback.last_at + SETTLE {
        playback.running = false;
        mailbox.emit(WorldSignal::EventsDone);
    }
}

/// The logical lights and lamps (the render plugin dresses them).
pub(crate) fn spawn_city(mut commands: Commands) {
    for i in 0..scene::LIGHTS {
        commands.spawn((
            Name::new(format!("light {i}")),
            TrafficLight { i },
            Signal::Red,
            ChangedAt::default(),
            Transform::from_translation(scene::light_position(i)),
        ));
    }
    for i in 0..scene::LAMPS {
        commands.spawn((
            Name::new(format!("lamp {i}")),
            BridgeLamp { i },
            LampLit(false),
            ChangedAt::default(),
            Transform::from_translation(scene::lamp_position(i)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_delays_are_scaled_and_sorted() {
        let s = schedule(&[
            CityEvent::Green { i: 1, delay: 8 },
            CityEvent::Green { i: 0, delay: 0 },
            CityEvent::Green { i: 2, delay: 4 },
        ]);
        let order: Vec<(f32, u32)> = s
            .iter()
            .map(|(t, e)| match e {
                CityEvent::Green { i, .. } => (*t, *i),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(order, vec![(0.0, 0), (1.0, 2), (2.0, 1)]);
    }

    #[test]
    fn lamps_light_one_beat_apart_and_huge_delays_are_capped() {
        let s = schedule(&[
            CityEvent::Lamp { i: 0, on: true },
            CityEvent::Lamp { i: 1, on: false },
            CityEvent::Green { i: 0, delay: 100_000 },
        ]);
        assert_eq!(s[0].0, 0.0);
        assert!((s[1].0 - LAMP_STAGGER).abs() < 1e-6);
        assert_eq!(s[2].0, MAX_WAIT);
    }
}
