use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::prelude::*;
use std::fmt::Write;
use shared::channel::ClientRequest;
use crate::state::{ConnectionStatus, ButtonOwner, PendingSelect};
use crate::client::DemoClient;

pub struct SelectButton;
pub struct DeselectButton;
pub struct ChatInputSelected;

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
        }).edit("owner", |l| {
            l.update_on(resource_mutation::<ButtonOwner>(),
                        |id: UpdateId, mut e: TextEditor, owner: ReactRes<ButtonOwner>| {
                            let _ = match owner.display_id() {
                                Some(display_id) => write_text!(e, *id, "Owner: {}", display_id % 1_000_000u128),
                                None => write_text!(e, *id, "No owner"),
                            };
                        }
            );
        }).edit("button", |l| {
            let button = l.id();
            l.on_pressed(move |mut c: Commands| {
                c.react().entity_event(button, Select);
                c.react().broadcast(SelectButton);
            })
                .update_on(broadcast::<DeselectButton>(), |id: UpdateId, mut c: Commands| {
                    c.react().entity_event(*id, Deselect);
                })
                .on_select(|| println!("selected"))
                .on_deselect(|| println!("deselected"));
        });
    });
}

pub fn handle_button_select(
    mut c: Commands,
    client: Res<DemoClient>,
    status: ReactRes<ConnectionStatus>,
    mut pending_select: ReactResMut<PendingSelect>,
    mut owner: ReactResMut<ButtonOwner>
) {
    if *status != ConnectionStatus::Connected {
        c.react().broadcast(DeselectButton);
        return;
    }

    let signal = client.request(ClientRequest::Select);
    pending_select.get_mut(&mut c).0 = Some(signal);
    owner.get_mut(&mut c).predicted_id = Some(client.id());
}

pub fn handle_button_deselect(
    mut c: Commands,
    mut pending_select: ReactResMut<PendingSelect>,
    mut owner: ReactResMut<ButtonOwner>
) {
    pending_select.get_mut(&mut c).0 = None;
    owner.get_mut(&mut c).predicted_id = None;
}