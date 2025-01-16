mod message_generator;

use futures_util::{SinkExt, StreamExt};
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use crate::message_generator::{parse_command, IntoWebSocketMessage};
use shared::message_utils::{display_text, MessageType};
use clap::Parser;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://127.0.0.1:8080";
    let args = Args::parse();
    let (ws_stream, _) = connect_async(url).await?;
    println!("WebSocket connected");
    println!("Commands: /room <msg>, /private <name> <msg>");

    let (mut write, mut read) = ws_stream.split();

    // Track our username and last sent message
    let my_username = std::sync::Arc::new(std::sync::Mutex::new(String::new()));

    // Store last sent message type and recipient for displaying our own messages
    let last_sent = std::sync::Arc::new(std::sync::Mutex::new(None::<(String, Option<String>)>));
    let last_sent_clone = last_sent.clone();

    *my_username.lock().unwrap() = args.name;
    if let Ok(connect_msg) = MessageType::new_connect(String::from(my_username.lock().unwrap().clone())) {
        write.send(connect_msg.into_ws_message().unwrap()).await?;
    } else {
        panic!("Failed to connect to the server - Invalid Username");
    }


    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    // Check if this is our own message
                    let username = my_username.lock().unwrap().clone();
                    let sender_name = value.get("sender_name")
                        .and_then(|s| s.as_str())
                        .unwrap_or("");

                    let is_own_message = !username.is_empty() && sender_name == username;

                    match value.get("type").and_then(|t| t.as_str()) {
                        Some("Room") => {
                            if let Some(content) = value.get("payload").and_then(|p| p.as_str()) {
                                let c = content;
                                display_text!("Room", c);
                            }
                        }
                        Some("Private") => {
                            if let Some(content) = value.get("payload").and_then(|p| p.get("content")).and_then(|c| c.as_str()) {
                                if is_own_message {
                                    // Get recipient from last sent message
                                    if let Some((_, Some(recipient))) = last_sent_clone.lock().unwrap().as_ref() {
                                        display_text!("Private", content, true, recipient);
                                    } else {
                                        display_text!("Private", content, true);
                                    }
                                } else {
                                    display_text!("Private", content);
                                }
                            }
                        }
                        Some("System") => {
                            if let Some(content) = value.get("payload").and_then(|p| p.as_str()) {
                                display_text!("System", content);
                            }
                        }
                        _ => {
                            println!("Unknown message: {}", text);
                        }
                    }
                } else {
                    display_text!("System", text);
                }
            }
        }
    });

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();

    loop {
        print!("> ");
        io::stdout().flush()?;
        line.clear();

        if stdin.read_line(&mut line).await? == 0 {
            break;
        }

        // Parse command and update last sent message info
        if line.starts_with("/private") || line.starts_with("/pm") {
            if let Some(content) = line.strip_prefix(if line.starts_with("/pm") { "/pm" } else { "/private" }) {
                if let Some((recipient, _)) = content.trim().split_once(' ') {
                    *last_sent.lock().unwrap() = Some(("Private".to_string(), Some(recipient.to_string())));
                }
            }
        } else if !line.starts_with('/') {
            *last_sent.lock().unwrap() = Some(("Room".to_string(), None));
        }

        match parse_command(&line) {
            Ok(message) => {
                write.send(message).await?;
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    receive_task.abort();
    Ok(())
}