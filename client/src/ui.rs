use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::prelude::*;
use std::fmt::Write;
use shared::channel::ClientRequest;
use crate::state::{ConnectionStatus, TurnPlayer, EndTurn};
use crate::client::Client;

pub struct SelectButton;
pub struct DeselectButton;

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

pub fn build_ui(mut c: Commands, mut s: ResMut<SceneLoader>) {
    let file = SceneFile::new("example.client");
    c.ui_root().load_scene_and_edit(&file + "game_container", &mut s, |l| {
        l.edit("status", |l| {
            l.update_on(resource_mutation::<ConnectionStatus>(),
                        |id: UpdateId, mut e: TextEditor, status: ReactRes<ConnectionStatus>| {
                            write_text!(e, *id, "Status: {}", status.to_string());
                        }
            );
        }).edit("turn_player", |l| {
            l.update_on(resource_mutation::<TurnPlayer>(),
                        |id: UpdateId, mut e: TextEditor, owner: ReactRes<TurnPlayer>, client: Res<Client>| {
                            let _ = match owner.display_id() {
                                Some(_display_id) => {
                                    if owner.is_current_turn(&client) {
                                        write_text!(e, *id, "Your Turn")
                                    } else {
                                        write_text!(e, *id, "Opponents Turn")
                                    }
                                },
                                None => write_text!(e, *id, "Not Started"),
                            };
                        }
            );
        }).edit("button", |l| {
            // Update button visual state based on whose turn it is
            l.update_on(resource_mutation::<TurnPlayer>(),
                        move |id: UpdateId, mut c: Commands, owner: ReactRes<TurnPlayer>, client: Res<Client>| {
                            if owner.is_current_turn(&client) {
                                c.react().entity_event(*id, Select);
                            } else {
                                c.react().entity_event(*id, Deselect);
                            }
                        }
            )
                .on_pressed(move |mut c: Commands, owner: ReactRes<TurnPlayer>, client: Res<Client>| {
                    if owner.is_current_turn(&client) {
                        c.react().broadcast(SelectButton);
                    }
                });
        });
    });
}

pub fn handle_button_select(
    mut c: Commands,
    client: Res<Client>,
    status: ReactRes<ConnectionStatus>,
    mut pending_select: ReactResMut<EndTurn>,
    owner: ReactResMut<TurnPlayer>
) {
    // Only allow button interaction if connected and it's the client's turn
    if *status != ConnectionStatus::Connected || !owner.is_current_turn(&client) {
        return;
    }

    // Send end turn request to server
    let signal = client.request(ClientRequest::EndTurn);
    pending_select.get_mut(&mut c).0 = Some(signal);
}

pub fn handle_button_deselect(
    mut c: Commands,
    mut pending_select: ReactResMut<EndTurn>,
    mut owner: ReactResMut<TurnPlayer>
) {
    pending_select.get_mut(&mut c).0 = None;
    owner.get_mut(&mut c).predicted_player_id = None;
}