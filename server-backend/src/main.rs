use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::tungstenite::{Message as WsMessage, Utf8Bytes};
use std::sync::Arc;
mod room;
mod messages;

use room::{RoomManager, Player};
use messages::{handle_incoming_message};
use shared::message_utils::{parse_incoming_message, CommsMessage, MessageType, PlayerMessage};

async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    state: Arc<RwLock<RoomManager>>,
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // First message must be a Connect message with the player's name
    let connect_msg = match ws_receiver.next().await {
        Some(Ok(msg)) => match parse_incoming_message(msg) {
            Ok(incoming) => match incoming.message_type {
                MessageType::Connect { name } => Ok(name),
                _ => Err("First message must be a Connect message with your name")
            },
            Err(e) => Err("Failed to parse connect message: {}")
        },
        Some(Err(e)) => Err("WebSocket error"),
        None => Err("Connection closed before receiving connect message")
    };

    // Handle connection errors by sending error message and returning
    let player_name = match connect_msg {
        Ok(name) => name,
        Err(e) => {
            let error_msg = WsMessage::Text(Utf8Bytes::from(e));
            let _ = ws_sender.send(error_msg).await;
            return;
        }
    };

    // Set up message channel for this player
    let (message_tx, mut message_rx) = mpsc::unbounded_channel();

    // Create and register the new player
    let player = Player::new(player_name, message_tx.clone());
    let player_id = player.id;

    // Add player to waiting list
    {
        let mut room_manager = state.write().await;
        room_manager.add_waiting_player(player);

        // Try to create a room if possible
        if let Some((player1_id, player2_id)) = room_manager.try_create_room() {
            PlayerMessage::player_matched(player1_id, player2_id).send(&room_manager.connections())
        }
    }

    // Spawn task to forward messages from other players to this client
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = message_rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Main message handling loop
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(msg) => {
                let room_manager = state.read().await;
                match parse_incoming_message(msg) {
                    Ok(incoming_msg) => {
                        handle_incoming_message(incoming_msg, player_id, &room_manager);
                    },
                    Err(e) => {
                        PlayerMessage::system("Unable to Parse Message", player_id).send(&room_manager.connections())
                    }
                }
            }
            Err(_) => break,
        }
    }

    // Clean up when the connection ends
    forward_task.abort();
    let mut state = state.write().await;
    state.handle_disconnect(&player_id);
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