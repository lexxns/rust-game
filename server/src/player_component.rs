use bevy::prelude::{Component, Entity, Event};

#[derive(Component)]
pub struct Player {
    pub id: u128,
    pub room: Entity,
}

#[derive(Event)]
pub struct PlayerJoinEvent(pub u128);

#[derive(Event)]
pub struct PlayerLeaveEvent {
    pub player_id: u128,
    pub room_entity: Entity,
}