use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_tungstenite::tungstenite::Message;
use std::net::SocketAddr;

#[derive(Debug)]
struct GameRoom {
    players: Vec<SocketAddr>,
    broadcast_tx: broadcast::Sender<Message>,
}

impl GameRoom {
    fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        GameRoom {
            players: Vec::new(),
            broadcast_tx,
        }
    }

    fn add_player(&mut self, addr: SocketAddr) -> bool {
        if self.is_full() {
            return false;
        }
        self.players.push(addr);
        true
    }

    fn is_full(&self) -> bool {
        self.players.len() >= 2
    }
}

#[derive(Debug)]
pub struct RoomManager {
    waiting_players: Vec<(SocketAddr, Arc<mpsc::Sender<Message>>)>,
    active_rooms: HashMap<usize, GameRoom>,
    next_room_id: usize,
}

impl RoomManager {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            waiting_players: Vec::new(),
            active_rooms: HashMap::new(),
            next_room_id: 0,
        }))
    }

    pub fn remove_player(&mut self, addr: SocketAddr) {
        println!("Removing player: {}", addr);
        self.waiting_players.retain(|(player_addr, _)| *player_addr != addr);

        let rooms_to_remove: Vec<usize> = self.active_rooms
            .iter()
            .filter(|(_, room)| room.players.contains(&addr))
            .map(|(&id, _)| id)
            .collect();

        for room_id in rooms_to_remove {
            println!("Removing room {}", room_id);
            if let Some(room) = self.active_rooms.remove(&room_id) {
                for player_addr in room.players {
                    if player_addr != addr {
                        println!("Notifying player {} about room closure", player_addr);
                        // Find the player's sender in waiting_players
                        if let Some((_, sender)) = self.waiting_players.iter()
                            .find(|(addr_, _)| *addr_ == player_addr) {
                            let _ = sender.try_send(Message::text(
                                "Your opponent has left. Room closed."
                            ));
                        }
                    }
                }
            }
        }
    }

    pub async fn broadcast_to_room(&self, from_addr: SocketAddr, msg: String) -> bool {
        for (room_id, room) in self.active_rooms.iter() {
            if room.players.contains(&from_addr) {
                println!("Broadcasting message from {} in room {}: {}", from_addr, room_id, msg);
                let broadcast_msg = Message::text(format!("{}: {}", from_addr, msg));
                return match room.broadcast_tx.send(broadcast_msg) {
                    Ok(_) => true,
                    Err(e) => {
                        println!("Failed to broadcast message: {}", e);
                        false
                    }
                };
            }
        }
        println!("No room found for player {}", from_addr);
        false
    }

    pub fn get_room_broadcast(&self, addr: SocketAddr) -> Option<&broadcast::Sender<Message>> {
        for room in self.active_rooms.values() {
            if room.players.contains(&addr) {
                return Some(&room.broadcast_tx);
            }
        }
        None
    }

    pub async fn add_player(&mut self, addr: SocketAddr, sender: Arc<mpsc::Sender<Message>>) -> Option<broadcast::Receiver<Message>> {
        println!("Adding player: {}", addr);

        // First check if player is already in a room and return a new subscription
        if let Some(broadcaster) = self.get_room_broadcast(addr) {
            println!("Player {} already in room, returning new subscription", addr);
            return Some(broadcaster.subscribe());
        }

        // Add to waiting players if not already waiting
        if !self.waiting_players.iter().any(|(a, _)| *a == addr) {
            self.waiting_players.push((addr, sender.clone()));
        }

        if self.waiting_players.len() >= 2 {
            println!("Creating new room with 2 players");
            let room_id = self.next_room_id;
            self.next_room_id += 1;

            let mut room = GameRoom::new();
            let room_tx = room.broadcast_tx.clone();

            // Take first two waiting players
            let mut players_to_notify = Vec::new();
            while room.players.len() < 2 && !self.waiting_players.is_empty() {
                if let Some((player_addr, player_sender)) = self.waiting_players.pop() {
                    println!("Adding player {} to room {}", player_addr, room_id);
                    room.add_player(player_addr);
                    players_to_notify.push((player_addr, player_sender));
                }
            }

            self.active_rooms.insert(room_id, room);

            // Notify players they've been matched and return a receiver for the current player
            for (player_addr, player_sender) in &players_to_notify {
                println!("Sending match notification to player {}", player_addr);
                let _ = player_sender.send(Message::text(format!(
                    "Matched! You are now in room {}",
                    room_id
                ))).await;
            }

            // Always return a new subscription for the requesting player
            Some(room_tx.subscribe())
        } else {
            println!("Player {} waiting for match", addr);
            let _ = sender.send(Message::text("Waiting for another player...")).await;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tokio::sync::mpsc;

    /// Creates two test players with their associated channels for testing
    async fn setup_two_players() -> (
        Arc<RwLock<RoomManager>>,
        SocketAddr,
        SocketAddr,
        Arc<mpsc::Sender<Message>>,
        Arc<mpsc::Sender<Message>>,
        mpsc::Receiver<Message>,
        mpsc::Receiver<Message>,
    ) {
        let room_manager = RoomManager::new();

        // Create test addresses
        let player1_addr = SocketAddr::from_str("127.0.0.1:8001").unwrap();
        let player2_addr = SocketAddr::from_str("127.0.0.1:8002").unwrap();

        // Create channels for both players
        let (player1_tx, player1_rx) = mpsc::channel(100);
        let (player2_tx, player2_rx) = mpsc::channel(100);

        let player1_tx = Arc::new(player1_tx);
        let player2_tx = Arc::new(player2_tx);

        (
            room_manager,
            player1_addr,
            player2_addr,
            player1_tx,
            player2_tx,
            player1_rx,
            player2_rx,
        )
    }

    #[tokio::test]
    async fn test_room_creation() {
        let (
            room_manager,
            player1_addr,
            player2_addr,
            player1_tx,
            player2_tx,
            mut player1_rx,
            mut player2_rx,
        ) = setup_two_players().await;

        // Add first player
        {
            let mut manager = room_manager.write().await;
            let receiver = manager.add_player(player1_addr, player1_tx).await;
            assert!(receiver.is_none(), "First player should not get a receiver yet");
        }

        // Verify waiting message
        let msg = player1_rx.recv().await.unwrap();
        assert_eq!(
            msg.to_string(),
            "Waiting for another player...",
            "First player should receive waiting message"
        );

        // Add second player
        {
            let mut manager = room_manager.write().await;
            let receiver = manager.add_player(player2_addr, player2_tx).await;
            assert!(receiver.is_some(), "Second player should get a receiver");
        }

        // Verify both players get matched messages
        let msg1 = player1_rx.recv().await.unwrap();
        let msg2 = player2_rx.recv().await.unwrap();
        assert!(
            msg1.to_string().contains("Matched!"),
            "First player should receive match notification"
        );
        assert!(
            msg2.to_string().contains("Matched!"),
            "Second player should receive match notification"
        );
    }
    #[tokio::test]
    async fn test_message_broadcast() {
        let (
            room_manager,
            player1_addr,
            player2_addr,
            player1_tx,
            player2_tx,
            mut player1_rx,
            mut player2_rx,
        ) = setup_two_players().await;

        // Add first player and store their receiver
        let player1_room_rx = {
            let mut manager = room_manager.write().await;
            manager.add_player(player1_addr, player1_tx.clone()).await
        };

        // Clear waiting message
        let _ = player1_rx.recv().await;

        // Add second player and store their receiver
        let player2_room_rx = {
            let mut manager = room_manager.write().await;
            manager.add_player(player2_addr, player2_tx.clone()).await
        };

        // Must be Some because it's the second player
        assert!(player2_room_rx.is_some(), "Second player should get room receiver");

        // Clear the match notification messages
        let _ = player1_rx.recv().await;
        let _ = player2_rx.recv().await;

        // Get room receivers for both players
        let manager = room_manager.read().await;
        let room_broadcast = manager.get_room_broadcast(player1_addr)
            .expect("Room should exist");
        let mut player1_room_rx = player1_room_rx.unwrap_or_else(|| room_broadcast.subscribe());
        let mut player2_room_rx = player2_room_rx.unwrap();
        drop(manager); // Release the read lock

        let player1_forward = tokio::spawn(async move {
            while let Ok(msg) = player1_room_rx.recv().await {
                if player1_tx.send(msg).await.is_err() {
                    break;
                }
            }
        });

        let player2_forward = tokio::spawn(async move {
            while let Ok(msg) = player2_room_rx.recv().await {
                if player2_tx.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Test message broadcast
        {
            let manager = room_manager.read().await;
            let result = manager.broadcast_to_room(player1_addr, "Hello from player 1".to_string()).await;
            assert!(result, "Message broadcast should succeed");
        }

        // Verify both players receive the message
        let msg1 = tokio::time::timeout(std::time::Duration::from_secs(1), player1_rx.recv()).await
            .expect("Timeout waiting for player 1 message")
            .expect("Failed to receive message");

        let msg2 = tokio::time::timeout(std::time::Duration::from_secs(1), player2_rx.recv()).await
            .expect("Timeout waiting for player 2 message")
            .expect("Failed to receive message");

        assert_eq!(
            msg1.to_string(),
            format!("{}: Hello from player 1", player1_addr),
            "First player should receive their own message"
        );
        assert_eq!(
            msg2.to_string(),
            format!("{}: Hello from player 1", player1_addr),
            "Second player should receive first player's message"
        );

        // Clean up
        player1_forward.abort();
        player2_forward.abort();
    }

    #[tokio::test]
    async fn test_player_disconnect() {
        let (
            room_manager,
            player1_addr,
            player2_addr,
            player1_tx,
            player2_tx,
            mut player1_rx,
            mut player2_rx,
        ) = setup_two_players().await;

        // Setup room with both players
        {
            let mut manager = room_manager.write().await;
            manager.add_player(player1_addr, player1_tx).await;
            manager.add_player(player2_addr, player2_tx).await;
        }

        // Clear the initial match messages
        let _ = player1_rx.recv().await;
        let _ = player2_rx.recv().await;

        // Disconnect player 1
        {
            let mut manager = room_manager.write().await;
            manager.remove_player(player1_addr);
        }

        // Verify player 2 receives disconnect notification
        let msg = player2_rx.recv().await.unwrap();
        assert_eq!(
            msg.to_string(),
            "Your opponent has left. Room closed.",
            "Remaining player should be notified of disconnect"
        );

        // Verify broadcasting no longer works
        {
            let manager = room_manager.read().await;
            let result = manager.broadcast_to_room(player2_addr, "Hello?".to_string()).await;
            assert!(!result, "Message broadcast should fail after room closure");
        }
    }

    #[tokio::test]
    async fn test_multiple_messages() {
        let (
            room_manager,
            player1_addr,
            player2_addr,
            player1_tx,
            player2_tx,
            mut player1_rx,
            mut player2_rx,
        ) = setup_two_players().await;

        // Add first player and store their receiver
        let player1_room_rx = {
            let mut manager = room_manager.write().await;
            manager.add_player(player1_addr, player1_tx.clone()).await
        };

        // Clear waiting message
        let _ = player1_rx.recv().await;

        // Add second player and store their receiver
        let player2_room_rx = {
            let mut manager = room_manager.write().await;
            manager.add_player(player2_addr, player2_tx.clone()).await
        };

        // Clear the match notification messages
        let _ = player1_rx.recv().await;
        let _ = player2_rx.recv().await;

        // Get room receivers for both players
        let manager = room_manager.read().await;
        let room_broadcast = manager.get_room_broadcast(player1_addr)
            .expect("Room should exist");
        let mut player1_room_rx = player1_room_rx.unwrap_or_else(|| room_broadcast.subscribe());
        let mut player2_room_rx = player2_room_rx.unwrap();
        drop(manager); // Release the read lock

        // Set up message forwarding tasks
        let player1_forward = tokio::spawn(async move {
            while let Ok(msg) = player1_room_rx.recv().await {
                if player1_tx.send(msg).await.is_err() {
                    break;
                }
            }
        });

        let player2_forward = tokio::spawn(async move {
            while let Ok(msg) = player2_room_rx.recv().await {
                if player2_tx.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Test sequence of messages
        let messages = vec![
            (player1_addr, "Message 1 from player 1"),
            (player2_addr, "Message 1 from player 2"),
            (player1_addr, "Message 2 from player 1"),
            (player2_addr, "Message 2 from player 2"),
        ];

        // Send all messages
        for (addr, msg) in &messages {
            let manager = room_manager.read().await;
            let result = manager.broadcast_to_room(*addr, msg.to_string()).await;
            assert!(result, "Message broadcast should succeed");
            // Small delay to ensure message ordering
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Verify all messages are received by both players in order
        for (addr, msg) in &messages {
            let expected_msg = format!("{}: {}", addr, msg);

            // Add timeouts to prevent test from hanging
            let msg1 = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                player1_rx.recv()
            ).await.expect("Timeout waiting for player 1 message")
                .expect("Failed to receive message");

            let msg2 = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                player2_rx.recv()
            ).await.expect("Timeout waiting for player 2 message")
                .expect("Failed to receive message");

            assert_eq!(
                msg1.to_string(),
                expected_msg,
                "Player 1 should receive message in order"
            );
            assert_eq!(
                msg2.to_string(),
                expected_msg,
                "Player 2 should receive message in order"
            );
        }

        // Clean up
        player1_forward.abort();
        player2_forward.abort();
    }

}