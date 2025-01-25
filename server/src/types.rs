use bevy_cobweb::prelude::*;
use shared::channel::ChatChannel;
use std::collections::HashMap;

pub type DemoServer = bevy_simplenet::Server<ChatChannel>;
pub type DemoServerEvent = bevy_simplenet::ServerEventFrom<ChatChannel>;

#[derive(ReactResource, Default)]
pub struct ButtonStates(pub HashMap<String, Option<u128>>);