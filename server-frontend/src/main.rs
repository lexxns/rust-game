mod message_generator;

use futures_util::{SinkExt, StreamExt};
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use std::str::FromStr;
use crate::message_generator::parse_command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "ws://127.0.0.1:8080";
    let (ws_stream, _) = connect_async(url).await?;
    println!("WebSocket connected");
    println!("Commands: /connect <name>, /room <msg>, /private <name> <msg>");
    println!("Connect first before anything else");

    let (mut write, mut read) = ws_stream.split();

    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    match value.get("type").and_then(|t| t.as_str()) {
                        Some("Room") | Some("Private") | Some("System") => {
                            if let Some(content) = value.get("payload").and_then(|p| p.as_str()) {
                                println!("{}", content);
                            }
                        }
                        _ => {
                            println!("Unknown message: {}", text);
                        }
                    }
                } else {
                    println!("\x1b[94m{}\x1b[0m", text); // Light blue for system messages
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