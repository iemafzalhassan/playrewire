//! The Binary Bridge: a deck over the canal, two railings, and 8 lamps on the far
//! railing that read left to right as an 8-bit number (lamp 0 = the biggest digit).

use bevy::prelude::*;

use crate::{
    city_events::{BridgeLamp, Motion},
    render::{LampQuery, Palette, bx, decal, lamp_materials, pop},
    scene::{CANAL, ROAD_HALF},
};

pub(crate) fn spawn_bridge(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, palette: Res<Palette>) {
    let p = &*palette;
    let c = &mut commands;
    let m = &mut *meshes;
    let (a, b) = CANAL;
    let x = (a + b) / 2.0;
    let len = b - a + 0.8;
    // Deck (its top is flush with the road) and its edge beam.
    decal(c, m, &p.road, Vec2::new(len, ROAD_HALF * 2.0), Vec3::new(x, 0.004, 0.0));
    bx(c, m, &p.kerb, Vec3::new(len, 0.4, ROAD_HALF * 2.0 + 0.5), Vec3::new(x, -0.2, 0.0));
    // Two piers in the water.
    for px in [a + 1.6, b - 1.6] {
        bx(c, m, &p.card, Vec3::new(0.7, 0.8, ROAD_HALF * 2.0), Vec3::new(px, -0.6, 0.0));
    }
    // Railings: a top rail and posts. The near one is low so it never hides the deck.
    for (z, h) in [(-ROAD_HALF - 0.15, 1.0), (ROAD_HALF + 0.15, 0.6)] {
        bx(c, m, &p.accent_deep, Vec3::new(len, 0.1, 0.12), Vec3::new(x, h, z));
        // The far railing's posts are the lamp posts; only its ends need their own.
        let posts = if z < 0.0 { 1 } else { 10 };
        for k in 0..=posts {
            let px = a - 0.4 + len * k as f32 / posts as f32;
            bx(c, m, &p.ink_soft, Vec3::new(0.08, h, 0.08), Vec3::new(px, h / 2.0, z));
        }
    }
}

/// A lamp's changing parts.
#[derive(Component)]
pub struct LampParts {
    bulb: Entity,
    rays: Entity,
}

pub(crate) fn dress_lamps(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    palette: Res<Palette>,
    added: Query<Entity, Added<BridgeLamp>>,
) {
    for e in &added {
        let p = &*palette;
        let mut bulb = Entity::PLACEHOLDER;
        let mut rays = Entity::PLACEHOLDER;
        commands.entity(e).insert(Visibility::default()).with_children(|c| {
            c.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.05, 1.3))),
                MeshMaterial3d(p.ink_soft.clone()),
                Transform::from_xyz(0.0, 0.65, 0.0),
            ));
            c.spawn((
                Mesh3d(meshes.add(Cone::new(0.2, 0.18))),
                MeshMaterial3d(p.ink_soft.clone()),
                Transform::from_xyz(0.0, 1.72, 0.0),
            ));
            bulb = c
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(0.21).mesh().uv(16, 10))),
                    MeshMaterial3d(p.lamp_off.clone()),
                    Transform::from_xyz(0.0, 1.47, 0.0),
                ))
                .id();
            rays = c
                .spawn((Transform::from_xyz(0.0, 1.47, 0.05), Visibility::Hidden))
                .with_children(|r| {
                    let ray = meshes.add(Cuboid::new(0.05, 0.16, 0.04));
                    for k in 0..7 {
                        // A fan of rays, but none pointing down into the post.
                        let ang = -1.9 + k as f32 * (3.8 / 6.0);
                        r.spawn((
                            Mesh3d(ray.clone()),
                            MeshMaterial3d(p.highlight.clone()),
                            Transform::from_xyz(ang.sin() * 0.36, ang.cos() * 0.36, 0.0)
                                .with_rotation(Quat::from_rotation_z(-ang)),
                        ));
                    }
                })
                .id();
        });
        commands.entity(e).insert(LampParts { bulb, rays });
    }
}

pub(crate) fn show_lamps(
    time: Res<Time>,
    motion: Res<Motion>,
    palette: Res<Palette>,
    cams: Query<&Transform, (With<Camera3d>, Without<BridgeLamp>)>,
    lamps: LampQuery,
    mut mats: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut parts: Query<(&mut Transform, &mut Visibility), (Without<BridgeLamp>, Without<Camera3d>)>,
) {
    let now = time.elapsed_secs();
    let face = cams.single().map(|c| c.rotation).unwrap_or_default();
    for (lit, at, lp) in &lamps {
        if let Ok(mut m) = mats.get_mut(lp.bulb) {
            m.0 = lamp_materials(&palette, lit.0);
        }
        let s = if motion.reduced { 1.0 } else { pop((now - at.0) / 0.3) };
        if let Ok((mut t, _)) = parts.get_mut(lp.bulb) {
            t.scale = Vec3::splat(if lit.0 { 0.85 + 0.15 * s } else { 1.0 });
        }
        if let Ok((mut t, mut v)) = parts.get_mut(lp.rays) {
            *v = if lit.0 { Visibility::Inherited } else { Visibility::Hidden };
            t.scale = Vec3::splat(s.max(0.01));
            // Rays face the camera (billboard), so they read from any angle.
            t.rotation = face;
        }
    }
}
