use futures::{SinkExt, StreamExt};
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use std::sync::Arc;
use uuid::Uuid;
use tokio::sync::mpsc;

mod room;
use room::{RoomManager, Player};

async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    state: Arc<RwLock<RoomManager>>,
) {
    let player_id = Uuid::new_v4();
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (message_tx, mut message_rx) = mpsc::unbounded_channel();

    // Add player to waiting list
    {
        let mut state = state.write().await;
        state.add_waiting_player(Player {
            id: player_id,
            sender: message_tx.clone(),
        });

        // Try to create a room if possible
        if let Some((player1_id, player2_id)) = state.try_create_room() {
            // Notify both players they've been matched
            let match_msg = Message::Text(Utf8Bytes::from("Matched with player!"));
            if let Some(sender1) = state.get_player_sender(&player1_id) {
                let _ = sender1.send(match_msg.clone());
            }
            if let Some(sender2) = state.get_player_sender(&player2_id) {
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
                if let Some((_, other_player_id)) = state.get_room_info(&player_id) {
                    if let Some(other_sender) = state.get_player_sender(&other_player_id) {
                        let _ = other_sender.send(msg);
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Clean up when player disconnects
    {
        let mut state = state.write().await;
        if let Some(other_player_id) = state.handle_disconnect(&player_id) {
            if let Some(other_sender) = state.get_player_sender(&other_player_id) {
                let _ = other_sender.send(Message::Text(Utf8Bytes::from("Your partner has disconnected")));
            }
        }
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