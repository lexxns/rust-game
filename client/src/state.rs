use bevy_cobweb::prelude::*;

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
pub struct ButtonOwner {
    pub server_authoritative_id: Option<u128>,
    pub predicted_id: Option<u128>
}

impl ButtonOwner {
    pub fn display_id(&self) -> Option<u128> {
        if self.predicted_id.is_some() { return self.predicted_id }
        self.server_authoritative_id
    }
}

#[derive(ReactResource)]
pub struct PendingSelect(pub Option<bevy_simplenet::RequestSignal>);

impl PendingSelect {
    pub fn equals_request(&self, request_id: u64) -> bool {
        let Some(signal) = &self.0 else { return false; };
        signal.id() == request_id
    }

    pub fn is_predicted(&self) -> bool {
        self.0.is_some()
    }
}

impl Default for PendingSelect { fn default() -> Self { Self(None) } }