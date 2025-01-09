use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Message as WsMessage, Utf8Bytes};
use uuid::Uuid;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use serde::ser::Error;

// Type alias for a player's WebSocket connection
pub type PlayerConnection = mpsc::UnboundedSender<WsMessage>;

// Represents all possible message types in our system
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum MessageType {
    Room(String),
    Private { recipient: Uuid, content: String },
    System(String),
    Connect { name: String },
}

// The structured format of messages coming from clients
#[derive(Debug, Serialize, Deserialize)]
pub struct IncomingMessage {
    #[serde(flatten)]
    pub message_type: MessageType,
}

pub trait CommsMessage {
    fn text(&self) -> &Utf8Bytes;
    fn from(&self) -> Uuid;
    fn targets(&self) -> &HashSet<Uuid>;
    fn send(&self, senders: &std::collections::HashMap<Uuid, PlayerConnection>) {
        let message = WsMessage::Text(self.text().clone());
        for target in self.targets() {
            if let Some(sender) = senders.get(target) {
                let _ = sender.send(message.clone());
            }
        }
    }
}

pub struct PlayerMessage {
    content: Utf8Bytes,
    originator: Uuid,
    target_players: HashSet<Uuid>,
}

impl PlayerMessage {
    pub fn new(content: impl Into<Utf8Bytes>, from: Uuid, to: impl Into<HashSet<Uuid>>) -> Self {
        Self {
            content: content.into(),
            originator: from,
            target_players: to.into(),
        }
    }

    // Factory methods for specific message types
    pub fn system(content: impl Into<Utf8Bytes>, to: Uuid) -> Self {
        let mut targets = HashSet::new();
        targets.insert(to);
        Self {
            content: content.into(),
            originator: Uuid::nil(),
            target_players: targets,
        }
    }

    pub fn room_broadcast(content: impl Into<Utf8Bytes>, from: Uuid, room_members: impl Into<HashSet<Uuid>>) -> Self {
        Self {
            content: content.into(),
            originator: from,
            target_players: room_members.into(),
        }
    }

    pub fn private(content: impl Into<Utf8Bytes>, from: Uuid, to: Uuid) -> Self {
        let mut targets = HashSet::new();
        targets.insert(to);
        Self {
            content: content.into(),
            originator: from,
            target_players: targets,
        }
    }

    pub fn player_matched(player1: Uuid, player2: Uuid) -> Self {
        let mut targets = HashSet::new();
        targets.insert(player1);
        targets.insert(player2);
        Self {
            content: Utf8Bytes::from("Matched with player!"),
            originator: Uuid::nil(), // System message
            target_players: targets,
        }
    }

    pub fn player_disconnected(to: Uuid) -> Self {
        let mut targets = HashSet::new();
        targets.insert(to);
        Self {
            content: Utf8Bytes::from("Your partner has disconnected"),
            originator: Uuid::nil(), // System message
            target_players: targets,
        }
    }
}

impl CommsMessage for PlayerMessage {
    fn text(&self) -> &Utf8Bytes {
        &self.content
    }

    fn from(&self) -> Uuid {
        self.originator
    }

    fn targets(&self) -> &HashSet<Uuid> {
        &self.target_players
    }
}

pub struct PrivateMessage {
    content: Utf8Bytes,
    originator: Uuid,
    recipient: Uuid,
}

impl PrivateMessage {
    pub fn new(content: impl Into<Utf8Bytes>, from: Uuid, to: Uuid) -> Self {
        Self {
            content: content.into(),
            originator: from,
            recipient: to,
        }
    }
}

impl CommsMessage for PrivateMessage {
    fn text(&self) -> &Utf8Bytes {
        &self.content
    }

    fn from(&self) -> Uuid {
        self.originator
    }

    fn targets(&self) -> &HashSet<Uuid> {
        static mut SINGLE_TARGET: Option<HashSet<Uuid>> = None;
        // SAFETY: This is safe because we're only using it for a single thread
        // and immediately returning a reference that won't outlive the function call
        unsafe {
            if SINGLE_TARGET.is_none() {
                SINGLE_TARGET = Some(HashSet::new());
            }
            let set = SINGLE_TARGET.as_mut().unwrap();
            set.clear();
            set.insert(self.recipient);
            SINGLE_TARGET.as_ref().unwrap()
        }
    }
}

// Message handling utilities
pub fn parse_incoming_message(msg: WsMessage) -> Result<IncomingMessage, serde_json::Error> {
    match msg {
        WsMessage::Text(content) => {
            serde_json::from_str(content.as_ref())
        },
        _ => Err(serde_json::Error::custom("Unsupported message type")),
    }
}