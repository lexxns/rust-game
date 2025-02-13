use bevy::prelude::*;
use shared::channel::GameMessage;
use crate::game::game_event_processing::process_game_events;
use crate::game::game_event_structs::{GameEvent, GameEventQueue, GameEventWithContext, GameStateComponent};
use crate::player_component::{Player, PlayerJoinEvent, PlayerLeaveEvent};
use crate::room::room_components::{CurrentTurn, Players, Room, RoomState, TurnTimer};
use crate::room::room_manager::RoomManager;
use crate::types::Server;

pub struct RoomPlugin;

impl Plugin for RoomPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<RoomManager>()
            .add_event::<PlayerJoinEvent>()
            .add_event::<PlayerLeaveEvent>()
            .add_event::<GameEventWithContext>()
            .add_systems(Update, (
                // First handle player management
                (
                    handle_player_join,
                    handle_player_leave,
                ),
                // Then route any generated events to room queues
                route_game_events,
                // Then process room state
                (
                    handle_room_turns,
                    update_room_timer,
                    process_game_events,
                ),
                // Finally cleanup
                cleanup_inactive_rooms,
            ).chain());
    }
}

fn handle_player_join(
    mut commands: Commands,
    mut room_manager: ResMut<RoomManager>,
    mut join_events: EventReader<PlayerJoinEvent>,
    mut rooms: Query<(Entity, &mut Players, &mut GameStateComponent)>,
    mut game_events: EventWriter<GameEventWithContext>,
) {
    for PlayerJoinEvent(player_id) in join_events.read() {
        let room_entity = room_manager.find_or_create_room(
            &mut commands,
            *player_id,
            &mut rooms,
            &mut game_events
        );
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
                server.send(player_id, GameMessage::CurrentTurn(Some(first_player)));
            }
        }
    }
}

pub fn route_game_events(
    mut game_events: EventReader<GameEventWithContext>,
    mut rooms: Query<(Entity, &mut GameEventQueue)>,
) {
    for event in game_events.read() {
        // Find the room using the context and add the event to its queue
        if let Ok((_, mut event_queue)) = rooms.get_mut(event.context.room_entity) {
            event_queue.next_events.push_back(event.clone());
        } else {
            warn!("Attempted to route event to non-existent room: {:?}", event.context.room_entity);
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
                    server.send(player_id, GameMessage::CurrentTurn(Some(next_player)));
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

#[derive(Component)]
struct RoomCleanup;