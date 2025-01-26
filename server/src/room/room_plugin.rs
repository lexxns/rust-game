use bevy::prelude::*;
use shared::channel::ServerMsg;
use crate::player_component::{Player, PlayerJoinEvent, PlayerLeaveEvent};
use crate::room::room_components::{CurrentTurn, NextTurn, Players, Room, RoomState, TurnTimer};
use crate::room::room_manager::RoomManager;
use crate::types::Server;

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<RoomManager>()
            .add_event::<PlayerJoinEvent>()
            .add_event::<PlayerLeaveEvent>()
            .add_systems(Update, (
                handle_player_join,
                handle_player_leave,
                handle_room_turns,
                update_room_timer,
                cleanup_inactive_rooms,
                handle_next_turn,
            ).chain());
    }
}

fn handle_player_join(
    mut commands: Commands,
    mut room_manager: ResMut<RoomManager>,
    mut join_events: EventReader<PlayerJoinEvent>,
    mut rooms: Query<(Entity, &mut Players)>,
) {
    for PlayerJoinEvent(player_id) in join_events.read() {
        let room_entity = room_manager.find_or_create_room(&mut commands, *player_id, &mut rooms);
        commands.spawn(Player {
            id: *player_id,
            room: room_entity,
        });
    }
}

fn handle_room_turns(
    mut query: Query<(Entity, &Room, &Players, &mut CurrentTurn, &mut TurnTimer)>,
    server: Res<Server>,
) {
    for (_entity, _room, players, mut current_turn, _timer) in query.iter_mut() {
        if players.set.len() != 2 {
            continue;
        }

        if current_turn.player.is_none() {
            let players_vec: Vec<_> = players.set.iter().collect();
            let first_player = *players_vec[rand::random::<usize>() % 2];
            current_turn.player = Some(first_player);

            // Notify players
            for &player_id in &players.set {
                server.send(player_id, ServerMsg::Current(Some(first_player)));
            }
        }
    }
}

fn update_room_timer(
    time: Res<Time>,
    mut query: Query<(Entity, &Room, &Players, &mut CurrentTurn, &mut TurnTimer)>,
    server: Res<Server>,
) {
    for (_entity, _room, players, mut current_turn, mut timer) in query.iter_mut() {
        timer.timer.tick(time.delta());

        if timer.timer.finished() {
            if let Some(current_player) = current_turn.player {
                let next_player = *players.set.iter()
                    .find(|&&p| p != current_player)
                    .unwrap();

                current_turn.player = Some(next_player);
                timer.timer.reset();

                // Notify players
                for &player_id in &players.set {
                    server.send(player_id, ServerMsg::Current(Some(next_player)));
                }
            }
        }
    }
}

fn handle_player_leave(
    mut commands: Commands,
    mut leave_events: EventReader<PlayerLeaveEvent>,
    mut rooms: Query<(Entity, &mut Players, &mut CurrentTurn)>,
) {
    for event in leave_events.read() {
        if let Ok((entity, mut players, mut current_turn)) = rooms.get_mut(event.room_entity) {
            players.set.remove(&event.player_id);
            current_turn.player = None;

            if players.set.is_empty() {
                commands.entity(entity).insert(RoomCleanup);
            }
        }
    }
}

fn cleanup_inactive_rooms(
    mut commands: Commands,
    rooms: Query<(Entity, &RoomState), With<RoomCleanup>>,
) {
    for (entity, _) in rooms.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn handle_next_turn(
    mut commands: Commands,
    mut query: Query<(Entity, &Players, &mut CurrentTurn), With<NextTurn>>,
    server: Res<Server>,
) {
    for (entity, players, mut current_turn) in query.iter_mut() {
        if let Some(current_player) = current_turn.player {
            let next_player = players.set.iter()
                .find(|&&p| p != current_player)
                .copied();

            if let Some(next_player) = next_player {
                current_turn.player = Some(next_player);
                for &player_id in &players.set {
                    server.send(player_id, ServerMsg::Current(Some(next_player)));
                }
            }
        }
        commands.entity(entity).remove::<NextTurn>();
    }
}

#[derive(Component)]
struct RoomCleanup;