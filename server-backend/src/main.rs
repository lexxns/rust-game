use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// Represents a connected player
struct Player {
    id: Uuid,
    sender: mpsc::UnboundedSender<Message>,
}

// Represents a private room with two players
struct Room {
    player1: Uuid,
    player2: Uuid,
}

// Global state management
struct ServerState {
    waiting_players: Vec<Player>,
    rooms: HashMap<Uuid, Room>,
    player_to_room: HashMap<Uuid, Uuid>,
    player_senders: HashMap<Uuid, mpsc::UnboundedSender<Message>>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            waiting_players: Vec::new(),
            rooms: HashMap::new(),
            player_to_room: HashMap::new(),
            player_senders: HashMap::new(),
        }
    }

    // Try to match players and create a room
    fn try_create_room(&mut self) -> Option<(Uuid, Uuid)> {
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
}

async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    state: Arc<RwLock<ServerState>>,
) {
    let player_id = Uuid::new_v4();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (message_tx, mut message_rx) = mpsc::unbounded_channel();

    // First, store the sender in the state
    {
        let mut state = state.write().await;
        state.player_senders.insert(player_id, message_tx.clone());
        state.waiting_players.push(Player {
            id: player_id,
            sender: message_tx,
        });

        // Try to create a room if possible
        if let Some((player1_id, player2_id)) = state.try_create_room() {
            // Notify both players they've been matched
            let match_msg = Message::Text(Utf8Bytes::from("Matched with player!"));
            if let Some(sender1) = state.player_senders.get(&player1_id) {
                let _ = sender1.send(match_msg.clone());
            }
            if let Some(sender2) = state.player_senders.get(&player2_id) {
                let _ = sender2.send(match_msg);
            }
        }
    }

    // Handle outgoing messages
    tokio::spawn(async move {
        while let Some(msg) = message_rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(msg) => {
                let state = state.read().await;
                if let Some(room_id) = state.player_to_room.get(&player_id) {
                    if let Some(room) = state.rooms.get(room_id) {
                        let other_player_id = if room.player1 == player_id {
                            room.player2
                        } else {
                            room.player1
                        };

                        if let Some(other_sender) = state.player_senders.get(&other_player_id) {
                            let _ = other_sender.send(msg);
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Clean up when player disconnects
    {
        let mut state = state.write().await;
        state.waiting_players.retain(|p| p.id != player_id);
        state.player_senders.remove(&player_id);

        if let Some(room_id) = state.player_to_room.remove(&player_id) {
            if let Some(room) = state.rooms.remove(&room_id) {
                let other_player_id = if room.player1 == player_id {
                    room.player2
                } else {
                    room.player1
                };
                state.player_to_room.remove(&other_player_id);

                if let Some(other_sender) = state.player_senders.get(&other_player_id) {
                    let _ = other_sender.send(Message::Text(Utf8Bytes::from("Your partner has disconnected")));
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(ServerState::new()));
    let addr = "127.0.0.1:8080";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("WebSocket server listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .expect("Error during WebSocket handshake");

        let state = Arc::clone(&state);
        tokio::spawn(async move {
            handle_connection(ws_stream, state).await;
        });
    }
}