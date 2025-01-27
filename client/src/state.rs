use bevy_cobweb::prelude::*;
use shared::EntityID;
use crate::client::{Client};

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