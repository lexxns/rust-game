use bevy::prelude::*;
use bevy_cobweb::prelude::{CommandsSyscallExt, ReactCommandsExt, ReactRes, ReactResMut};
use shared::api::API_VERSION;
use shared::channel::{GameChannel, GameMessage};
use crate::state::{ConnectionStatus, TurnPlayer, EndTurn, GameState};
use crate::ui::DeselectButton;

pub type Client = bevy_simplenet::Client<GameChannel>;
pub type ClientEvent = bevy_simplenet::ClientEventFrom<GameChannel>;

pub fn client_factory() -> bevy_simplenet::ClientFactory<GameChannel> {
    bevy_simplenet::ClientFactory::<GameChannel>::new(API_VERSION)
}

fn set_new_server_state(
    In(server_state): In<Option<u128>>,
    mut c: Commands,
    client: Res<Client>,
    pending_select: ReactRes<EndTurn>,
    mut owner: ReactResMut<TurnPlayer>
) {
    owner.get_mut(&mut c).server_determined_player_id = server_state;

    if pending_select.is_predicted() { return; }

    if server_state != Some(client.id()) {
        c.react().broadcast(DeselectButton);
    }
}

pub fn handle_client_events(
    mut c: Commands,
    mut client: ResMut<Client>,
    mut status: ReactResMut<ConnectionStatus>,
    mut pending_select: ReactResMut<EndTurn>,
    mut turn_player: ReactResMut<TurnPlayer>,
    mut game_state: ReactResMut<GameState>,
) {
    let mut next_status = *status;

    while let Some(client_event) = client.next() {
        match client_event {
            ClientEvent::Report(connection_report) => match connection_report {
                bevy_simplenet::ClientReport::Connected => next_status = ConnectionStatus::Connected,
                bevy_simplenet::ClientReport::Disconnected |
                bevy_simplenet::ClientReport::ClosedByServer(_) |
                bevy_simplenet::ClientReport::ClosedBySelf => next_status = ConnectionStatus::Connecting,
                bevy_simplenet::ClientReport::IsDead(aborted_reqs) => {
                    for aborted_req in aborted_reqs {
                        if !pending_select.equals_request(aborted_req) { continue; }
                        c.react().broadcast(DeselectButton);
                    }
                    next_status = ConnectionStatus::Dead;
                }
            }
            ClientEvent::Msg(message) => match message {
                GameMessage::CurrentTurn(new_id) => {
                    c.syscall(new_id, set_new_server_state);
                }
                GameMessage::CardsDrawn(mut cards) => {
                    let state = game_state.get_mut(&mut c);
                    state.hand.append(&mut cards);
                    let hand_size = state.hand.len();
                    println!("{hand_size} cards in hand");
                }
                _ => {}
            }
            ClientEvent::Ack(request_id) => {
                if !pending_select.equals_request(request_id) { continue; }
                let tp = turn_player.get_mut(&mut c);
                tp.server_determined_player_id = tp.predicted_player_id;
                tp.predicted_player_id = None;
                pending_select.get_mut(&mut c).0 = None;
            }
            ClientEvent::Reject(request_id) |
            ClientEvent::Response((), request_id) |
            ClientEvent::SendFailed(request_id) |
            ClientEvent::ResponseLost(request_id) => {
                if !pending_select.equals_request(request_id) { continue; }
                c.react().broadcast(DeselectButton);
            }
        }
    }

    if next_status != *status {
        *status.get_mut(&mut c) = next_status;
    }
}