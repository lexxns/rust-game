use shared::channel::*;
use bevy::app::*;
use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use std::collections::{HashMap, HashSet};
use bevy::log::tracing_subscriber;
use bevy::utils::tracing;
use serde::{Deserialize, Serialize};
use shared::api::API_VERSION;

type DemoServer = bevy_simplenet::Server<ChatChannel>;
type DemoServerEvent = bevy_simplenet::ServerEventFrom<ChatChannel>;

fn server_factory() -> bevy_simplenet::ServerFactory<ChatChannel> {
    bevy_simplenet::ServerFactory::<ChatChannel>::new(API_VERSION)
}

#[derive(Default)]
struct Room {
    players: HashSet<u128>,
    button_state: Option<u128>,
}

#[derive(Resource, Default)]
struct GameState {
    rooms: HashMap<String, Room>,
    player_to_room: HashMap<u128, String>,
}

#[derive(ReactResource, Default)]
struct ButtonStates(HashMap<String, Option<u128>>);

fn send_room_state(
    server: &DemoServer,
    game_state: &GameState,
    room_id: &str,
    button_state: Option<u128>,
) {
    if let Some(room) = game_state.rooms.get(room_id) {
        for &player_id in &room.players {
            server.send(player_id, ServerMsg::Current(button_state));
        }
    }
}

fn handle_server_events(
    mut commands: Commands,
    mut server: ResMut<DemoServer>,
    mut game_state: ResMut<GameState>,
    mut button_states: ReactResMut<ButtonStates>,
) {
    let mut state_updates = Vec::new();
    let mut rooms_to_remove = Vec::new();

    while let Some((client_id, server_event)) = server.next() {
        match server_event {
            DemoServerEvent::Report(connection_report) => match connection_report {
                bevy_simplenet::ServerReport::Connected(_, _) => {
                    tracing::info!("client {:?} connected", client_id);
                    let room_id = find_or_create_room(&mut game_state);

                    // Get current room state before modifying
                    let current_state = game_state.rooms.get(&room_id)
                        .map(|r| r.button_state)
                        .unwrap_or(None);

                    // Update room
                    if let Some(room) = game_state.rooms.get_mut(&room_id) {
                        room.players.insert(client_id);
                    }
                    game_state.player_to_room.insert(client_id, room_id.clone());

                    // Send state to new client
                    server.send(client_id, ServerMsg::Current(current_state));
                }
                bevy_simplenet::ServerReport::Disconnected => {
                    tracing::info!("client {:?} disconnected", client_id);

                    if let Some(room_id) = game_state.player_to_room.remove(&client_id) {
                        if let Some(room) = game_state.rooms.get_mut(&room_id) {
                            room.players.remove(&client_id);

                            if room.button_state == Some(client_id) {
                                room.button_state = None;
                                state_updates.push((room_id.clone(), None));
                            }

                            if room.players.is_empty() {
                                rooms_to_remove.push(room_id.clone());
                            }
                        }
                    }
                }
            },
            DemoServerEvent::Msg(()) => continue,
            DemoServerEvent::Request(token, request) => match request {
                ClientRequest::Select => {
                    tracing::info!("received {:?} from client {:?}", request, client_id);
                    server.ack(token);

                    if let Some(room_id) = game_state.player_to_room.get(&client_id).cloned() {
                        if let Some(room) = game_state.rooms.get_mut(&room_id) {
                            room.button_state = Some(client_id);
                            state_updates.push((room_id, Some(client_id)));
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

fn find_or_create_room(game_state: &mut GameState) -> String {
    // Try to find a room with space
    for (room_id, room) in &game_state.rooms {
        if room.players.len() < 2 {
            return room_id.clone();
        }
    }

    // Create new room if none found
    let room_id = format!("room_{}", game_state.rooms.len());
    game_state.rooms.insert(room_id.clone(), Room::default());
    room_id
}

fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let server = server_factory().new_server(
        enfync::builtin::native::TokioHandle::default(),
        "127.0.0.1:48888",
        bevy_simplenet::AcceptorConfig::Default,
        bevy_simplenet::Authenticator::None,
        bevy_simplenet::ServerConfig{
            heartbeat_interval: std::time::Duration::from_secs(6),
            ..Default::default()
        },
    );

    let mut app = App::new();
    app
        .add_plugins(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(100)))
        .add_plugins(ReactPlugin)
        .insert_resource(server)
        .init_resource::<GameState>()
        .insert_react_resource(ButtonStates::default());

    app.react(|rc| {
        rc.on_persistent(resource_mutation::<ButtonStates>(),
                         |server: Res<DemoServer>,
                          game_state: Res<GameState>,
                          button_states: ReactRes<ButtonStates>| {
                             for (room_id, state) in &button_states.0 {
                                 send_room_state(&server, &game_state, room_id, *state);
                             }
                         });
    });

    app.add_systems(Main, handle_server_events)
        .run();
}