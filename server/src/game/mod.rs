use bevy::prelude::*;
use bevy_cobweb::prelude::{ReactAppExt, ReactResAppExt};
use crate::types::*;

mod systems;
mod state;
pub mod turn_timer;

pub use state::GameState;
use systems::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameState>()
            .insert_react_resource(PlayerTurns::default())
            .add_systems(Main, handle_server_events);

        // Changed from add_systems to react
        app.react(|rc| {
            setup_room_state_reaction(rc);
        });
    }
}