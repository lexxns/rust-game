use bevy::prelude::*;
use bevy_cobweb::prelude::{CommandsSyscallExt, ReactCommandsExt, ReactRes, ReactResMut};
use shared::api::API_VERSION;
use shared::channel::{ChatChannel, ServerMsg};
use crate::state::{ConnectionStatus, ButtonOwner, PendingSelect};
use crate::ui::DeselectButton;

pub type DemoClient = bevy_simplenet::Client<ChatChannel>;
pub type DemoClientEvent = bevy_simplenet::ClientEventFrom<ChatChannel>;

pub fn client_factory() -> bevy_simplenet::ClientFactory<ChatChannel> {
    bevy_simplenet::ClientFactory::<ChatChannel>::new(API_VERSION)
}

fn set_new_server_state(
    In(server_state): In<Option<u128>>,
    mut c: Commands,
    client: Res<DemoClient>,
    pending_select: ReactRes<PendingSelect>,
    mut owner: ReactResMut<ButtonOwner>
) {
    owner.get_mut(&mut c).server_authoritative_id = server_state;

    if pending_select.is_predicted() { return; }

    if server_state != Some(client.id()) {
        c.react().broadcast(DeselectButton);
    }
}

pub fn handle_client_events(
    mut c: Commands,
    mut client: ResMut<DemoClient>,
    mut status: ReactResMut<ConnectionStatus>,
    mut pending_select: ReactResMut<PendingSelect>,
    mut owner: ReactResMut<ButtonOwner>
) {
    let mut next_status = *status;

    while let Some(client_event) = client.next() {
        match client_event {
            DemoClientEvent::Report(connection_report) => match connection_report {
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
            DemoClientEvent::Msg(message) => match message {
                ServerMsg::Current(new_id) => {
                    c.syscall(new_id, set_new_server_state);
                }
                _ => {}
            }
            DemoClientEvent::Ack(request_id) => {
                if !pending_select.equals_request(request_id) { continue; }
                let owner = owner.get_mut(&mut c);
                owner.server_authoritative_id = owner.predicted_id;
                owner.predicted_id = None;
                pending_select.get_mut(&mut c).0 = None;
            }
            DemoClientEvent::Reject(request_id) |
            DemoClientEvent::Response((), request_id) |
            DemoClientEvent::SendFailed(request_id) |
            DemoClientEvent::ResponseLost(request_id) => {
                if !pending_select.equals_request(request_id) { continue; }
                c.react().broadcast(DeselectButton);
            }
        }
    }

    if next_status != *status {
        *status.get_mut(&mut c) = next_status;
    }
}