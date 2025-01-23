use serde::{Deserialize, Serialize};
use std::error::Error;

// Represents all possible message types in our system
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum MessageType {
    Room {
        #[serde(skip_serializing_if = "Option::is_none")]
        sender: Option<String>,
        content: String,
    },
    Private {
        #[serde(skip_serializing_if = "Option::is_none")]
        sender: Option<String>,
        recipient: String,
        content: String
    },
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

    pub fn new_private(sender: Option<String>, recipient: String, content: String) -> Result<Self, Box<dyn Error>> {
        if recipient.is_empty() {
            return Err("Recipient name cannot be empty".into());
        }
        if content.is_empty() {
            return Err("Content cannot be empty".into());
        }
        Ok(MessageType::Private { sender, recipient, content })
    }

    pub fn new_system(content: String) -> Result<Self, Box<dyn Error>> {
        if content.is_empty() {
            return Err("Content cannot be empty".into());
        }
        Ok(MessageType::System(content))
    }

    pub fn new_room(sender: Option<String>, content: String) -> Result<Self, Box<dyn Error>> {
        if content.is_empty() {
            return Err("Content cannot be empty".into());
        }
        Ok(MessageType::Room {sender, content})
    }
}