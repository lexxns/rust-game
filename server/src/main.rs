use bevy::app::*;
use bevy::log::tracing_subscriber;
use bevy::time::TimePlugin;
use bevy_cobweb::prelude::ReactPlugin;
use crate::room::room_plugin::RoomPlugin;
use crate::server::setup_server;
use crate::server_plugin::handle_server_events;

mod server;
mod types;
mod player_component;
mod server_plugin;
mod room;

fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let server = setup_server();

    App::new()
        .add_plugins((
            ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(100)),
            ReactPlugin,
            RoomPlugin,
            TimePlugin::default(),
        ))
        .insert_resource(server)
        .add_systems(Update, handle_server_events)
        .run();
}