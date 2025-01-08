mod room;

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use room::RoomManager;

async fn handle_command(msg: &str) -> Option<Message> {
    match msg.trim() {
        "/help" => Some(Message::text(
            "Available commands:\n\
             /help - Show this help message\n\
             /time - Show current time\n\
             /users - Show number of connected users"
        )),
        "/time" => {
            let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            Some(Message::text(format!("Current server time: {}", time)))
        }
        msg if msg.starts_with('/') => {
            Some(Message::text("Unknown command. Type /help for available commands."))
        }
        _ => None
    }
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8000";
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    let (broadcast_tx, _) = broadcast::channel::<Message>(100);
    let broadcast_tx = Arc::new(broadcast_tx);

    println!("WebSocket server listening on: {}", addr);

    let room_manager = RoomManager::new();

    while let Ok((stream, addr)) = listener.accept().await {
        let room_manager = room_manager.clone();
        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    println!("Client connected: {}", addr);
                    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                    // Create a channel for sending messages to the WebSocket
                    let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(100);
                    let msg_tx = Arc::new(msg_tx);

                    // Add player to room management
                    let mut room_rx = {
                        let mut manager = room_manager.write().await;
                        manager.add_player(addr, msg_tx.clone()).await
                    };
                    if room_rx.is_none() {
                        let manager = room_manager.read().await;
                        if let Some(broadcaster) = manager.get_room_broadcast(addr) {
                            room_rx = Some(broadcaster.subscribe());
                        }
                    }

                    // Task for sending messages to WebSocket
                    let sender_task = tokio::spawn(async move {
                        while let Some(message) = msg_rx.recv().await {
                            if ws_sender.send(message).await.is_err() {
                                break;
                            }
                        }
                    });


                    // Task for handling room messages
                    let msg_tx_clone = msg_tx.clone();
                    let room_task = room_rx.map(|mut rx| {
                        tokio::spawn(async move {
                            while let Ok(msg) = rx.recv().await {
                                if msg_tx_clone.send(msg).await.is_err() {
                                    break;
                                }
                            }
                        })
                    });

                    // Handle incoming messages
                    while let Some(Ok(msg)) = ws_receiver.next().await {
                        match msg {
                            Message::Text(text) => {
                                println!("Received from {}: {}", addr, text);

                                // Handle commands
                                if let Some(response) = handle_command(&text).await {
                                    if msg_tx.send(response).await.is_err() {
                                        break;
                                    }
                                } else {
                                    // Send message to room if player is in one
                                    let manager = room_manager.read().await;
                                    if !manager.broadcast_to_room(addr, text.parse().unwrap()).await {
                                        println!("Failed to broadcast message for {}", addr);
                                    }
                                }
                            }
                            Message::Close(_) => {
                                println!("Client disconnected: {}", addr);
                                break;
                            }
                            _ => {}
                        }
                    }

                    // Clean up
                    sender_task.abort();
                    if let Some(task) = room_task {
                        task.abort();
                    }

                    // Remove player from room management
                    {
                        let mut manager = room_manager.write().await;
                        manager.remove_player(addr);
                    }

                    println!("Connection closed: {}", addr);
                }
                Err(e) => println!("Error during WebSocket handshake: {}", e),
            }
        });
    }
}