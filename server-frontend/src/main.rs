use futures_util::{SinkExt, StreamExt};
use std::io::{self, Write};
use tokio::io::AsyncBufReadExt;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[tokio::main]
async fn main() {
    let url = "ws://127.0.0.1:8000";
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket connected");

    let (mut write, mut read) = ws_stream.split();

    let receive_task = tokio::spawn(async move {
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => {
                    println!("Received: {}", msg);
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    break;
                }
            }
        }
    });

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();

    println!("Enter messages (press Ctrl+C to exit):");
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        line.clear();
        if let Ok(n) = stdin.read_line(&mut line).await {
            if n == 0 {
                break;
            }

            let message = line.trim();
            if message.is_empty() {
                continue;
            }

            // Using the correct Message type
            if let Err(e) = write.send(Message::text(message)).await {
                eprintln!("Error sending message: {}", e);
                break;
            }
        }
    }

    receive_task.abort();
}