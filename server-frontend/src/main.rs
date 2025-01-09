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
    println!("Just type to send to current room");

    let (mut write, mut read) = ws_stream.split();

    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            if let WsMessage::Text(text) = msg {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                    println!("Received: {}", serde_json::to_string_pretty(&value).unwrap());
                } else {
                    println!("Received: {}", text);
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