use bevy::prelude::*;
use bevy_simplenet::{ClientId, RequestToken, ServerReport};
use shared::channel::GameMessage;
use crate::game::game_events::{GameEvent, IntoGameEvent, MessageContext};
use crate::player_component::{Player, PlayerJoinEvent, PlayerLeaveEvent};
use crate::room::room_components::{Players};
use crate::types::{Server, ServerEvent};

#[allow(clippy::type_complexity)]
pub fn handle_server_events(
    mut commands: Commands,
    mut server: ResMut<Server>,
    mut join_events: EventWriter<PlayerJoinEvent>,
    mut leave_events: EventWriter<PlayerLeaveEvent>,
    mut game_events: EventWriter<GameEvent>,
    player_query: Query<(Entity, &Player)>,
    rooms: Query<(Entity, &Players)>,
) {
    while let Some((client_id, event)) = server.next() {
        match event {
            ServerEvent::Report(report) => handle_report(
                &mut commands,
                &mut join_events,
                &mut leave_events,
                &player_query,
                client_id,
                report,
            ),
            ServerEvent::Request(token, request) => handle_request(
                &mut game_events,
                &mut commands,
                &mut server,
                &player_query,
                &rooms,
                client_id,
                token,
                request,
            ),
            ServerEvent::Msg(..) => {}
        }
    }
}

fn handle_request(
    game_events: &mut EventWriter<GameEvent>,
    commands: &mut Commands,
    server: &mut ResMut<Server>,
    player_query: &Query<(Entity, &Player)>,
    rooms: &Query<(Entity, &Players)>,
    client_id: ClientId,
    token: RequestToken,
    message: GameMessage,
) {
    // Try to convert the message to a game event
    if let Some((_, player)) = player_query.iter().find(|(_, p)| p.id == client_id) {
        let context = MessageContext {
            client_id,
            room_entity: Some(player.room),
        };

        match message.clone().into_game_event(&context) {
            Some(event) => {
                println!("Processing game event: {:?}", event);
                game_events.send(event);
                server.ack(token);
            }
            None => {
                handle_non_event_message(message, player.room, rooms, server);
                server.ack(token);
            }
        }
    } else {
        // Player not found - might be in the process of joining
        server.send(
            client_id,
            GameMessage::Error("Cannot process request - player not initialized".to_string()),
        );
        server.reject(token);
    }
}

fn handle_report(
    commands: &mut Commands,
    join_events: &mut EventWriter<PlayerJoinEvent>,
    leave_events: &mut EventWriter<PlayerLeaveEvent>,
    player_query: &Query<(Entity, &Player)>,
    client_id: ClientId,
    report: ServerReport<()>,
) {
    match report {
        ServerReport::Connected(_, _) => {
            join_events.send(PlayerJoinEvent(client_id));
        }
        ServerReport::Disconnected => {
            if let Some((player_entity, player)) = player_query.iter().find(|(_, p)| p.id == client_id)
            {
                leave_events.send(PlayerLeaveEvent {
                    player_id: client_id,
                    room_entity: player.room,
                });
                commands.entity(player_entity).despawn();
            }
        }
    }
}

fn handle_non_event_message(
    message: GameMessage,
    room_entity: Entity,
    rooms: &Query<(Entity, &Players)>,
    server: &Server,
) {
    match message {
        GameMessage::Chat(msg) => {
            // We can directly query the room using its Entity
            if let Ok((_, players)) = rooms.get(room_entity) {
                for &player_id in &players.set {
                    server.send(player_id, GameMessage::Chat(msg.clone()));
                }
            }
        }
        _ => {
            println!(
                "Warning: Unexpected action received in non-event handler: {:?}",
                message
            );
        }
    }
}