use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Message as WsMessage};
use serde::{Deserialize, Serialize};
use std::error::Error;

// Type alias for a player's WebSocket connection
pub type PlayerConnection = mpsc::UnboundedSender<WsMessage>;
pub type Receiver = mpsc::UnboundedReceiver<WsMessage>;

// Represents all possible message types in our system
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum MessageType {
    Room(String),
    Private { recipient: String, content: String },
    System(String),
    Connect { name: String }
}

// The structured format of messages coming from clients
#[derive(Debug, Serialize, Deserialize)]
pub struct IncomingMessage {
    #[serde(flatten)]
    pub message_type: MessageType,
}

impl MessageType {
    pub fn new_connect(name: String) -> Result<Self, Box<dyn Error>> {
        if name.is_empty() {
            return Err("Name cannot be empty".into());
        }
        Ok(MessageType::Connect { name })
    }

    pub fn new_private(recipient: String, content: String) -> Result<Self, Box<dyn Error>> {
        if recipient.is_empty() {
            return Err("Recipient name cannot be empty".into());
        }
        if content.is_empty() {
            return Err("Content cannot be empty".into());
        }
        Ok(MessageType::Private { recipient, content })
    }

    pub fn new_system(content: String) -> Result<Self, Box<dyn Error>> {
        if content.is_empty() {
            return Err("Content cannot be empty".into());
        }
        Ok(MessageType::System(content))
    }

    pub fn new_room(content: String) -> Result<Self, Box<dyn Error>> {
        if content.is_empty() {
            return Err("Content cannot be empty".into());
        }
        Ok(MessageType::Room(content))
    }
}

// Print functions
pub fn print_white(text: &str) {
    println!("\x1b[37m{}\x1b[0m", text);
}

pub fn print_blue(text: &str) {
    println!("\x1b[94m{}\x1b[0m", text);
}

pub fn print_yellow(text: &str) {
    println!("\x1b[93m{}\x1b[0m", text);
}

pub fn print_pink(text: &str) {
    println!("\x1b[95m{}\x1b[0m", text);
}

pub fn print_private_with_recipient(recipient: &str, text: &str) {
    println!("\x1b[37m({}) {}\x1b[0m", recipient, text);
}

#[macro_export]
macro_rules! display_text {
    // For own messages without recipient
    ($msg_type:expr, $content:expr, true) => {
        $crate::message_utils::print_white(&$content.to_string())
    };

    // For own private messages with recipient
    ($msg_type:expr, $content:expr, true, $recipient:expr) => {
        if $msg_type == "Private" {
            $crate::message_utils::print_private_with_recipient($recipient, &$content.to_string())
        } else {
            $crate::message_utils::print_white(&$content.to_string())
        }
    };

    // For received messages (original colors)
    ($msg_type:expr, $content:expr) => {
        match $msg_type {
            "System" => $crate::message_utils::print_blue(&$content.to_string()),
            "Room" => $crate::message_utils::print_yellow(&$content.to_string()),
            "Private" => $crate::message_utils::print_pink(&$content.to_string()),
            _ => println!("{}", $content)
        }
    };
}

pub use display_text;