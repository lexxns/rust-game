use std::env;
use std::path::PathBuf;
use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy::winit::{UpdateMode, WinitSettings};
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::prelude::*;
use wasm_timer::{SystemTime, UNIX_EPOCH};

mod state;
mod ui;
mod client;
mod hand;
mod texture;

use state::{ConnectionStatus, TurnPlayer, EndTurn};
use ui::{build_ui, setup};
use client::{client_factory, handle_client_events};

#[derive(Resource)]
struct AssetDirectory(PathBuf);

fn main() {
    // simplenet client setup
    let client = client_factory().new_client(
        enfync::builtin::Handle::default(),
        url::Url::parse("ws://127.0.0.1:48888/ws").unwrap(),
        bevy_simplenet::AuthRequest::None{
            client_id: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
        },
        bevy_simplenet::ClientConfig{
            reconnect_on_disconnect   : true,
            reconnect_on_server_close : true,
            ..Default::default()
        },
        ()
    );

    // prepare bevy plugins
    let bevy_plugins = bevy::DefaultPlugins
        .set(
            WindowPlugin{
                primary_window: Some(Window{ window_theme: Some(WindowTheme::Dark), ..Default::default() }),
                ..Default::default()
            }
        );

    // reduce input lag on native targets
    #[cfg(not(target_family = "wasm"))]
    let bevy_plugins = bevy_plugins.build().disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>();

    let mut asset_path = env::current_dir().expect("Failed to get current directory");
    asset_path.push("client/assets");

    // run client
    App::new()
        .add_plugins((
            bevy_plugins,
            ReactPlugin,
            CobwebUiPlugin
        ))
        .insert_resource(WinitSettings{
            focused_mode   : UpdateMode::reactive(std::time::Duration::from_millis(100)),
            unfocused_mode : UpdateMode::reactive(std::time::Duration::from_millis(100)),
            ..Default::default()
        })
        .insert_resource(client)
        .insert_resource(AssetDirectory(asset_path.clone()))
        .insert_react_resource(ConnectionStatus::Connecting)
        .init_react_resource::<TurnPlayer>()
        .init_react_resource::<EndTurn>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(LoadState::Done), build_ui)
        .add_systems(Update, handle_client_events)
        .add_reactor(broadcast::<ui::SelectButton>(), ui::handle_button_select)
        .add_reactor(broadcast::<ui::DeselectButton>(), ui::handle_button_deselect)
        .load("main.cob")
        .run();
}