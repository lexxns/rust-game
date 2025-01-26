use shared::channel::GameChannel;

pub type Server = bevy_simplenet::Server<GameChannel>;
pub type ServerEvent = bevy_simplenet::ServerEventFrom<GameChannel>;
