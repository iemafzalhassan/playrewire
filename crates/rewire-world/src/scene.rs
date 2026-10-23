//! Street layout, in metres. The camera looks north (toward -z), so +x is to the right.
//!
//! Dock St runs east-west. Its 5 traffic lights stand on the far (north) kerb, with the
//! `dock-st-traffic` terminal on the pavement between lights 1 and 2, its screen facing
//! the road (and the camera). East of Dock St
//! a canal crosses the street; the Binary Bridge carries the road over it, with 8 lamps
//! on its far railing, read left to right (lamp 0 = the biggest binary digit). The
//! `bridge-panel` terminal stands at the west end of the bridge.
//!
//! Terminals sit where the follow camera already shows the whole wave or the whole
//! bridge, so a learner with reduced motion (no camera moves) still sees every light.

use bevy::prelude::*;

/// Traffic lights on Dock St.
pub const LIGHTS: u32 = 5;
/// Lamps on the Binary Bridge.
pub const LAMPS: u32 = 8;
/// A terminal opens only within this distance (metres, on the ground plane).
pub const TERMINAL_RANGE: f32 = 2.0;

/// Kerb lines of the road (z), and the far edge of each pavement.
pub const ROAD_HALF: f32 = 2.2;
pub const PAVEMENT_EDGE: f32 = 4.6;
/// The street's west and east ends.
pub const WEST_END: f32 = -24.0;
pub const EAST_END: f32 = 27.0;
/// The canal the bridge crosses (x range).
pub const CANAL: (f32, f32) = (9.0, 15.0);

pub fn light_position(i: u32) -> Vec3 {
    Vec3::new(-18.0 + i as f32 * 4.0, 0.0, -2.9)
}

pub fn lamp_position(i: u32) -> Vec3 {
    let (a, b) = CANAL;
    let span = b - a - 1.0;
    Vec3::new(a + 0.5 + span * i as f32 / (LAMPS - 1) as f32, 0.0, -ROAD_HALF - 0.15)
}

/// The middle of the wave and of the bridge: where the camera looks while events play.
pub fn lights_center() -> Vec3 {
    (light_position(0) + light_position(LIGHTS - 1)) / 2.0
}

pub fn lamps_center() -> Vec3 {
    (lamp_position(0) + lamp_position(LAMPS - 1)) / 2.0
}

/// A mission terminal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalSpot {
    pub id: &'static str,
    pub mission: &'static str,
    pub x: f32,
    pub z: f32,
    /// Where a player spawns for this mission: a short walk away.
    pub spawn: (f32, f32),
}

impl TerminalSpot {
    pub fn position(&self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.z)
    }

    /// Where "go to the terminal" walks to: on the road, facing the screen.
    pub fn approach(&self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.z - self.z.signum() * 1.3)
    }
}

pub const TERMINALS: [TerminalSpot; 2] = [
    TerminalSpot {
        id: "dock-st-traffic",
        mission: "green-wave",
        x: -12.0,
        z: -3.5,
        spawn: (-17.0, 0.6),
    },
    TerminalSpot {
        id: "bridge-panel",
        mission: "binary-bridge",
        x: 7.6,
        z: -3.5,
        spawn: (2.0, 0.6),
    },
];

pub fn terminal_for(mission: &str) -> Option<TerminalSpot> {
    TERMINALS.iter().copied().find(|t| t.mission == mission)
}

/// Where the player starts before any mission is loaded.
pub const DEFAULT_SPAWN: (f32, f32) = (-11.0, 0.6);

/// Can the player stand here? Road and pavements, the bridge deck, and nothing inside
/// a terminal kiosk or a light post.
pub fn walkable(p: Vec2) -> bool {
    let (x, z) = (p.x, p.y);
    let (a, b) = CANAL;
    let on_street = x > WEST_END + 0.5 && x < EAST_END - 0.5 && z.abs() < PAVEMENT_EDGE - 0.3;
    let over_canal = x > a - 0.1 && x < b + 0.1;
    let on_bridge = z.abs() < ROAD_HALF - 0.35;
    if !on_street || (over_canal && !on_bridge) {
        return false;
    }
    let solid = |c: Vec3, r: f32| Vec2::new(c.x, c.z).distance(p) < r;
    !(TERMINALS.iter().any(|t| solid(t.position(), 0.75))
        || (0..LIGHTS).any(|i| solid(light_position(i), 0.4)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lamps_read_left_to_right() {
        let xs: Vec<f32> = (0..LAMPS).map(|i| lamp_position(i).x).collect();
        assert!(xs.windows(2).all(|w| w[0] < w[1]));
        assert!(xs[0] > CANAL.0 && xs[7] < CANAL.1);
    }

    #[test]
    fn walkable_street_but_not_the_canal() {
        assert!(walkable(Vec2::new(0.0, 0.0)));
        assert!(walkable(Vec2::new(12.0, 0.0)), "on the bridge");
        assert!(!walkable(Vec2::new(12.0, 3.5)), "in the canal");
        assert!(!walkable(Vec2::new(0.0, 9.0)), "inside a building");
        let t = TERMINALS[0];
        assert!(!walkable(Vec2::new(t.x, t.z)), "inside the kiosk");
        assert!(walkable(Vec2::new(t.approach().x, t.approach().z)));
        for t in TERMINALS {
            let (x, z) = t.spawn;
            assert!(walkable(Vec2::new(x, z)));
            assert!(t.approach().distance(t.position()) < TERMINAL_RANGE);
        }
    }
}
