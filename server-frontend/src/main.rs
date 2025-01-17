mod message_generator;

use futures_util::{SinkExt, StreamExt};
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use crate::message_generator::{parse_command, IntoWebSocketMessage};
use shared::message_utils::{display_text, IncomingMessage, MessageType};
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

                // Try parsing the incoming message
                if let Ok(message) = serde_json::from_str::<IncomingMessage>(&text) {
                    // TODO: use message id / sender id to figure this out
                    let is_own_message = match &message.message_type {
                        MessageType::Room { sender, .. } |
                        MessageType::Private { sender, .. } => {
                            if let Some(sender_name) = sender {  // Handle optional sender
                                let username = my_username.lock().unwrap().clone();
                                !username.is_empty() && sender_name == &username
                            } else {
                                false
                            }
                        }
                        _ => false  // System messages and Connect messages aren't "owned"
                    };

                    match message.message_type {
                        MessageType::Room { sender, content } => {
                            if is_own_message {
                                display_text!("Room", &content, true)
                            } else {
                                let s = sender.unwrap();
                                let c = format!("{s}: {content}");
                                display_text!("Room", &c)
                            };
                        }
                        MessageType::Private { sender, recipient, content } => {
                            if is_own_message {
                                display_text!("Private", &content, true, &recipient)
                            } else {
                                let s = sender.unwrap();
                                let c = format!("{s}: {content}");
                                display_text!("Private", &c)
                            }
                        }
                        MessageType::System(content) => {
                            display_text!("System", &content)
                        }
                        MessageType::Connect { name: _ } => {
                            // Don't need to do anything here
                        }
                    }
                } else {
                    println!("Failed to parse message: {}", text);
                }            }
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