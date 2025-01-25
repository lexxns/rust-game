use bevy::app::*;
use bevy::log::tracing_subscriber;
use bevy::time::TimePlugin;
use bevy_cobweb::prelude::ReactPlugin;
use crate::server::setup_server;
use crate::game::GamePlugin;
use crate::game::turn_timer::{spawn_turn_timer, update_turn_timers, ActiveTurnTimers};

mod server;
mod game;
mod room;
mod types;

fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let server = setup_server();

    let mut app = App::new();
    app.add_plugins((
        ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(100)),
        ReactPlugin,
        GamePlugin,
        TimePlugin::default(),
    ))
        .insert_resource(server)
        .init_resource::<ActiveTurnTimers>()
        .add_systems(Update, (
            spawn_turn_timer,
            update_turn_timers,
        ));

    app.run();
}