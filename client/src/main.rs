use std::env;
use std::path::PathBuf;
use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy_cobweb::prelude::*;
use bevy_cobweb_ui::prelude::*;
use bevy_inspector_egui::bevy_egui;
use bevy_inspector_egui::bevy_egui::{EguiPlugin, EguiPostUpdateSet};
use wasm_timer::{SystemTime, UNIX_EPOCH};

mod state;
mod ui;
mod client;
mod hand;
mod texture;

use state::{ConnectionStatus, TurnPlayer, EndTurn};
use client::{client_factory, handle_client_events};
use crate::hand::{setup_hand, HandLayoutParams};
use crate::state::{setup_game_state, GameState, SelectedCard, UiState};
use crate::texture::uv_debug_texture;
use crate::ui::{show_ui_system, set_camera_viewport, setup_camera, setup_lighting, setup_play_field};

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
    let bevy_plugins = DefaultPlugins
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
            CobwebUiPlugin,
            EguiPlugin
        ))
        // .add_plugins(WorldInspectorPlugin::new())
        .insert_resource(client)
        .insert_resource(HandLayoutParams::default())
        .insert_resource(AssetDirectory(asset_path.clone()))
        .insert_react_resource(ConnectionStatus::Connecting)
        .insert_resource(UiState::new())
        .insert_resource(GameState::default())
        .init_resource::<HandLayoutParams>()
        .init_resource::<SelectedCard>()
        .init_react_resource::<TurnPlayer>()
        .init_react_resource::<EndTurn>()
        .init_react_resource::<GameState>()
        .add_systems(Startup, (setup, setup_hand))
        .add_systems(Update, (
            handle_client_events,
            hand::update_card_positions,
            hand::update_card_count
        ))
        .add_systems(
            PostUpdate,
            show_ui_system
                .before(EguiPostUpdateSet::ProcessOutput)
                .before(bevy_egui::end_pass_system)
                .before(TransformSystem::TransformPropagate),
        )
        .add_systems(PostUpdate, set_camera_viewport.after(show_ui_system))
        .register_type::<Option<Handle<Image>>>()
        .register_type::<AlphaMode>()
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut game_state: ResMut<GameState>,
) {
    setup_game_state(&mut game_state);
    setup_camera(&mut commands);
    setup_lighting(&mut commands);
    setup_play_field(&mut commands, &mut meshes, &mut materials);
}