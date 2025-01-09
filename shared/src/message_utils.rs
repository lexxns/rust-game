use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Message as WsMessage};
use serde::{Deserialize, Serialize};

// Type alias for a player's WebSocket connection
pub type PlayerConnection = mpsc::UnboundedSender<WsMessage>;

// Represents all possible message types in our system
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum MessageType {
    Room(String),
    Private { recipient: String, content: String },
    System(String),
    Connect { name: String },
}

// The structured format of messages coming from clients
#[derive(Debug, Serialize, Deserialize)]
pub struct IncomingMessage {
    #[serde(flatten)]
    pub message_type: MessageType,
}

