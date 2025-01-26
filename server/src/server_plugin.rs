use bevy::prelude::{Commands, Entity, EventWriter, Query, ResMut};
use bevy_simplenet::{ClientId, RequestToken, ServerReport};
use shared::channel::ClientRequest;
use crate::player_component::{Player, PlayerJoinEvent, PlayerLeaveEvent};
use crate::room::room_components::NextTurn;
use crate::types::{Server, ServerEvent};

pub fn handle_server_events(
    mut commands: Commands,
    mut server: ResMut<Server>,
    mut join_events: EventWriter<PlayerJoinEvent>,
    mut leave_events: EventWriter<PlayerLeaveEvent>,
    player_query: Query<(Entity, &Player)>,
) {
    while let Some((client_id, event)) = server.next() {
        match event {
            ServerEvent::Report(report) => handle_report(
                &mut commands, &mut join_events, &mut leave_events, &player_query, client_id, report
            ),
            ServerEvent::Request(token, request) => handle_request(
                &mut commands, &mut server, &player_query, client_id, token, request
            ),
            ServerEvent::Msg(..) => {}
        }
    }
}

fn handle_request(
    mut commands: &mut Commands,
    server: &mut ResMut<Server>,
    player_query: &Query<(Entity, &Player)>,
    client_id: ClientId,
    token: RequestToken,
    request: ClientRequest
) {
    match request {
        ClientRequest::EndTurn => {
            handle_end_turn_request(&mut commands, &server, token, client_id, &player_query);
        }
        _ => server.ack(token),
    }
}

fn handle_report(
    commands: &mut Commands,
    join_events: &mut EventWriter<PlayerJoinEvent>,
    leave_events: &mut EventWriter<PlayerLeaveEvent>,
    player_query: &Query<(Entity, &Player)>,
    client_id: ClientId,
    report: ServerReport<()>
) {
    match report {
        ServerReport::Connected(_, _) => {
            join_events.send(PlayerJoinEvent(client_id));
        }
        ServerReport::Disconnected => {
            if let Some((player_entity, player)) = player_query
                .iter()
                .find(|(_, p)| p.id == client_id)
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

fn handle_end_turn_request(
    commands: &mut Commands,
    server: &Server,
    token: RequestToken,
    client_id: u128,
    player_query: &Query<(Entity, &Player)>,
) {
    if let Some((_, player)) = player_query.iter().find(|(_, p)| p.id == client_id) {
        commands.entity(player.room).insert(NextTurn);
        server.ack(token);
    } else {
        server.reject(token);
    }
}