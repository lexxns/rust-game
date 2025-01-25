use bevy_cobweb::prelude::*;
use shared::channel::ChatChannel;
use std::collections::HashMap;

pub type Server = bevy_simplenet::Server<ChatChannel>;
pub type ServerEvent = bevy_simplenet::ServerEventFrom<ChatChannel>;

#[derive(ReactResource, Default)]
pub struct PlayerTurns(pub HashMap<String, Option<u128>>);