//! The player: WASD / arrow keys, click or tap to walk, and an auto-walk to the
//! terminal for keyboard-only and assistive play.

use bevy::prelude::*;

use crate::{
    city_events::{CameraFocus, Motion},
    scene,
};

/// Walking speed, metres per second.
pub const SPEED: f32 = 4.5;

#[derive(Component, Debug, Default)]
pub struct Player {
    /// Distance walked; drives the little walking bob.
    pub stride: f32,
}

/// What the player wants this frame. Keyboard, pointer and the page all write here, so
/// the movement rules are the same however the learner plays (and testable headless).
#[derive(Resource, Debug, Default)]
pub struct Intent {
    /// Walking direction on the ground (x, z); zero when standing.
    pub move_dir: Vec2,
    /// Walking somewhere (a click, a tap, or "go to the terminal").
    pub walk_to: Option<Vec3>,
    /// Open the live terminal on arrival.
    pub open_on_arrival: bool,
    /// E / Enter / a tap on the terminal this frame (consumed by the terminal system).
    pub interact: bool,
}

/// Off while the editor overlay is open, so keys typed there never move the player.
#[derive(Resource, Debug, Clone, Copy)]
pub struct InputEnabled(pub bool);

impl Default for InputEnabled {
    fn default() -> Self {
        Self(true)
    }
}

pub(crate) fn spawn_player(mut commands: Commands) {
    let (x, z) = scene::DEFAULT_SPAWN;
    commands.spawn((Name::new("player"), Player::default(), Transform::from_xyz(x, 0.0, z)));
}

/// Keys only reach the world while the canvas has focus (winit listens on the canvas),
/// and only while input is enabled.
pub(crate) fn read_keyboard(
    keys: Option<Res<ButtonInput<KeyCode>>>,
    enabled: Res<InputEnabled>,
    mut intent: ResMut<Intent>,
) {
    let Some(keys) = keys else {
        return;
    };
    if !enabled.0 {
        intent.move_dir = Vec2::ZERO;
        return;
    }
    let held = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let mut dir = Vec2::ZERO;
    if held(KeyCode::KeyW, KeyCode::ArrowUp) {
        dir.y -= 1.0;
    }
    if held(KeyCode::KeyS, KeyCode::ArrowDown) {
        dir.y += 1.0;
    }
    if held(KeyCode::KeyA, KeyCode::ArrowLeft) {
        dir.x -= 1.0;
    }
    if held(KeyCode::KeyD, KeyCode::ArrowRight) {
        dir.x += 1.0;
    }
    intent.move_dir = dir;
    if keys.just_pressed(KeyCode::KeyE)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
    {
        intent.interact = true;
    }
}

/// Moves along x and z separately so the player slides along kerbs and kiosks.
fn step(from: Vec3, delta: Vec2) -> Vec3 {
    let mut p = Vec2::new(from.x, from.z);
    let try_x = p + Vec2::new(delta.x, 0.0);
    if scene::walkable(try_x) {
        p = try_x;
    }
    let try_z = p + Vec2::new(0.0, delta.y);
    if scene::walkable(try_z) {
        p = try_z;
    }
    Vec3::new(p.x, from.y, p.y)
}

pub(crate) fn move_player(
    time: Res<Time>,
    enabled: Res<InputEnabled>,
    mut intent: ResMut<Intent>,
    mut focus: ResMut<CameraFocus>,
    mut players: Query<(&mut Transform, &mut Player)>,
) {
    let Ok((mut transform, mut player)) = players.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let manual = if enabled.0 { intent.move_dir } else { Vec2::ZERO };
    let pos = transform.translation;

    let dir = if manual != Vec2::ZERO {
        // The learner took the wheel: drop any auto-walk, follow them again.
        intent.walk_to = None;
        intent.open_on_arrival = false;
        focus.0 = None;
        manual.normalize()
    } else if let Some(target) = intent.walk_to {
        let to = Vec2::new(target.x - pos.x, target.z - pos.z);
        if to.length() <= SPEED * dt.max(1e-3) {
            if scene::walkable(Vec2::new(target.x, target.z)) {
                transform.translation = Vec3::new(target.x, pos.y, target.z);
            }
            intent.walk_to = None;
            if intent.open_on_arrival {
                intent.open_on_arrival = false;
                intent.interact = true;
            }
            return;
        }
        to.normalize()
    } else {
        return;
    };
    if dt <= 0.0 {
        return;
    }
    let next = step(pos, dir * SPEED * dt);
    if next == pos && intent.walk_to.is_some() {
        // Walked into something: give up rather than push forever.
        intent.walk_to = None;
        intent.open_on_arrival = false;
    }
    player.stride += next.distance(pos);
    transform.translation = next;
    let facing = Vec3::new(dir.x, 0.0, dir.y);
    transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, facing.normalize());
}

/// "Go to the terminal": walk there and open it. Reduced motion just puts the player there.
pub(crate) fn go_to_terminal(
    mission: Option<&str>,
    motion: Motion,
    intent: &mut Intent,
    players: &mut Query<(&mut Transform, &mut Player)>,
) {
    let Some(t) = mission.and_then(scene::terminal_for) else {
        return;
    };
    if motion.reduced {
        if let Ok((mut transform, _)) = players.single_mut() {
            transform.translation = t.approach();
            transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, (t.position() - t.approach()).normalize());
        }
        intent.walk_to = None;
        intent.open_on_arrival = false;
        intent.interact = true;
    } else {
        intent.walk_to = Some(t.approach());
        intent.open_on_arrival = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliding_along_the_kerb() {
        // Pushing diagonally into the north pavement edge still moves east.
        let from = Vec3::new(0.0, 0.0, -(scene::PAVEMENT_EDGE - 0.35));
        let to = step(from, Vec2::new(0.5, -0.5));
        assert!(to.x > from.x);
        assert_eq!(to.z, from.z);
    }
}
