use std::collections::HashMap;
use bevy::prelude::*;
use crate::room::Room;

#[derive(Resource, Default)]
pub struct GameState {
    pub(crate) rooms: HashMap<String, Room>,
    pub(crate) player_to_room: HashMap<u128, String>,
}

impl GameState {
    pub fn find_or_create_room(&mut self) -> String {
        // Try to find a room with space
        for (room_id, room) in &self.rooms {
            if room.players.len() < 2 {
                return room_id.clone();
            }
        }

        // Create new room if none found
        let room_id = format!("room_{}", self.rooms.len());
        self.rooms.insert(room_id.clone(), Room::default());
        room_id
    }

    // Add other game state management methods...
}