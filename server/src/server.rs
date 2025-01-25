use crate::types::*;
use shared::api::API_VERSION;
use bevy_simplenet::{ServerFactory, AcceptorConfig, Authenticator, ServerConfig};
use shared::channel::ChatChannel;

pub fn setup_server() -> Server {
    ServerFactory::<ChatChannel>::new(API_VERSION)
        .new_server(
            enfync::builtin::native::TokioHandle::default(),
            "127.0.0.1:48888",
            AcceptorConfig::Default,
            Authenticator::None,
            ServerConfig {
                heartbeat_interval: std::time::Duration::from_secs(6),
                ..Default::default()
            },
        )
}