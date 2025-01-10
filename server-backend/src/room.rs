use std::collections::HashMap;
use uuid::Uuid;
use shared::message_utils::PlayerConnection;
use crate::messages::{CommsMessage, PlayerMessage};

// Represents a connected player
#[derive(Clone)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    pub connection: PlayerConnection,
}

impl Player {
    pub fn new(name: String, sender: PlayerConnection) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            connection: sender,
        }
    }
}

// Represents a private room with two players
pub struct Room {
    pub player1: Uuid,
    pub player2: Uuid,
}

// Global state management for rooms
pub struct RoomManager {
    waiting_players: Vec<Player>,
    rooms: HashMap<Uuid, Room>,
    player_to_room: HashMap<Uuid, Uuid>,
    player_connections: HashMap<Uuid, PlayerConnection>,
    online_players: Vec<Player>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            waiting_players: Vec::new(),
            rooms: HashMap::new(),
            player_to_room: HashMap::new(),
            player_connections: HashMap::new(),
            online_players: Vec::new(),
        }
    }

    pub fn rooms(&self) -> &HashMap<Uuid, Room> {
        &self.rooms
    }

    pub fn players(&self) -> &Vec<Player> {
        &self.online_players
    }

    pub fn player_connections(&self) -> &HashMap<Uuid, PlayerConnection> {
        &self.player_connections
    }

    pub fn get_player_id(&self, name: String) -> Option<Uuid> {
        let res = self.online_players.iter().find(|&p| p.name == name);
        res.map(|p| p.id)
    }

    // Add a player to the waiting list
    pub fn add_waiting_player(&mut self, player: Player) {
        let player_copy = player.clone();
        self.online_players.push(player_copy);
        self.waiting_players.push(player);
    }

    // Try to match players and create a room
    pub fn try_create_room(&mut self) -> Option<(Uuid, Uuid)> {
        if self.waiting_players.len() >= 2 {
            let player2 = self.waiting_players.pop()?;
            let player1 = self.waiting_players.pop()?;

            let room_id = Uuid::new_v4();
            let room = Room {
                player1: player1.id,
                player2: player2.id,
            };

            self.rooms.insert(room_id, room);
            self.player_to_room.insert(player1.id, room_id);
            self.player_to_room.insert(player2.id, room_id);

            Some((player1.id, player2.id))
        } else {
            None
        }
    }

    // Get sender for a specific player
    pub fn get_player_sender(&self, player_id: &Uuid) -> Option<PlayerConnection> {
        let res = self.online_players.iter().find(|&p| p.id == *player_id);
        res.map(|p| p.connection.clone())
    }

    // Get room and other player info for a given player
    pub fn get_room_info(&self, player_id: &Uuid) -> Option<(Uuid, Uuid)> {
        let room_id = self.player_to_room.get(player_id)?;
        let room = self.rooms.get(room_id)?;

        let other_player_id = if room.player1 == *player_id {
            room.player2
        } else {
            room.player1
        };

        Some((*room_id, other_player_id))
    }

    // Handle player disconnect and notify other player if needed
    pub fn handle_disconnect(&mut self, player_id: &Uuid) {
        self.waiting_players.retain(|p| p.id != *player_id);

        if let Some(room_id) = self.player_to_room.remove(player_id) {
            if let Some(room) = self.rooms.remove(&room_id) {
                let other_player_id = if room.player1 == *player_id {
                    room.player2
                } else {
                    room.player1
                };
                self.player_to_room.remove(&other_player_id);

                // Notify other player about disconnect
                PlayerMessage::player_disconnected(other_player_id).send(&self.player_connections);
            }
        }
        self.online_players.retain(|p| p.id != *player_id);
    }
}