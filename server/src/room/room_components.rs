use std::collections::HashSet;
use bevy::prelude::{Component, Timer};

#[derive(Component)]
pub struct Room {
    pub room_id: String,
}

#[derive(Component)]
pub struct Players {
    pub set: HashSet<u128>
}

#[derive(Component)]
pub struct CurrentTurn {
    pub player: Option<u128>
}

#[derive(Component)]
pub struct TurnTimer {
    pub timer: Timer,
}

#[derive(Component)]
pub struct NextTurn;

#[derive(Component)]
pub struct RoomState {
    pub is_active: bool,
    pub last_update: f32,
}
