use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use bevy_simplenet::ServerEvent::Report;
use shared::channel::{ClientRequest, ServerMsg};
use crate::types::*;
use crate::game::GameState;

pub fn handle_server_events(
    mut commands: Commands,
    mut server: ResMut<DemoServer>,
    mut game_state: ResMut<GameState>,
    mut button_states: ReactResMut<ButtonStates>,
) {
    let mut state_updates = Vec::new();
    let mut rooms_to_remove = Vec::new();

    while let Some((client_id, server_event)) = server.next() {
        match server_event {
            Report(connection_report) => match connection_report {
                bevy_simplenet::ServerReport::Connected(_, _) => {
                    tracing::info!("client {:?} connected", client_id);
                    let room_id = game_state.find_or_create_room();

                    // Update room with new player
                    if let Some(room) = game_state.rooms.get_mut(&room_id) {
                        room.players.insert(client_id);

                        // If room is now full, select first player
                        if room.is_ready_to_start() {
                            if let Some(first_player) = room.select_initial_player() {
                                tracing::info!("Game starting in room {}, first player: {}", room_id, first_player);
                                state_updates.push((room_id.clone(), Some(first_player)));
                            }
                        }
                    }

                    game_state.player_to_room.insert(client_id, room_id.clone());                }
                bevy_simplenet::ServerReport::Disconnected => {
                    tracing::info!("client {:?} disconnected", client_id);

                    if let Some(room_id) = game_state.player_to_room.remove(&client_id) {
                        if let Some(room) = game_state.rooms.get_mut(&room_id) {
                            room.players.remove(&client_id);

                            // Reset room state when a player leaves
                            room.current_turn = None;
                            state_updates.push((room_id.clone(), None));

                            if room.players.is_empty() {
                                rooms_to_remove.push(room_id.clone());
                            }
                        }
                    }
                }
            },
            DemoServerEvent::Msg(()) => continue,
            DemoServerEvent::Request(token, request) => {
                tracing::info!("Received request: {:?}", request);
                match request {
                    ClientRequest::EndTurn => {
                        tracing::info!("received end turn from client {:?}", client_id);

                        if let Some(room_id) = game_state.player_to_room.get(&client_id).cloned() {
                            if let Some(room) = game_state.rooms.get_mut(&room_id) {
                                // Verify it's actually this player's turn
                                if room.current_turn == Some(client_id) {
                                    if let Some(next_player) = room.switch_turn() {
                                        server.ack(token);
                                        state_updates.push((room_id, Some(next_player)));
                                        tracing::info!("Turn switched to player {}", next_player);
                                    }
                                } else {
                                    server.reject(token);
                                    tracing::warn!("Player {} tried to end turn when it wasn't their turn", client_id);
                                }
                            }
                        }
                    }
                    ClientRequest::Chat(msg_type) => {
                        tracing::info!("received {:?} from client {:?}", msg_type, client_id);
                        server.ack(token);
                    }
                }
            }
        }
    }

    // Apply all state updates at once
    for (room_id, state) in state_updates {
        button_states.get_mut(&mut commands).0.insert(room_id, state);
    }

    // Remove empty rooms at the end
    for room_id in rooms_to_remove {
        game_state.rooms.remove(&room_id);
        button_states.get_mut(&mut commands).0.remove(&room_id);
    }

}

pub fn setup_room_state_reaction(rc: &mut ReactCommands) {
    rc.on_persistent(
        resource_mutation::<ButtonStates>(),
        |server: Res<DemoServer>,
         game_state: Res<GameState>,
         button_states: ReactRes<ButtonStates>| {
            for (room_id, _state) in &button_states.0 {
                send_room_state(&server, &game_state, room_id);
            }
        },
    );
}

fn send_room_state(
    server: &DemoServer,
    game_state: &GameState,
    room_id: &str,
) {
    if let Some(room) = game_state.rooms.get(room_id) {
        for &player_id in &room.players {
            server.send(player_id, ServerMsg::Current(room.get_current_turn()));
        }
    }
}