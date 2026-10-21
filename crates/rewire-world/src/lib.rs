//! rewire-world: the hand-inked PlayRewire street.
//!
//! Spike scene (Task 2): ground, 6 buildings, 5 traffic lights, a capsule player and an
//! orbiting camera, drawn with the ink look from [`ink`]. The canvas is transparent; the
//! page puts the paper colour behind it.

use bevy::{
    core_pipeline::prepass::NormalPrepass,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    render::{
        render_resource::TextureUsages,
        view::{DebandDither, Msaa, Tonemapping},
    },
    window::WindowResolution,
};

pub mod ink;

pub use ink::{InkOutline, InkPlugin};

/// The `<canvas>` id the page provides.
pub const CANVAS_SELECTOR: &str = "#world";
/// Canvas size in physical pixels (scale factor forced to 1 so every browser renders the
/// same number of pixels).
pub const WIDTH: u32 = 1280;
pub const HEIGHT: u32 = 720;
/// How often the average FPS is logged.
pub const FPS_LOG_SECONDS: f32 = 5.0;

/// Builds the app without running it.
pub fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "PlayRewire world".into(),
                canvas: Some(CANVAS_SELECTOR.into()),
                resolution: WindowResolution::new(WIDTH, HEIGHT).with_scale_factor_override(1.0),
                transparent: true,
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin::default(),
        InkPlugin,
    ))
    .insert_resource(ClearColor(Color::NONE))
    .add_systems(Startup, spawn_scene)
    .add_systems(Update, (orbit_camera, log_fps));
    app
}

/// Entry point: runs automatically when the wasm module is instantiated.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    build_app().run();
}

#[derive(Component)]
struct Orbit {
    target: Vec3,
    radius: f32,
    height: f32,
    speed: f32,
}

fn unlit(materials: &mut Assets<StandardMaterial>, hex: &str) -> Handle<StandardMaterial> {
    let color = Srgba::hex(hex).expect("valid hex colour");
    materials.add(StandardMaterial {
        base_color: color.into(),
        unlit: true,
        ..default()
    })
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground: a slightly darker paper so the street reads against the page.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(36.0, 22.0))),
        MeshMaterial3d(unlit(&mut materials, "#ebe2cf")),
    ));
    // Road strip down the middle.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(36.0, 0.02, 4.0))),
        MeshMaterial3d(unlit(&mut materials, "#d9cfba")),
        Transform::from_xyz(0.0, 0.01, 0.0),
    ));

    // A street of 6 buildings, 3 each side.
    let building_colors = [
        "#fffdf7", "#f2d9b8", "#dfe6da", "#f5e3c8", "#e8dccb", "#fffdf7",
    ];
    let heights = [4.0, 6.5, 3.2, 5.0, 7.5, 3.8];
    for (i, (hex, h)) in building_colors.iter().zip(heights).enumerate() {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let x = -8.0 + (i / 2) as f32 * 8.0;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(5.0, h, 4.0))),
            MeshMaterial3d(unlit(&mut materials, hex)),
            Transform::from_xyz(x, h / 2.0, side * 6.0),
        ));
    }

    // 5 traffic lights along the road edge: a post and a lamp box each.
    let post = meshes.add(Cuboid::new(0.2, 2.4, 0.2));
    let lamp = meshes.add(Cuboid::new(0.5, 0.9, 0.5));
    let post_mat = unlit(&mut materials, "#8a8172");
    let lamp_mat = unlit(&mut materials, "#e2572b");
    for i in 0..5 {
        let x = -12.0 + i as f32 * 6.0;
        let z = if i % 2 == 0 { 2.6 } else { -2.6 };
        commands.spawn((
            Mesh3d(post.clone()),
            MeshMaterial3d(post_mat.clone()),
            Transform::from_xyz(x, 1.2, z),
        ));
        commands.spawn((
            Mesh3d(lamp.clone()),
            MeshMaterial3d(lamp_mat.clone()),
            Transform::from_xyz(x, 2.85, z),
        ));
    }

    // The player.
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
        MeshMaterial3d(unlit(&mut materials, "#7fa7c9")),
        Transform::from_xyz(0.0, 0.9, 0.8),
    ));

    commands.spawn((
        Camera3d {
            // The ink pass reads the main depth texture directly (WebGL2 has no DepthPrepass).
            depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING)
                .into(),
            ..default()
        },
        Msaa::Off,
        Tonemapping::None,
        DebandDither::Disabled,
        NormalPrepass,
        InkOutline::default(),
        Orbit {
            target: Vec3::new(0.0, 1.5, 0.0),
            radius: 26.0,
            height: 15.0,
            speed: 0.15,
        },
        Transform::default(),
    ));
}

fn orbit_camera(time: Res<Time>, mut cameras: Query<(&Orbit, &mut Transform)>) {
    for (orbit, mut transform) in &mut cameras {
        let angle = time.elapsed_secs() * orbit.speed;
        let eye = orbit.target
            + Vec3::new(
                angle.cos() * orbit.radius,
                orbit.height,
                angle.sin() * orbit.radius,
            );
        *transform = Transform::from_translation(eye).looking_at(orbit.target, Vec3::Y);
    }
}

#[derive(Default)]
struct FpsWindow {
    frames: u32,
    seconds: f32,
}

/// Logs the average FPS over each 5 s window (real time, not clamped virtual time),
/// plus the smoothed value from `FrameTimeDiagnosticsPlugin`.
fn log_fps(
    time: Res<Time<Real>>,
    diagnostics: Res<DiagnosticsStore>,
    mut window: Local<FpsWindow>,
) {
    window.frames += 1;
    window.seconds += time.delta_secs();
    if window.seconds >= FPS_LOG_SECONDS {
        let avg = window.frames as f32 / window.seconds;
        let smoothed = diagnostics
            .get(&FrameTimeDiagnosticsPlugin::FPS)
            .and_then(|d| d.average())
            .unwrap_or(0.0);
        info!("rewire-world fps avg5s={avg:.1} diag={smoothed:.1}");
        *window = FpsWindow::default();
    }
}
