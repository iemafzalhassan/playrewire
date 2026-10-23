//! Window/canvas, the ink look and every mesh. The logic plugin spawns bare entities
//! (lights, lamps, terminals, the player); the systems here dress and animate them.
//!
//! HiDPI: the canvas follows its parent (`fit_canvas_to_parent`) and there is no scale
//! factor override, so winit sizes the backing store to CSS size x devicePixelRatio and
//! keeps it in step on resize. (The spike forced 1280x720 at scale 1, which WebKitGTK at
//! DPR 2 drew into the top-left quarter.) The ink pen width follows the scale factor.

use std::f32::consts::{PI, TAU};

use bevy::{
    asset::RenderAssetUsages,
    camera::Exposure,
    core_pipeline::prepass::NormalPrepass,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::{
        render_resource::TextureUsages,
        view::{DebandDither, Msaa, Tonemapping},
    },
    window::PrimaryWindow,
    winit::{UpdateMode, WinitSettings},
};

use crate::{
    WorldSystems,
    bridge,
    city_events::{BridgeLamp, CameraFocus, ChangedAt, Highlight, LampLit, Motion, Signal, TrafficLight},
    ink::{InkOutline, InkPlugin},
    mailbox::Target,
    player::{InputEnabled, Intent, Player},
    scene::{self, CANAL, EAST_END, PAVEMENT_EDGE, ROAD_HALF, WEST_END},
    terminals::{ActiveMission, Terminal, in_range},
    tokens::{self, color},
};

/// The street's window and look. `canvas` is a CSS selector (`#id`) on the web.
pub struct WorldRenderPlugin {
    pub canvas: Option<String>,
}

impl Plugin for WorldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Rewire street".into(),
                    canvas: self.canvas.clone(),
                    fit_canvas_to_parent: true,
                    transparent: true,
                    // Tab must still leave the canvas; world.ts stops arrow keys scrolling.
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            }),
            InkPlugin,
        ))
        .insert_resource(ClearColor(Color::NONE))
        // Keep animating while focus is in the editor overlay (the canvas is "unfocused").
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .init_resource::<ScreenLabels>()
        .add_systems(
            Startup,
            (
                setup_palette,
                (spawn_scenery, bridge::spawn_bridge, spawn_camera, spawn_ring, spawn_marker),
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                (dress_lights, bridge::dress_lamps, dress_terminals, dress_player),
                click_to_walk.in_set(WorldSystems::Input),
                (
                    show_lights,
                    bridge::show_lamps,
                    show_terminals,
                    animate_player,
                    camera_follow,
                    (show_ring, ink_pen, screen_labels).chain(),
                )
                    .after(WorldSystems::Playback),
            ),
        );
    }
}

/// Every material the street uses, made once from the design tokens.
#[derive(Resource, Clone)]
pub struct Palette {
    pub ground: Handle<StandardMaterial>,
    pub road: Handle<StandardMaterial>,
    pub marking: Handle<StandardMaterial>,
    pub kerb: Handle<StandardMaterial>,
    pub water: Handle<StandardMaterial>,
    pub card: Handle<StandardMaterial>,
    pub paper: Handle<StandardMaterial>,
    pub ink_soft: Handle<StandardMaterial>,
    pub pencil: Handle<StandardMaterial>,
    pub ink: Handle<StandardMaterial>,
    pub accent: Handle<StandardMaterial>,
    pub accent_deep: Handle<StandardMaterial>,
    pub highlight: Handle<StandardMaterial>,
    pub shadow: Handle<StandardMaterial>,
    pub red_on: Handle<StandardMaterial>,
    pub red_off: Handle<StandardMaterial>,
    pub green_on: Handle<StandardMaterial>,
    pub green_off: Handle<StandardMaterial>,
    pub lamp_on: Handle<StandardMaterial>,
    pub lamp_off: Handle<StandardMaterial>,
    pub screen_live: Handle<StandardMaterial>,
    pub screen_dark: Handle<StandardMaterial>,
    pub coat: Handle<StandardMaterial>,
    pub walls: Vec<Handle<StandardMaterial>>,
    pub roofs: Vec<Handle<StandardMaterial>>,
    pub leaf: Handle<StandardMaterial>,
}

fn unlit(materials: &mut Assets<StandardMaterial>, c: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: c,
        unlit: true,
        ..default()
    })
}

fn setup_palette(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    use tokens::*;
    let m = &mut *materials;
    let walls = vec![
        unlit(m, color(CARD)),
        unlit(m, mix(PAPER, HIGHLIGHT, 0.45)),
        unlit(m, mix(PAPER, OK, 0.22)),
        unlit(m, mix(PAPER, ACCENT, 0.25)),
        unlit(m, mix(PAPER, FOCUS, 0.18)),
        unlit(m, mix(CARD, PAPER_DEEP, 0.6)),
    ];
    let roofs = vec![
        unlit(m, mix(ACCENT_DEEP, PAPER, 0.25)),
        unlit(m, mix(INK_SOFT, PAPER, 0.35)),
        unlit(m, mix(OK, INK_SOFT, 0.35)),
    ];
    let p = Palette {
        ground: unlit(m, color(PAPER_DEEP)),
        road: unlit(m, mix(PAPER_DEEP, PENCIL, 0.42)),
        marking: unlit(m, color(CARD)),
        kerb: unlit(m, mix(CARD, PAPER_DEEP, 0.4)),
        water: unlit(m, tokens::water()),
        card: unlit(m, color(CARD)),
        paper: unlit(m, color(PAPER)),
        ink_soft: unlit(m, color(INK_SOFT)),
        pencil: unlit(m, color(PENCIL)),
        ink: unlit(m, color(INK)),
        accent: unlit(m, color(ACCENT)),
        accent_deep: unlit(m, color(ACCENT_DEEP)),
        highlight: unlit(m, color(HIGHLIGHT)),
        shadow: unlit(m, mix(PAPER_DEEP, INK, 0.22)),
        red_on: unlit(m, color(ACCENT)),
        red_off: unlit(m, mix(ACCENT, INK, 0.7)),
        green_on: unlit(m, tokens::signal_green()),
        green_off: unlit(m, mix(OK, INK, 0.6)),
        lamp_on: unlit(m, color(HIGHLIGHT)),
        lamp_off: unlit(m, mix(PENCIL, PAPER, 0.25)),
        screen_live: unlit(m, mix(OK, INK, 0.25)),
        screen_dark: unlit(m, mix(INK_SOFT, INK, 0.4)),
        coat: unlit(m, mix(FOCUS, PAPER, 0.3)),
        leaf: unlit(m, mix(OK, PAPER, 0.45)),
        walls,
        roofs,
    };
    commands.insert_resource(p);
}

/// A box by size and centre.
pub(crate) fn bx(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
    size: Vec3,
    center: Vec3,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(center),
        ))
        .id()
}

/// Paint on the ground: a flat plane with no sides, so the ink pass never outlines it.
pub(crate) fn decal(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mat: &Handle<StandardMaterial>,
    size: Vec2,
    center: Vec3,
) -> Entity {
    commands
        .spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(size.x, size.y))),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(center),
        ))
        .id()
}

/// A tiny deterministic "random" so the street is the same every time.
fn jitter(n: u32) -> f32 {
    let x = (n as f32 * 12.9898).sin() * 43758.547;
    x - x.floor()
}

fn spawn_scenery(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, palette: Res<Palette>) {
    let p = &*palette;
    let c = &mut commands;
    let m = &mut *meshes;
    let (ca, cb) = CANAL;

    // The diorama: two ground slabs either side of the canal, water in between and at
    // the dock end of Dock St.
    let depth = (-13.0, 10.0);
    let dz = depth.1 - depth.0;
    let zc = (depth.0 + depth.1) / 2.0;
    bx(c, m, &p.ground, Vec3::new(ca - WEST_END, 0.8, dz), Vec3::new((WEST_END + ca) / 2.0, -0.4, zc));
    bx(c, m, &p.ground, Vec3::new(EAST_END - cb, 0.8, dz), Vec3::new((cb + EAST_END) / 2.0, -0.4, zc));
    bx(c, m, &p.water, Vec3::new(cb - ca, 0.3, dz), Vec3::new((ca + cb) / 2.0, -0.6, zc));
    bx(c, m, &p.water, Vec3::new(9.0, 0.3, dz), Vec3::new(WEST_END - 4.5, -0.6, zc));

    // Road (painted flat: the ink pass draws no line where it meets the ground), kerbs
    // (real steps, so they get a clean line) and the dashed centre line.
    for (a, b) in [(WEST_END, ca), (cb, EAST_END)] {
        let len = b - a;
        let x = (a + b) / 2.0;
        decal(c, m, &p.road, Vec2::new(len, ROAD_HALF * 2.0), Vec3::new(x, 0.004, 0.0));
        for side in [-1.0, 1.0] {
            bx(c, m, &p.kerb, Vec3::new(len, 0.14, 0.22), Vec3::new(x, 0.07, side * (ROAD_HALF + 0.11)));
        }
        let mut dx = a + 1.2;
        while dx + 1.4 < b {
            decal(c, m, &p.marking, Vec2::new(1.4, 0.16), Vec3::new(dx + 0.7, 0.008, 0.0));
            dx += 3.2;
        }
    }
    // A zebra crossing by the Dock St terminal.
    for k in 0..6 {
        let z = -ROAD_HALF + 0.45 + k as f32 * 0.68;
        decal(c, m, &p.marking, Vec2::new(1.8, 0.36), Vec3::new(-12.0, 0.008, z));
    }

    // North side of Dock St: a row of hand-drawn houses.
    let widths = [4.4, 3.6, 5.0, 4.0, 3.4, 5.2, 4.4];
    let heights = [5.2, 7.6, 4.4, 6.4, 8.6, 5.0, 6.8];
    let mut x = WEST_END + 0.6;
    for (k, (w, h)) in widths.iter().zip(heights).enumerate() {
        let k = k as u32;
        house(c, m, p, Vec3::new(x + w / 2.0, 0.0, -PAVEMENT_EDGE - 2.7), Vec2::new(*w, 4.6), h, k);
        x += w + 0.25;
    }
    // Silicon Quarter, east of the canal: a chip-shaped hall and a tower.
    let chip = Vec3::new(20.5, 0.0, -8.0);
    bx(c, m, &p.ink_soft, Vec3::new(8.0, 1.8, 5.2), chip + Vec3::Y * 0.9);
    bx(c, m, &p.pencil, Vec3::new(5.0, 0.2, 3.0), chip + Vec3::new(0.0, 1.9, 0.0));
    for k in 0..9 {
        let px = chip.x - 3.6 + k as f32 * 0.9;
        bx(c, m, &p.card, Vec3::new(0.3, 0.5, 0.5), Vec3::new(px, 0.25, chip.z + 2.85));
    }
    house(c, m, p, Vec3::new(25.0, 0.0, -8.0), Vec2::new(3.4, 4.0), 9.0, 11);

    // South side: low dockside things that never hide the player.
    for k in 0..7 {
        let x = WEST_END + 2.0 + k as f32 * 4.6 + jitter(k) * 1.2;
        if x > ca - 1.0 {
            break;
        }
        match k % 3 {
            0 => {
                let s = 0.7 + jitter(k + 9) * 0.3;
                bx(c, m, &p.walls[1], Vec3::splat(s), Vec3::new(x, s / 2.0, PAVEMENT_EDGE + 0.8));
                bx(c, m, &p.walls[3], Vec3::splat(s * 0.8), Vec3::new(x + 0.9, s * 0.4, PAVEMENT_EDGE + 1.1));
            }
            1 => tree(c, m, p, Vec3::new(x, 0.0, PAVEMENT_EDGE + 1.4)),
            _ => {
                for b in 0..3 {
                    let bx_ = x + b as f32 * 0.9;
                    c.spawn((
                        Mesh3d(m.add(Cylinder::new(0.16, 0.6))),
                        MeshMaterial3d(p.ink_soft.clone()),
                        Transform::from_xyz(bx_, 0.3, ROAD_HALF + 0.6),
                    ));
                }
            }
        }
    }
    for k in 0..3 {
        tree(c, m, p, Vec3::new(cb + 2.5 + k as f32 * 3.6, 0.0, PAVEMENT_EDGE + 1.6));
    }

    // The dock: a jetty and a little boat.
    bx(c, m, &p.kerb, Vec3::new(5.0, 0.2, 1.4), Vec3::new(WEST_END - 2.5, -0.05, -1.5));
    let boat = Vec3::new(WEST_END - 3.5, -0.3, 1.6);
    bx(c, m, &p.walls[3], Vec3::new(3.2, 0.6, 1.3), boat);
    bx(c, m, &p.card, Vec3::new(1.2, 0.7, 0.9), boat + Vec3::new(-0.4, 0.65, 0.0));
}

fn house(c: &mut Commands, m: &mut Assets<Mesh>, p: &Palette, base: Vec3, foot: Vec2, h: f32, k: u32) {
    let wall = &p.walls[k as usize % p.walls.len()];
    let roof = &p.roofs[k as usize % p.roofs.len()];
    bx(c, m, wall, Vec3::new(foot.x, h, foot.y), base + Vec3::Y * h / 2.0);
    // A roof lip, a chimney on some.
    bx(c, m, roof, Vec3::new(foot.x + 0.3, 0.35, foot.y + 0.3), base + Vec3::Y * (h + 0.17));
    if k.is_multiple_of(2) {
        bx(c, m, roof, Vec3::new(0.5, 1.0, 0.5), base + Vec3::new(foot.x * 0.25, h + 0.8, -0.6));
    }
    let front = base.z + foot.y / 2.0 + 0.04;
    // Ground floor: a door and a shop window with a striped awning.
    bx(c, m, &p.ink_soft, Vec3::new(0.9, 1.8, 0.08), Vec3::new(base.x - foot.x * 0.3, 0.9, front));
    let shop = Vec3::new(base.x + foot.x * 0.15, 1.1, front);
    bx(c, m, &p.highlight, Vec3::new(foot.x * 0.42, 1.0, 0.08), shop);
    let stripes = 4;
    let sw = foot.x * 0.5 / stripes as f32;
    for s in 0..stripes {
        let mat = if s % 2 == 0 { &p.accent } else { &p.card };
        let e = bx(
            c,
            m,
            mat,
            Vec3::new(sw, 0.06, 0.8),
            Vec3::new(shop.x - foot.x * 0.25 + sw * (s as f32 + 0.5), 1.95, front + 0.35),
        );
        c.entity(e).insert(Transform {
            translation: Vec3::new(shop.x - foot.x * 0.25 + sw * (s as f32 + 0.5), 1.95, front + 0.35),
            rotation: Quat::from_rotation_x(0.35),
            ..default()
        });
    }
    // Upper windows, a few lit.
    let cols = ((foot.x - 0.6) / 1.3).floor().max(1.0) as u32;
    let rows = ((h - 2.6) / 1.6).floor().max(0.0) as u32;
    for r in 0..rows {
        for col in 0..cols {
            let n = k * 31 + r * 7 + col;
            let wx = base.x - foot.x / 2.0 + (foot.x / cols as f32) * (col as f32 + 0.5);
            let wy = 3.0 + r as f32 * 1.6;
            let mat = if jitter(n) > 0.7 { &p.highlight } else { &p.card };
            bx(c, m, mat, Vec3::new(0.62, 0.85, 0.1), Vec3::new(wx, wy, front));
        }
    }
}

fn tree(c: &mut Commands, m: &mut Assets<Mesh>, p: &Palette, at: Vec3) {
    c.spawn((
        Mesh3d(m.add(Cylinder::new(0.12, 1.2))),
        MeshMaterial3d(p.ink_soft.clone()),
        Transform::from_translation(at + Vec3::Y * 0.6),
    ));
    c.spawn((
        Mesh3d(m.add(Sphere::new(0.85).mesh().ico(1).unwrap())),
        MeshMaterial3d(p.leaf.clone()),
        Transform::from_translation(at + Vec3::Y * 1.75).with_scale(Vec3::new(1.0, 0.9, 1.0)),
    ));
}

/// The parts of one traffic light that change.
#[derive(Component)]
pub struct LightParts {
    red: Entity,
    green: Entity,
    rays: Entity,
    halo: Entity,
}

/// Traffic lights are drawn a little larger than life, so the wave reads from afar.
pub const LIGHT_SCALE: f32 = 1.4;

fn dress_lights(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    palette: Res<Palette>,
    mut added: Query<(Entity, &mut Transform), Added<TrafficLight>>,
) {
    for (e, mut tf) in &mut added {
        // Only the drawing grows; the logical position stays where the logic put it.
        tf.scale = Vec3::splat(LIGHT_SCALE);
        let p = &*palette;
        let disc = meshes.add(Cylinder::new(0.19, 0.06));
        let facing = Quat::from_rotation_x(PI / 2.0);
        let mut red = Entity::PLACEHOLDER;
        let mut green = Entity::PLACEHOLDER;
        let mut rays = Entity::PLACEHOLDER;
        let mut halo = Entity::PLACEHOLDER;
        commands.entity(e).insert(Visibility::default()).with_children(|c| {
            halo = c
                .spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.29, 0.01))),
                    MeshMaterial3d(p.highlight.clone()),
                    Transform::from_xyz(0.0, 2.63, 0.176).with_rotation(facing),
                    Visibility::Hidden,
                ))
                .id();
            c.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.07, 2.6))),
                MeshMaterial3d(p.ink_soft.clone()),
                Transform::from_xyz(0.0, 1.3, 0.0),
            ));
            c.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.56, 1.12, 0.34))),
                MeshMaterial3d(p.pencil.clone()),
                Transform::from_xyz(0.0, 2.9, 0.0),
            ));
            for y in [3.17, 2.63] {
                c.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.46, 0.05, 0.16))),
                    MeshMaterial3d(p.ink_soft.clone()),
                    Transform::from_xyz(0.0, y + 0.23, 0.24),
                ));
            }
            red = c
                .spawn((
                    Mesh3d(disc.clone()),
                    MeshMaterial3d(p.red_on.clone()),
                    Transform::from_xyz(0.0, 3.17, 0.19).with_rotation(facing),
                ))
                .id();
            green = c
                .spawn((
                    Mesh3d(disc.clone()),
                    MeshMaterial3d(p.green_off.clone()),
                    Transform::from_xyz(0.0, 2.63, 0.19).with_rotation(facing),
                ))
                .id();
            // Pen rays round a lit green: "on" is never colour alone.
            rays = c
                .spawn((Transform::from_xyz(0.0, 2.63, 0.22), Visibility::Hidden))
                .with_children(|r| {
                    let ray = meshes.add(Cuboid::new(0.07, 0.24, 0.04));
                    for k in 0..8 {
                        let a = k as f32 * TAU / 8.0 + TAU / 16.0;
                        r.spawn((
                            Mesh3d(ray.clone()),
                            MeshMaterial3d(p.green_on.clone()),
                            Transform::from_xyz(a.sin() * 0.48, a.cos() * 0.48, 0.0)
                                .with_rotation(Quat::from_rotation_z(-a)),
                        ));
                    }
                })
                .id();
        });
        commands.entity(e).insert(LightParts { red, green, rays, halo });
    }
}

/// 0 -> 1 with a little overshoot: a lamp "pops" on.
pub(crate) fn pop(t: f32) -> f32 {
    if t >= 1.0 {
        return 1.0;
    }
    let t = t.max(0.0);
    let s = 2.2;
    let u = t - 1.0;
    1.0 + u * u * ((s + 1.0) * u + s)
}

fn show_lights(
    time: Res<Time>,
    motion: Res<Motion>,
    palette: Res<Palette>,
    lights: Query<(&Signal, &ChangedAt, &LightParts)>,
    mut mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut parts: Query<(&mut Transform, &mut Visibility), Without<TrafficLight>>,
) {
    let now = time.elapsed_secs();
    for (signal, at, lp) in &lights {
        let green = *signal == Signal::Green;
        if let Ok(mut m) = mats.get_mut(lp.red) {
            m.0 = if green { palette.red_off.clone() } else { palette.red_on.clone() };
        }
        if let Ok(mut m) = mats.get_mut(lp.green) {
            m.0 = if green { palette.green_on.clone() } else { palette.green_off.clone() };
        }
        let s = if motion.reduced { 1.0 } else { pop((now - at.0) / 0.35) };
        if let Ok((mut t, mut v)) = parts.get_mut(lp.rays) {
            *v = if green { Visibility::Inherited } else { Visibility::Hidden };
            let spin = if motion.reduced { 0.0 } else { (now * 0.6).sin() * 0.12 };
            t.scale = Vec3::splat(s.max(0.01));
            t.rotation = Quat::from_rotation_z(spin);
        }
        if let Ok((mut t, mut v)) = parts.get_mut(lp.halo) {
            *v = if green { Visibility::Inherited } else { Visibility::Hidden };
            t.scale = Vec3::new(s.max(0.01), 1.0, s.max(0.01));
        }
        if let Ok((mut t, _)) = parts.get_mut(lp.green) {
            let k = if green { 0.8 + 0.2 * s } else { 1.0 };
            t.scale = Vec3::new(k, 1.0, k);
        }
    }
}

/// A terminal's changing parts.
#[derive(Component)]
pub struct TerminalParts {
    screen: Entity,
    cursor: Entity,
}

fn dress_terminals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    palette: Res<Palette>,
    added: Query<Entity, Added<Terminal>>,
) {
    for e in &added {
        let p = &*palette;
        let tilt = Quat::from_rotation_x(-0.45);
        let mut screen = Entity::PLACEHOLDER;
        let mut cursor = Entity::PLACEHOLDER;
        commands.entity(e).insert(Visibility::default()).with_children(|c| {
            c.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.1, 0.6))),
                MeshMaterial3d(p.walls[0].clone()),
                Transform::from_xyz(0.0, 0.55, 0.0),
            ));
            c.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.1, 0.12, 0.7))),
                MeshMaterial3d(p.accent_deep.clone()),
                Transform::from_xyz(0.0, 1.16, 0.0),
            ));
            // A slanted desk with the screen, facing the road.
            c.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 0.08, 0.7))),
                MeshMaterial3d(p.ink_soft.clone()),
                Transform::from_xyz(0.0, 1.32, 0.12).with_rotation(tilt),
            ));
            screen = c
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.8, 0.04, 0.5))),
                    MeshMaterial3d(p.screen_dark.clone()),
                    Transform::from_xyz(0.0, 1.38, 0.14).with_rotation(tilt),
                ))
                .id();
            cursor = c
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.14, 0.03, 0.07))),
                    MeshMaterial3d(p.highlight.clone()),
                    Transform::from_xyz(-0.22, 1.42, 0.2).with_rotation(tilt),
                    Visibility::Hidden,
                ))
                .id();
            // A sign on a pole.
            c.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.04, 2.4))),
                MeshMaterial3d(p.ink_soft.clone()),
                Transform::from_xyz(0.62, 1.2, -0.2),
            ));
            c.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.9, 0.4, 0.06))),
                MeshMaterial3d(p.highlight.clone()),
                Transform::from_xyz(0.95, 2.2, -0.2),
            ));
        });
        commands.entity(e).insert(TerminalParts { screen, cursor });
    }
}

/// The bobbing "go here" marker over the live terminal.
#[derive(Component)]
pub struct Marker;

fn spawn_marker(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, palette: Res<Palette>) {
    commands.spawn((
        Marker,
        Mesh3d(meshes.add(Cone::new(0.32, 0.7))),
        MeshMaterial3d(palette.accent.clone()),
        Transform::from_xyz(0.0, -50.0, 0.0),
        Visibility::Hidden,
    ));
}

fn show_terminals(
    time: Res<Time>,
    motion: Res<Motion>,
    palette: Res<Palette>,
    active: Res<ActiveMission>,
    terminals: Query<(&Terminal, &Transform, &TerminalParts)>,
    players: Query<&Transform, (With<Player>, Without<Terminal>, Without<Marker>)>,
    mut mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut vis: Query<&mut Visibility, (Without<Marker>, Without<Terminal>)>,
    mut marker: Query<(&mut Transform, &mut Visibility), (With<Marker>, Without<Terminal>, Without<Player>)>,
) {
    let now = time.elapsed_secs();
    let player = players.single().map(|t| t.translation).unwrap_or_default();
    let Ok((mut mt, mut mv)) = marker.single_mut() else {
        return;
    };
    *mv = Visibility::Hidden;
    for (t, tf, parts) in &terminals {
        let live = active.0.as_deref() == Some(t.mission);
        if let Ok(mut m) = mats.get_mut(parts.screen) {
            m.0 = if live { palette.screen_live.clone() } else { palette.screen_dark.clone() };
        }
        let blink = motion.reduced || (now * 2.0).fract() < 0.6;
        if let Ok(mut v) = vis.get_mut(parts.cursor) {
            *v = if live && blink { Visibility::Inherited } else { Visibility::Hidden };
        }
        if live && !in_range(player, t.mission) {
            *mv = Visibility::Visible;
            let bob = if motion.reduced { 0.0 } else { (now * 2.4).sin() * 0.14 };
            mt.translation = tf.translation + Vec3::new(0.0, 2.9 + bob, 0.0);
            // Point down at the kiosk, turning slowly.
            let turn = if motion.reduced { 0.0 } else { now * 1.2 };
            mt.rotation = Quat::from_rotation_y(turn) * Quat::from_rotation_x(PI);
        }
    }
}

/// The player's drawn body (a child, so it can bob without moving the logical position).
#[derive(Component)]
pub struct Body;

fn dress_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    palette: Res<Palette>,
    added: Query<Entity, Added<Player>>,
) {
    for e in &added {
        let p = &*palette;
        commands.entity(e).insert(Visibility::default()).with_children(|c| {
            c.spawn((
                Mesh3d(meshes.add(Circle::new(0.5))),
                MeshMaterial3d(p.shadow.clone()),
                Transform::from_xyz(0.0, 0.012, 0.0)
                    .with_rotation(Quat::from_rotation_x(-PI / 2.0))
                    .with_scale(Vec3::new(1.0, 0.75, 1.0)),
            ));
            c.spawn((Body, Transform::default(), Visibility::default())).with_children(|b| {
                b.spawn((
                    Mesh3d(meshes.add(Capsule3d::new(0.3, 0.6))),
                    MeshMaterial3d(p.coat.clone()),
                    Transform::from_xyz(0.0, 0.62, 0.0),
                ));
                b.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.24).mesh().uv(16, 10))),
                    MeshMaterial3d(p.card.clone()),
                    Transform::from_xyz(0.0, 1.38, 0.0),
                ));
                b.spawn((
                    Mesh3d(meshes.add(Torus::new(0.2, 0.33))),
                    MeshMaterial3d(p.accent.clone()),
                    Transform::from_xyz(0.0, 1.12, 0.0),
                ));
                // A satchel on the back, so you can see which way they face.
                b.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.36, 0.42, 0.18))),
                    MeshMaterial3d(p.walls[1].clone()),
                    Transform::from_xyz(0.0, 0.75, 0.3),
                ));
            });
        });
    }
}

fn animate_player(
    motion: Res<Motion>,
    players: Query<&Player>,
    mut bodies: Query<&mut Transform, With<Body>>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    for mut t in &mut bodies {
        t.translation.y = if motion.reduced { 0.0 } else { ((player.stride * 3.4).sin().abs()) * 0.08 };
    }
}

/// Smoothed camera state.
#[derive(Component, Default)]
pub struct CameraRig {
    look: Vec3,
    dist: f32,
    placed: bool,
}

/// The camera looks down at the street from the south, about 34 degrees.
const VIEW_DIR: Vec3 = Vec3::new(0.0, 0.55, 0.835);
const FOV: f32 = 0.62;

/// Distance at which `width` metres fit across the view.
fn fit(width: f32, aspect: f32) -> f32 {
    (width / 2.0) / ((FOV / 2.0).tan() * aspect.max(0.3))
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d {
            // The ink pass reads the main depth texture directly (WebGL2 has no DepthPrepass).
            depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING).into(),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection { fov: FOV, ..default() }),
        Exposure::default(),
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        NormalPrepass,
        InkOutline::default(),
        CameraRig::default(),
        Transform::from_xyz(0.0, 9.0, 14.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn camera_follow(
    time: Res<Time>,
    motion: Res<Motion>,
    focus: Res<CameraFocus>,
    windows: Query<&Window, With<PrimaryWindow>>,
    players: Query<&Transform, (With<Player>, Without<CameraRig>)>,
    mut cams: Query<(&mut Transform, &mut CameraRig)>,
) {
    let Ok((mut tf, mut rig)) = cams.single_mut() else {
        return;
    };
    let aspect = windows
        .single()
        .map(|w| w.width() / w.height().max(1.0))
        .unwrap_or(16.0 / 9.0);
    let (look, dist) = match focus.0 {
        Some(c) => (c + Vec3::new(0.0, 1.4, 1.0), fit(focus.1.max(8.0), aspect).clamp(9.0, 32.0)),
        None => {
            let p = players.single().map(|t| t.translation).unwrap_or_default();
            (p + Vec3::new(0.0, 0.8, -1.8), fit(22.0, aspect).clamp(14.5, 30.0))
        }
    };
    if motion.reduced || !rig.placed {
        rig.look = look;
        rig.dist = dist;
        rig.placed = true;
    } else {
        let k = 1.0 - (-time.delta_secs() * 3.5).exp();
        rig.look = rig.look.lerp(look, k);
        rig.dist += (dist - rig.dist) * k;
    }
    *tf = Transform::from_translation(rig.look + VIEW_DIR.normalize() * rig.dist).looking_at(rig.look, Vec3::Y);
}

/// Click or tap the street to walk there; tap the terminal to walk up and open it.
#[allow(clippy::too_many_arguments)]
fn click_to_walk(
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    touches: Option<Res<Touches>>,
    enabled: Res<InputEnabled>,
    active: Res<ActiveMission>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(&Camera, &GlobalTransform)>,
    players: Query<&Transform, With<Player>>,
    mut intent: ResMut<Intent>,
) {
    if !enabled.0 {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let clicked = buttons
        .filter(|b| b.just_pressed(MouseButton::Left))
        .and_then(|_| window.cursor_position());
    let tapped = touches.and_then(|t| t.iter_just_pressed().next().map(|t| t.position()));
    let Some(at) = clicked.or(tapped) else {
        return;
    };
    let Ok((camera, cam_tf)) = cams.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(cam_tf, at) else {
        return;
    };
    if ray.direction.y.abs() < 1e-4 {
        return;
    }
    let t = -ray.origin.y / ray.direction.y;
    if t <= 0.0 {
        return;
    }
    let ground = ray.origin + *ray.direction * t;
    let g = Vec2::new(ground.x, ground.z);
    let player = players.single().map(|t| t.translation).unwrap_or_default();
    if let Some(term) = active.0.as_deref().and_then(scene::terminal_for)
        && g.distance(Vec2::new(term.x, term.z)) < 1.8
    {
        if in_range(player, term.mission) {
            intent.interact = true;
        } else {
            intent.walk_to = Some(term.approach());
            intent.open_on_arrival = true;
        }
        return;
    }
    if scene::walkable(g) {
        intent.walk_to = Some(Vec3::new(g.x, 0.0, g.y));
        intent.open_on_arrival = false;
    }
}

/// The hand-drawn ink circle, made of short strokes that appear one after another, as if
/// someone were drawing round the light.
#[derive(Component)]
pub struct InkRing {
    since: f32,
    shown: Option<Target>,
}

#[derive(Component)]
pub struct RingStroke(u32);

const RING_STROKES: u32 = 30;
/// The circle overshoots its start, like a pen.
const RING_SWEEP: f32 = TAU + 0.7;

fn ring_radius(a: f32) -> f32 {
    1.0 + 0.06 * (3.0 * a + 0.8).sin() + 0.035 * (5.0 * a).cos() + a / RING_SWEEP * 0.08
}

/// One stroke of the ring: a flat ribbon from angle a0 to a1 (unit radius, facing +z).
fn ring_stroke(a0: f32, a1: f32, width: f32) -> Mesh {
    let steps = 4;
    let mut pos = Vec::new();
    let mut idx = Vec::new();
    for s in 0..=steps {
        let a = a0 + (a1 - a0) * s as f32 / steps as f32;
        let r = ring_radius(a);
        let dir = Vec3::new(a.cos(), a.sin(), 0.0);
        pos.push((dir * (r - width / 2.0)).to_array());
        pos.push((dir * (r + width / 2.0)).to_array());
        if s > 0 {
            let b = (s * 2) as u32;
            idx.extend_from_slice(&[b - 2, b - 1, b, b - 1, b + 1, b]);
        }
    }
    let n = pos.len();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n])
        .with_inserted_indices(Indices::U32(idx))
}

fn spawn_ring(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mat = materials.add(StandardMaterial {
        base_color: color(tokens::ACCENT),
        unlit: true,
        cull_mode: None,
        double_sided: true,
        ..default()
    });
    commands
        .spawn((
            InkRing { since: 0.0, shown: None },
            Transform::from_xyz(0.0, -50.0, 0.0),
            Visibility::Hidden,
        ))
        .with_children(|c| {
            for k in 0..RING_STROKES {
                let a0 = RING_SWEEP * k as f32 / RING_STROKES as f32 - PI / 2.0 - 0.3;
                let a1 = a0 + RING_SWEEP / RING_STROKES as f32 + 0.01;
                c.spawn((
                    RingStroke(k),
                    Mesh3d(meshes.add(ring_stroke(a0, a1, 0.18))),
                    MeshMaterial3d(mat.clone()),
                    Transform::default(),
                    Visibility::Hidden,
                ));
            }
        });
}

/// World point and ring size for a target.
pub fn target_spot(t: Target) -> Option<(Vec3, f32)> {
    match t {
        Target::Light { i } if i < scene::LIGHTS => Some((scene::light_position(i) + Vec3::Y * 2.9 * LIGHT_SCALE, 0.85 * LIGHT_SCALE)),
        Target::Lamp { i } if i < scene::LAMPS => Some((scene::lamp_position(i) + Vec3::Y * 1.45, 0.42)),
        _ => None,
    }
}

fn show_ring(
    time: Res<Time>,
    motion: Res<Motion>,
    highlight: Res<Highlight>,
    cams: Query<&Transform, (With<CameraRig>, Without<InkRing>)>,
    mut rings: Query<(&mut InkRing, &mut Transform, &mut Visibility), Without<RingStroke>>,
    mut strokes: Query<(&RingStroke, &mut Visibility), Without<InkRing>>,
) {
    let Ok((mut ring, mut tf, mut vis)) = rings.single_mut() else {
        return;
    };
    let now = time.elapsed_secs();
    if ring.shown != highlight.0 {
        ring.shown = highlight.0;
        ring.since = now;
    }
    let Some((at, size)) = highlight.0.and_then(target_spot) else {
        *vis = Visibility::Hidden;
        return;
    };
    let Ok(cam) = cams.single() else {
        return;
    };
    *vis = Visibility::Visible;
    let to_cam = (cam.translation - at).normalize();
    // In front of the light, facing the camera, so nothing cuts through the circle.
    tf.translation = at + to_cam * 0.7;
    tf.rotation = cam.rotation;
    tf.scale = Vec3::splat(size);
    let drawn = if motion.reduced {
        RING_STROKES
    } else {
        ((now - ring.since) / 0.5 * RING_STROKES as f32) as u32 + 1
    };
    for (s, mut v) in &mut strokes {
        *v = if s.0 < drawn { Visibility::Inherited } else { Visibility::Hidden };
    }
}

/// Pen width follows the device pixel ratio; the line "boils" every boil step unless
/// motion is reduced.
fn ink_pen(
    time: Res<Time>,
    motion: Res<Motion>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut inks: Query<&mut InkOutline>,
) {
    let scale = windows.single().map(|w| w.scale_factor()).unwrap_or(1.0);
    let seed = if motion.reduced {
        0.0
    } else {
        (time.elapsed_secs() / tokens::BOIL_STEP).floor() % 3.0
    };
    for mut ink in &mut inks {
        let next = InkOutline {
            wobble: Vec4::new(ink.wobble.x, ink.wobble.y, seed, ink.wobble.w),
            ..*ink
        }
        .for_scale(scale);
        if next.params != ink.params || next.wobble != ink.wobble {
            *ink = next;
        }
    }
}

/// Where things are on screen (CSS px, from the canvas's top-left), for the page's
/// number tags and the mismatch caption.
#[derive(Resource, Default, Debug, Clone, serde::Serialize)]
pub struct ScreenLabels {
    /// The viewport these points are in (logical px). The page scales by its CSS size,
    /// which keeps the tags right even where a browser reports DPR oddly.
    pub size: [f32; 2],
    pub lights: Vec<Option<[f32; 2]>>,
    pub lamps: Vec<Option<[f32; 2]>>,
    pub terminal: Option<[f32; 2]>,
    pub ring: Option<[f32; 2]>,
}

fn screen_labels(
    active: Res<ActiveMission>,
    highlight: Res<Highlight>,
    cams: Query<(&Camera, &GlobalTransform)>,
    mut labels: ResMut<ScreenLabels>,
) {
    let Ok((camera, cam)) = cams.single() else {
        return;
    };
    let size = camera.logical_viewport_size().unwrap_or(Vec2::ONE);
    labels.size = [size.x, size.y];
    let project = |p: Vec3| -> Option<[f32; 2]> {
        let v = camera.world_to_viewport(cam, p).ok()?;
        (v.x >= 0.0 && v.y >= 0.0 && v.x <= size.x && v.y <= size.y).then_some([v.x, v.y])
    };
    labels.lights = (0..scene::LIGHTS)
        .map(|i| project(scene::light_position(i) + Vec3::Y * 3.65 * LIGHT_SCALE))
        .collect();
    labels.lamps = (0..scene::LAMPS)
        .map(|i| project(scene::lamp_position(i) + Vec3::Y * 2.0))
        .collect();
    labels.terminal = active
        .0
        .as_deref()
        .and_then(scene::terminal_for)
        .and_then(|t| project(t.position() + Vec3::Y * 1.7));
    labels.ring = highlight
        .0
        .and_then(target_spot)
        .and_then(|(p, s)| project(p + Vec3::Y * (s + 0.1)));
}

pub(crate) fn lamp_materials(palette: &Palette, on: bool) -> Handle<StandardMaterial> {
    if on { palette.lamp_on.clone() } else { palette.lamp_off.clone() }
}

pub(crate) type LampQuery<'w, 's> = Query<'w, 's, (&'static LampLit, &'static ChangedAt, &'static bridge::LampParts), With<BridgeLamp>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pop_settles_at_one() {
        assert_eq!(pop(0.0), 0.0);
        assert!(pop(0.7) > 1.0, "overshoots");
        assert_eq!(pop(1.0), 1.0);
        assert_eq!(pop(3.0), 1.0);
    }

    #[test]
    fn camera_fits_the_wave_on_a_wide_canvas() {
        let d = fit(22.0, 16.0 / 9.0);
        assert!(d > 15.0 && d < 22.0, "{d}");
    }
}
