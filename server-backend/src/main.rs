use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::{Message as WsMessage, Utf8Bytes};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use shared::message_utils::{MessageType, IncomingMessage, PlayerConnection};

mod room;
mod messages;
use room::{RoomManager, Player};
use messages::{handle_incoming_message, parse_incoming_message, CommsMessage, PlayerMessage};

// Core connection handler that manages the business logic
pub struct ConnectionHandler {
    state: Arc<RwLock<RoomManager>>,
}

impl ConnectionHandler {
    pub fn new(state: Arc<RwLock<RoomManager>>) -> Self {
        Self { state }
    }

    // Handle an incoming message and return responses
    pub async fn handle_message(&self, incoming: IncomingMessage, player_id: Option<Uuid>) -> Vec<WsMessage> {
        let mut responses = Vec::new();

        match (&incoming.message_type, player_id) {
            (MessageType::Connect { name }, None) => {
                // Handle initial connection
                let (message_tx, _) = mpsc::unbounded_channel();
                let player = Player::new(name.to_string(), message_tx);
                let new_player_id = player.id;

                let mut room_manager = self.state.write().await;
                room_manager.add_waiting_player(player);

                // Try to create a room
                if let Some((p1_id, p2_id)) = room_manager.try_create_room() {
                    let mut targets = HashSet::new();
                    targets.insert(p1_id);
                    targets.insert(p2_id);
                    let match_msg = PlayerMessage::player_matched(p1_id, p2_id);
                    match_msg.send(room_manager.player_connections());
                }

                responses.push(WsMessage::Text(Utf8Bytes::from("Connected successfully")));
            },
            (MessageType::Room(content), Some(player_id)) => {
                let room_manager = self.state.read().await;
                if let Some((_, other_player_id)) = room_manager.get_room_info(&player_id) {
                    let mut targets = HashSet::new();
                    targets.insert(other_player_id);
                    let msg = PlayerMessage::room_broadcast(content.to_string(), player_id, targets);
                    msg.send(room_manager.player_connections());
                } else {
                    let msg = PlayerMessage::system("You are not in a room", player_id);
                    msg.send(room_manager.player_connections());
                }
            },
            (MessageType::Private { recipient, content }, Some(player_id)) => {
                let room_manager = self.state.read().await;
                if let Some(recipient_id) = room_manager.get_player_id(recipient.to_string()) {
                    let msg = PlayerMessage::private(content.to_string(), player_id, recipient_id);
                    msg.send(room_manager.player_connections());
                } else {
                    let msg = PlayerMessage::system("Recipient not found", player_id);
                    msg.send(room_manager.player_connections());
                }
            },
            (MessageType::System(content), Some(player_id)) => {
                let room_manager = self.state.read().await;
                let msg = PlayerMessage::system(content.to_string(), player_id);
                msg.send(room_manager.player_connections());
            },
            (MessageType::Connect { .. }, Some(_)) => {
                responses.push(WsMessage::Text(Utf8Bytes::from("Already connected")));
            },
            (_, None) => {
                responses.push(WsMessage::Text(Utf8Bytes::from("First message must be Connect")));
            }
        }

        responses
    }
}

// Main WebSocket connection handler
pub async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    state: Arc<RwLock<RoomManager>>,
) {
    let handler = ConnectionHandler::new(state);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut player_id = None;

    // First message must be a Connect message
    if let Some(Ok(msg)) = ws_receiver.next().await {
        match parse_incoming_message(msg) {
            Ok(incoming) => {
                match &incoming.message_type {
                    MessageType::Connect { name } => {
                        let name = name.clone(); // Clone the name before moving incoming
                        let responses = handler.handle_message(incoming, None).await;
                        for response in responses {
                            if ws_sender.send(response).await.is_err() {
                                return;
                            }
                        }
                        player_id = Some(handler.state.read().await.get_player_id(name).unwrap());
                    },
                    _ => {
                        let error_msg = WsMessage::Text(Utf8Bytes::from("First message must be Connect"));
                        let _ = ws_sender.send(error_msg).await;
                        return;
                    }
                }
            },
            Err(_) => {
                let error_msg = WsMessage::Text(Utf8Bytes::from("Failed to parse connect message"));
                let _ = ws_sender.send(error_msg).await;
                return;
            }
        }
    }

    // Main message handling loop
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match parse_incoming_message(msg) {
            Ok(incoming) => {
                let responses = handler.handle_message(incoming, player_id).await;
                for response in responses {
                    if ws_sender.send(response).await.is_err() {
                        break;
                    }
                }
            },
            Err(_) => {
                let error_msg = WsMessage::Text(Utf8Bytes::from("Failed to parse message"));
                if ws_sender.send(error_msg).await.is_err() {
                    break;
                }
            }
        }
    }

    // Clean up when connection ends
    if let Some(pid) = player_id {
        let mut state = handler.state.write().await;
        state.handle_disconnect(&pid);
    }
}

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(RoomManager::new()));
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_handler() -> (ConnectionHandler, Arc<RwLock<RoomManager>>) {
        let state = Arc::new(RwLock::new(RoomManager::new()));
        let handler = ConnectionHandler::new(state.clone());
        (handler, state)
    }

    #[tokio::test]
    async fn test_connection_flow() {
        let (handler, state) = create_test_handler().await;

        // Test connect message
        let connect_msg = IncomingMessage {
            message_type: MessageType::Connect {
                name: "Player1".to_string()
            }
        };

        let responses = handler.handle_message(connect_msg, None).await;
        assert!(responses.iter().any(|msg| match msg {
            WsMessage::Text(text) => text.contains("Connected successfully"),
            _ => false
        }));

        // Verify player was added
        let room_manager = state.read().await;
        assert_eq!(room_manager.players().len(), 1);
        assert!(room_manager.get_player_id("Player1".to_string()).is_some());
    }

    #[tokio::test]
    async fn test_room_matching() {
        let (handler, state) = create_test_handler().await;

        // Connect first player
        let connect_msg1 = IncomingMessage {
            message_type: MessageType::Connect {
                name: "Player1".to_string()
            }
        };
        let _ = handler.handle_message(connect_msg1, None).await;

        // Connect second player
        let connect_msg2 = IncomingMessage {
            message_type: MessageType::Connect {
                name: "Player2".to_string()
            }
        };
        let _ = handler.handle_message(connect_msg2, None).await;

        // Both should be in a room
        let room_manager = state.read().await;
        assert!(!room_manager.rooms().is_empty());
    }

    #[tokio::test]
    async fn test_room_message() {
        let (handler, state) = create_test_handler().await;

        // Set up two players
        let connect_msg1 = IncomingMessage {
            message_type: MessageType::Connect {
                name: "Player1".to_string()
            }
        };
        let _ = handler.handle_message(connect_msg1, None).await;
        let player1_id = state.read().await.get_player_id("Player1".to_string()).unwrap();

        let connect_msg2 = IncomingMessage {
            message_type: MessageType::Connect {
                name: "Player2".to_string()
            }
        };
        let _ = handler.handle_message(connect_msg2, None).await;

        // Send room message
        let room_msg = IncomingMessage {
            message_type: MessageType::Room("Hello room!".to_string())
        };
        let responses = handler.handle_message(room_msg, Some(player1_id)).await;

        // Message should be handled (actual delivery tested in room_manager tests)
        assert!(responses.is_empty());
    }

    #[tokio::test]
    async fn test_private_message() {
        let (handler, state) = create_test_handler().await;

        // Connect two players
        let connect_msg1 = IncomingMessage {
            message_type: MessageType::Connect {
                name: "Player1".to_string()
            }
        };
        let _ = handler.handle_message(connect_msg1, None).await;
        let player1_id = state.read().await.get_player_id("Player1".to_string()).unwrap();

        let connect_msg2 = IncomingMessage {
            message_type: MessageType::Connect {
                name: "Player2".to_string()
            }
        };
        let _ = handler.handle_message(connect_msg2, None).await;

        // Send private message
        let private_msg = IncomingMessage {
            message_type: MessageType::Private {
                recipient: "Player2".to_string(),
                content: "Secret message".to_string()
            }
        };
        let responses = handler.handle_message(private_msg, Some(player1_id)).await;

        // Message should be handled (actual delivery tested in room_manager tests)
        assert!(responses.is_empty());
    }

    #[tokio::test]
    async fn test_error_cases() {
        let (handler, _) = create_test_handler().await;

        // Test non-connect first message
        let room_msg = IncomingMessage {
            message_type: MessageType::Room("Invalid first message".to_string())
        };
        let responses = handler.handle_message(room_msg, None).await;
        assert!(responses.iter().any(|msg| match msg {
            WsMessage::Text(text) => text.contains("First message must be Connect"),
            _ => false
        }));

        // Test message to non-existent room
        let random_id = Uuid::new_v4();
        let room_msg = IncomingMessage {
            message_type: MessageType::Room("No room exists".to_string())
        };
        let responses = handler.handle_message(room_msg, Some(random_id)).await;
        assert!(responses.is_empty()); // Error sent through room manager

        // Test private message to non-existent user
        let private_msg = IncomingMessage {
            message_type: MessageType::Private {
                recipient: "NonExistentUser".to_string(),
                content: "Won't be delivered".to_string()
            }
        };
        let responses = handler.handle_message(private_msg, Some(random_id)).await;
        assert!(responses.is_empty()); // Error sent through room manager
    }
}