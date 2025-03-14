use std::any::TypeId;
use bevy::prelude::Resource;
use bevy_asset::UntypedAssetId;
use bevy_cobweb::prelude::*;
use bevy_inspector_egui::bevy_inspector::hierarchy::SelectedEntities;
use egui_dock::DockState;
use shared::channel::CardData;
use shared::EntityID;
use crate::client::{Client};

pub(crate) fn setup_game_state(game_state: &mut GameState) {
    game_state.player_hand = vec![];
    game_state.player_health = 30;
    game_state.opponent_health = 30;
    game_state.available_mana = 10;
}

#[derive(ReactResource, Copy, Clone, Eq, PartialEq, Debug)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Dead,
}

impl ConnectionStatus {
    pub fn to_string(&self) -> &'static str {
        match *self {
            ConnectionStatus::Connecting => "connecting...",
            ConnectionStatus::Connected  => "connected",
            ConnectionStatus::Dead       => "DEAD",
        }
    }
}

#[derive(ReactResource, Default)]
pub struct TurnPlayer {
    pub server_determined_player_id: Option<EntityID>,
    pub predicted_player_id: Option<EntityID>
}

impl TurnPlayer {
    pub fn display_id(&self) -> Option<EntityID> {
        if self.predicted_player_id.is_some() { return self.predicted_player_id }
        self.server_determined_player_id
    }

    pub fn is_current_turn(&self, client: &Client) -> bool {
        self.display_id().map_or(false, |id| id == client.id())
    }
}

#[derive(ReactResource)]
pub struct EndTurn(pub Option<bevy_simplenet::RequestSignal>);

impl EndTurn {
    pub fn equals_request(&self, request_id: u64) -> bool {
        let Some(signal) = &self.0 else { return false; };
        signal.id() == request_id
    }

    pub fn is_predicted(&self) -> bool {
        self.0.is_some()
    }
}

impl Default for EndTurn { fn default() -> Self { Self(None) } }

#[derive(Default, PartialEq, Clone)]
pub(crate) enum Turn {
    #[default]
    Player,
    Opponent,
}

#[derive(Resource, Default)]
pub(crate) struct SelectedCard {
    pub(crate) index: Option<usize>,
}

#[derive(Resource, Default)]
pub(crate) struct GameState {
    pub(crate) player_hand: Vec<CardData>,
    pub(crate) play_field: Vec<CardData>,
    pub(crate) player_health: u32,
    pub(crate) opponent_health: u32,
    pub(crate) current_turn: Turn,
    pub(crate) available_mana: u32,
}

#[derive(Resource)]
pub (crate) struct UiState {
    pub(crate) state: DockState<GameWindow>,
    pub(crate) viewport_rect: egui::Rect,
    pub(crate) selected_entities: SelectedEntities,
    pub(crate) selection: GameSelection,
}

#[derive(Eq, PartialEq)]
pub(crate) enum GameSelection {
    CardInHand(usize),
    CardInPlay(usize),
    CardDetail(TypeId, String),
    InventoryItem(TypeId, String, UntypedAssetId),
}

#[derive(Debug)]
pub(crate) enum GameWindow {
    PlayingField,   // Main game view
    PlayerHand,     // Card hand
    CardCollection, // Card collection/deck building
    Inventory,      // Player inventory
    CardDetail,     // Card details/inspector
}

impl ReactResource for GameState {}