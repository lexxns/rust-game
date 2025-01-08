use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{accept_async, tungstenite::Message};

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

    while let Ok((stream, addr)) = listener.accept().await {
        let broadcast_tx = broadcast_tx.clone();
        let mut broadcast_rx = broadcast_tx.subscribe();

        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    println!("Client connected: {}", addr);
                    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                    // Create a channel for sending messages to the WebSocket
                    let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(100);
                    let msg_tx = Arc::new(msg_tx);

                    // Send welcome message
                    let msg_tx_clone = msg_tx.clone();
                    msg_tx_clone.send(Message::text("Welcome! Type /help for available commands."))
                        .await
                        .expect("Failed to send welcome message");

                    // Task for sending messages to WebSocket
                    let sender_task = tokio::spawn(async move {
                        while let Some(message) = msg_rx.recv().await {
                            if ws_sender.send(message).await.is_err() {
                                break;
                            }
                        }
                    });

                    // Task for handling broadcast messages
                    let msg_tx_clone = msg_tx.clone();
                    let broadcast_task = tokio::spawn(async move {
                        while let Ok(msg) = broadcast_rx.recv().await {
                            if msg_tx_clone.send(msg).await.is_err() {
                                break;
                            }
                        }
                    });

                    // Handle incoming messages from the WebSocket
                    while let Some(Ok(msg)) = ws_receiver.next().await {
                        match msg {
                            Message::Text(text) => {
                                println!("Received from {}: {}", addr, text);

                                // Handle commands
                                if let Some(response) = handle_command(&text).await {
                                    // Send command response only to this client
                                    if msg_tx.send(response).await.is_err() {
                                        break;
                                    }
                                } else {
                                    // Broadcast regular messages to all clients
                                    let broadcast_msg = Message::text(format!("{}: {}", addr, text));
                                    if broadcast_tx.send(broadcast_msg).is_err() {
                                        break;
                                    }
                                }
                            }
                            Message::Close(_) => {
                                println!("Client disconnected: {}", addr);
                                break;
                            }
                            _ => {} // Ignore other message types
                        }
                    }

                    // Clean up tasks
                    sender_task.abort();
                    broadcast_task.abort();
                    println!("Connection closed: {}", addr);
                }
                Err(e) => println!("Error during WebSocket handshake: {}", e),
            }
        });
    }
}