//! Mission terminals: one per mission. Only the loaded mission's terminal is live; the
//! player must stand within 2 m of it to open it.

use bevy::prelude::*;

use crate::{
    mailbox::{Mailbox, WorldSignal},
    player::{Intent, Player},
    scene::{self, TERMINAL_RANGE},
};

#[derive(Component, Debug, Clone, Copy)]
pub struct Terminal {
    pub id: &'static str,
    pub mission: &'static str,
}

/// The mission whose terminal is live.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct ActiveMission(pub Option<String>);

/// The last `Near` the page was told, so it hears only changes.
#[derive(Resource, Debug, Default)]
pub struct NearTerminal(pub Option<String>);

pub(crate) fn spawn_terminals(mut commands: Commands) {
    for t in scene::TERMINALS {
        commands.spawn((
            Name::new(t.id),
            Terminal { id: t.id, mission: t.mission },
            Transform::from_translation(t.position()),
        ));
    }
}

/// Is the player close enough to use `mission`'s terminal?
pub fn in_range(player: Vec3, mission: &str) -> bool {
    scene::terminal_for(mission).is_some_and(|t| {
        Vec2::new(player.x, player.z).distance(Vec2::new(t.x, t.z)) <= TERMINAL_RANGE
    })
}

pub(crate) fn use_terminals(
    active: Res<ActiveMission>,
    mailbox: Res<Mailbox>,
    mut near: ResMut<NearTerminal>,
    mut intent: ResMut<Intent>,
    players: Query<&Transform, With<Player>>,
) {
    let Ok(player) = players.single() else {
        return;
    };
    let here = active
        .0
        .as_deref()
        .filter(|m| in_range(player.translation, m))
        .map(str::to_owned);
    if here != near.0 {
        near.0 = here.clone();
        mailbox.emit(WorldSignal::Near { mission: here.clone() });
    }
    if std::mem::take(&mut intent.interact)
        && let Some(mission) = here
    {
        mailbox.emit(WorldSignal::Terminal { mission });
    }
}
