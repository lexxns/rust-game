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
        let player_id = player.id;
        let connection = player.connection.clone();
        self.online_players.push(player_copy);
        self.waiting_players.push(player);
        self.player_connections.insert(player_id, connection);
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

            let match_msg = PlayerMessage::player_matched(player1.id, player2.id);
            match_msg.send(&self.player_connections);

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
        self.player_connections.remove(player_id);

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

#[cfg(test)]
mod connection_tests {
    use std::collections::HashSet;
    use super::*;
    use tokio::sync::mpsc;
    use std::time::Duration;
    use tokio::time::timeout;
    use tokio_tungstenite::tungstenite::{Message as WsMessage};

    // Helper function to create a test player with working message channels
    async fn create_test_player(name: &str) -> (Player, mpsc::UnboundedReceiver<WsMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let player = Player::new(name.to_string(), PlayerConnection::from(tx));
        (player, rx)
    }

    // Helper to wait for and verify a message
    async fn expect_message(rx: &mut mpsc::UnboundedReceiver<WsMessage>, expected_content: &str) -> bool {
        match timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(Some(WsMessage::Text(content))) => content.contains(expected_content),
            _ => false,
        }
    }

    #[tokio::test]
    async fn test_connection_storage() {
        let mut room_manager = RoomManager::new();

        // Create two players with their receivers
        let (player1, mut rx1) = create_test_player("Player1").await;
        let (player2, mut rx2) = create_test_player("Player2").await;

        let player1_id = player1.id;
        let player2_id = player2.id;

        // Add players and verify connections are stored
        room_manager.add_waiting_player(player1);
        room_manager.add_waiting_player(player2);

        assert!(room_manager.player_connections().contains_key(&player1_id),
                "Player 1 connection should be stored");
        assert!(room_manager.player_connections().contains_key(&player2_id),
                "Player 2 connection should be stored");

        // Test message delivery using stored connections
        let test_msg = "Test message";
        PlayerMessage::private(test_msg, player1_id, player2_id)
            .send(room_manager.player_connections());

        assert!(expect_message(&mut rx2, test_msg).await,
                "Message should be delivered to player 2");
        assert!(!expect_message(&mut rx1, test_msg).await,
                "Message should not be delivered to player 1");
    }

    #[tokio::test]
    async fn test_connection_cleanup() {
        let mut room_manager = RoomManager::new();

        // Create and add players
        let (player1, _rx1) = create_test_player("Player1").await;
        let (player2, mut rx2) = create_test_player("Player2").await;

        let player1_id = player1.id;
        let player2_id = player2.id;

        room_manager.add_waiting_player(player1);
        room_manager.add_waiting_player(player2);

        // Create a room
        let room_result = room_manager.try_create_room();
        assert!(room_result.is_some(), "Room should be created");

        // Disconnect player1
        room_manager.handle_disconnect(&player1_id);

        // Verify cleanup
        assert!(!room_manager.player_connections().contains_key(&player1_id),
                "Player 1 connection should be removed");
        assert!(room_manager.player_connections().contains_key(&player2_id),
                "Player 2 connection should remain");
        assert!(expect_message(&mut rx2, "disconnected").await,
                "Player 2 should receive disconnect notification");
    }

    #[tokio::test]
    async fn test_connection_room_messaging() {
        let mut room_manager = RoomManager::new();

        // Create and add players
        let (player1, mut rx1) = create_test_player("Player1").await;
        let (player2, mut rx2) = create_test_player("Player2").await;

        let player1_id = player1.id;
        let player2_id = player2.id;

        room_manager.add_waiting_player(player1);
        room_manager.add_waiting_player(player2);

        // Create room and verify match notification
        let room_result = room_manager.try_create_room();
        assert!(room_result.is_some(), "Room should be created");

        assert!(expect_message(&mut rx1, "Matched").await,
                "Player 1 should receive match notification");
        assert!(expect_message(&mut rx2, "Matched").await,
                "Player 2 should receive match notification");

        // Test room message delivery
        let room_msg = "Hello room";
        if let Some((room_id, _)) = room_manager.get_room_info(&player1_id) {
            let mut targets = HashSet::new();
            targets.insert(player2_id);
            PlayerMessage::room_broadcast(room_msg, player1_id, targets)
                .send(room_manager.player_connections());
        }

        assert!(expect_message(&mut rx2, room_msg).await,
                "Room message should be delivered to player 2");
    }

    #[tokio::test]
    async fn test_connection_error_handling() {
        let mut room_manager = RoomManager::new();

        // Create player but drop receiver immediately to simulate connection error
        let (player, _rx) = create_test_player("Player1").await;
        let player_id = player.id;

        room_manager.add_waiting_player(player);

        // Verify connection is stored
        assert!(room_manager.player_connections().contains_key(&player_id),
                "Player connection should be initially stored");

        // Simulate disconnect due to error
        room_manager.handle_disconnect(&player_id);

        // Verify cleanup
        assert!(!room_manager.player_connections().contains_key(&player_id),
                "Player connection should be removed after error");
        assert!(room_manager.waiting_players.is_empty(),
                "Player should be removed from waiting list");
        assert!(room_manager.online_players.is_empty(),
                "Player should be removed from online players");
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        let mut room_manager = RoomManager::new();
        let mut receivers = vec![];
        let mut player_ids = vec![];

        // Create multiple players
        for i in 0..5 {
            let (player, rx) = create_test_player(&format!("Player{}", i)).await;
            player_ids.push(player.id);
            receivers.push(rx);
            room_manager.add_waiting_player(player);
        }

        // Verify all connections are stored
        for id in &player_ids {
            assert!(room_manager.player_connections().contains_key(id),
                    "All player connections should be stored");
        }

        // Test room creation for multiple pairs
        while room_manager.try_create_room().is_some() {
            // Room created
        }

        // Verify appropriate number of rooms created
        assert_eq!(room_manager.rooms().len(), 2,
                   "Should create 2 rooms with 5 players");
        assert_eq!(room_manager.waiting_players.len(), 1,
                   "Should have 1 player left waiting");
    }
}