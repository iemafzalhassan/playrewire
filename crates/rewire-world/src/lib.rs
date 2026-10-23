//! rewire-world: the hand-inked PlayRewire street.
//!
//! Two plugins:
//! - [`WorldLogicPlugin`]: the street's rules (lights, lamps, terminals, the player,
//!   event playback, the page mailbox). Needs nothing but `MinimalPlugins`, so the
//!   tests run headless.
//! - [`WorldRenderPlugin`]: the window/canvas, the ink look and every mesh. It dresses
//!   the logical entities the logic plugin spawns.
//!
//! On wasm, `web.rs` exports the page API (`start`, `loadMission`, `applyEvents`, ...),
//! wrapped by `apps/web/src/world/world.ts`.

// Bevy systems take many params and long Query types by design.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use bevy::prelude::*;

pub mod bridge;
pub mod city_events;
pub mod ink;
pub mod mailbox;
pub mod player;
pub mod render;
pub mod scene;
pub mod terminals;
pub mod tokens;
#[cfg(target_arch = "wasm32")]
mod web;

pub use city_events::{
    BridgeLamp, CameraFocus, ChangedAt, Highlight, LampLit, Motion, Playback, Signal,
    TrafficLight,
};
pub use ink::{InkOutline, InkPlugin};
pub use mailbox::{Mailbox, Target, WorldCommand, WorldSignal};
pub use player::{InputEnabled, Intent, Player};
pub use render::WorldRenderPlugin;
pub use terminals::{ActiveMission, Terminal};

/// The whole street: window + look + rules. `canvas` is a CSS selector on the web.
pub fn build_app(canvas: Option<String>) -> App {
    let mut app = App::new();
    app.add_plugins((WorldRenderPlugin { canvas }, WorldLogicPlugin));
    app
}

/// Ordering inside a frame: commands first, then input, movement, terminals, playback.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum WorldSystems {
    Commands,
    Input,
    Move,
    Terminals,
    Playback,
}

/// The street's rules. Headless-safe.
pub struct WorldLogicPlugin;

impl Plugin for WorldLogicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Mailbox>()
            .init_resource::<Intent>()
            .init_resource::<InputEnabled>()
            .init_resource::<Motion>()
            .init_resource::<CameraFocus>()
            .init_resource::<Highlight>()
            .init_resource::<Playback>()
            .init_resource::<ActiveMission>()
            .init_resource::<terminals::NearTerminal>()
            .configure_sets(
                Update,
                (
                    WorldSystems::Commands,
                    WorldSystems::Input,
                    WorldSystems::Move,
                    WorldSystems::Terminals,
                    WorldSystems::Playback,
                )
                    .chain(),
            )
            .add_systems(
                Startup,
                (
                    city_events::spawn_city,
                    terminals::spawn_terminals,
                    player::spawn_player,
                ),
            )
            .add_systems(
                Update,
                (
                    (say_ready, handle_commands).chain().in_set(WorldSystems::Commands),
                    player::read_keyboard.in_set(WorldSystems::Input),
                    player::move_player.in_set(WorldSystems::Move),
                    terminals::use_terminals.in_set(WorldSystems::Terminals),
                    city_events::tick_playback.in_set(WorldSystems::Playback),
                ),
            );
    }
}

fn say_ready(mailbox: Res<Mailbox>, mut said: Local<bool>) {
    if !*said {
        *said = true;
        mailbox.emit(WorldSignal::Ready);
    }
}

fn handle_commands(
    time: Res<Time>,
    mailbox: Res<Mailbox>,
    mut motion: ResMut<Motion>,
    mut enabled: ResMut<InputEnabled>,
    mut intent: ResMut<Intent>,
    mut focus: ResMut<CameraFocus>,
    mut highlight: ResMut<Highlight>,
    mut playback: ResMut<Playback>,
    mut active: ResMut<ActiveMission>,
    mut players: Query<(&mut Transform, &mut Player)>,
    mut lights: Query<(&TrafficLight, &mut Signal, &mut ChangedAt)>,
    mut lamps: Query<(&BridgeLamp, &mut LampLit, &mut ChangedAt), Without<TrafficLight>>,
) {
    for cmd in mailbox.take_commands() {
        match cmd {
            WorldCommand::LoadMission(id) => {
                if let Some(t) = scene::terminal_for(&id)
                    && let Ok((mut transform, _)) = players.single_mut()
                {
                    let (x, z) = t.spawn;
                    transform.translation = Vec3::new(x, 0.0, z);
                    transform.rotation = Quat::IDENTITY;
                }
                active.0 = Some(id);
                *intent = Intent::default();
                focus.0 = None;
                highlight.0 = None;
            }
            WorldCommand::ApplyEvents(events) => city_events::start_playback(
                events,
                time.elapsed_secs(),
                *motion,
                &mut playback,
                &mut focus,
                &mut highlight,
                &mut lights,
                &mut lamps,
                &mailbox,
            ),
            WorldCommand::SetReducedMotion(on) => {
                motion.reduced = on;
                if on {
                    focus.0 = None;
                }
            }
            WorldCommand::SetInputEnabled(on) => {
                enabled.0 = on;
                if !on {
                    intent.move_dir = Vec2::ZERO;
                }
            }
            WorldCommand::GoToTerminal => {
                player::go_to_terminal(active.0.as_deref(), *motion, &mut intent, &mut players)
            }
            WorldCommand::Highlight(target) => highlight.0 = target,
        }
    }
}
