use std::collections::HashSet;
use tokio_tungstenite::tungstenite::{Message as WsMessage, Utf8Bytes};
use uuid::Uuid;
use shared::message_utils::{IncomingMessage, MessageType, PlayerConnection};
use crate::room::RoomManager;
use serde::de::Error as SerdeError;

pub trait CommsMessage {
    fn text(&self) -> &Utf8Bytes;
    fn from(&self) -> Uuid;
    fn targets(&self) -> &HashSet<Uuid>;
    fn send(&self, senders: &std::collections::HashMap<Uuid, PlayerConnection>) {
        // Create the message with the same format as IncomingMessage
        let message_type = if self.from() == Uuid::nil() {
            MessageType::System(self.text().to_string())
        } else {
            MessageType::Room(self.text().to_string())
        };

        let message = IncomingMessage {
            message_type,
        };

        // Serialize and send
        if let Ok(json) = serde_json::to_string(&message) {
            let ws_message = WsMessage::Text(Utf8Bytes::from(json));
            for target in self.targets() {
                if let Some(sender) = senders.get(target) {
                    let _ = sender.send(ws_message.clone());
                }
            }
        }
    }
}

pub struct PlayerMessage {
    content: Utf8Bytes,
    originator: Uuid,
    originator_name: Option<String>,
    target_players: HashSet<Uuid>,
}

impl PlayerMessage {
    pub fn new(content: impl Into<Utf8Bytes>, from: Uuid, from_name: Option<String>, to: impl Into<HashSet<Uuid>>) -> Self {
        Self {
            content: content.into(),
            originator: from,
            originator_name: from_name,
            target_players: to.into(),
        }
    }

    pub fn system(content: impl Into<Utf8Bytes>, to: Uuid) -> Self {
        let mut targets = HashSet::new();
        targets.insert(to);
        Self {
            content: content.into(),
            originator: Uuid::nil(),
            originator_name: None,
            target_players: targets,
        }
    }

    pub fn room_broadcast(content: impl Into<Utf8Bytes>, from: Uuid, from_name: String, room_members: impl Into<HashSet<Uuid>>) -> Self {
        let formatted_content = format!("{}: {}", from_name, content.into());
        Self {
            content: Utf8Bytes::from(formatted_content),
            originator: from,
            originator_name: Some(from_name),
            target_players: room_members.into(),
        }
    }

    pub fn private(content: impl Into<Utf8Bytes>, from: Uuid, from_name: String, to: Uuid) -> Self {
        let formatted_content = format!("{}: {}", from_name, content.into());
        let mut targets = HashSet::new();
        targets.insert(to);
        Self {
            content: Utf8Bytes::from(formatted_content),
            originator: from,
            originator_name: Some(from_name),
            target_players: targets,
        }
    }

    pub fn player_matched(player1: Uuid, player2: Uuid) -> Self {
        let mut targets = HashSet::new();
        targets.insert(player1);
        targets.insert(player2);
        Self {
            content: Utf8Bytes::from("Matched with player!"),
            originator: Uuid::nil(),
            originator_name: None,
            target_players: targets,
        }
    }

    pub fn player_disconnected(to: Uuid) -> Self {
        let mut targets = HashSet::new();
        targets.insert(to);
        Self {
            content: Utf8Bytes::from("Your partner has disconnected"),
            originator: Uuid::nil(),
            originator_name: None,
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

    fn send(&self, senders: &std::collections::HashMap<Uuid, PlayerConnection>) {
        // Create the message with the same format as IncomingMessage
        let message_type = if self.originator == Uuid::nil() {
            MessageType::System(self.content.clone().parse().unwrap())
        } else {
            MessageType::Room(self.content.clone().parse().unwrap())
        };

        let message = IncomingMessage {
            message_type,
        };

        // Serialize and send
        if let Ok(json) = serde_json::to_string(&message) {
            let ws_message = WsMessage::Text(Utf8Bytes::from(json));
            for target in self.targets() {
                if let Some(sender) = senders.get(target) {
                    let _ = sender.send(ws_message.clone());
                }
            }
        }
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

pub fn handle_incoming_message(
    msg: IncomingMessage,
    sender_id: Uuid,
    room_manager: &RoomManager,
) {
    match msg.message_type {
        MessageType::Room(content) => {
            if let Some(members) = room_manager.get_room_info(&sender_id).map(|(_, other_id)| {
                let mut members = HashSet::new();
                members.insert(other_id);
                members
            }) {
                if let Some(sender_name) = room_manager.get_player_name(&sender_id) {
                    Box::new(PlayerMessage::room_broadcast(content, sender_id, sender_name, members)).send(&room_manager.player_connections())
                }
            } else {
                Box::new(PlayerMessage::system("You are not in a room", sender_id)).send(&room_manager.player_connections())
            }
        },
        MessageType::Private { recipient, content } => {
            println!("Got a private message for {:?} with content {:?}", recipient, content);

            if let Some(recipient_id) = room_manager.get_player_id(recipient) {
                if let Some(sender_name) = room_manager.get_player_name(&sender_id) {
                    Box::new(PlayerMessage::private(content, sender_id, sender_name, recipient_id)).send(&room_manager.player_connections())
                }
            }
        },
        MessageType::System(content) => {
            Box::new(PlayerMessage::system(content, sender_id)).send(&room_manager.player_connections())
        },
        MessageType::Connect { .. } => {
            Box::new(PlayerMessage::system("Connected successfully", sender_id)).send(&room_manager.player_connections())
        }
        _ => {
            Box::new(PlayerMessage::system("Unknown Message Type", sender_id)).send(&room_manager.player_connections())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::mpsc;
    use serde_json::json;

    // Helper function to create a mock player connection
    fn create_mock_connection() -> (PlayerConnection, mpsc::UnboundedReceiver<WsMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (PlayerConnection::from(tx), rx)
    }

    #[test]
    fn test_room_message_parsing() {
        let room_msg = json!({
            "type": "Room",
            "payload": "Hello room!"
        });
        println!("Sending JSON: {}", room_msg.to_string());
        let ws_message = WsMessage::Text(Utf8Bytes::from(room_msg.to_string()));
        let result = parse_incoming_message(ws_message);
        println!("Parse result: {:?}", result);

        match result {
            Ok(parsed) => {
                println!("Successfully parsed message: {:?}", parsed);
                match &parsed.message_type {
                    MessageType::Room(content) => {
                        println!("Got Room message with content: {}", content);
                        assert_eq!(content, "Hello room!");
                    },
                    MessageType::Private { .. } => panic!("Got Private when expecting Room"),
                    MessageType::System(_) => panic!("Got System when expecting Room"),
                    MessageType::Connect { .. } => panic!("Got Connect when expecting Room"),
                    _ => panic!("Got unexpected message type"),
                }
            },
            Err(e) => panic!("Failed to parse message: {:?}", e)
        }
    }

    #[test]
    fn test_private_message_parsing() {
        let private_msg = json!({
            "type": "Private",
            "payload": {
                "recipient": "user123",
                "content": "Private message"
            }
        });
        let ws_message = WsMessage::Text(Utf8Bytes::from(private_msg.to_string()));
        let result = parse_incoming_message(ws_message);

        match result {
            Ok(parsed) => {
                match &parsed.message_type {
                    MessageType::Private { recipient, content } => {
                        assert_eq!(recipient, "user123");
                        assert_eq!(content, "Private message");
                    },
                    other => panic!("Expected MessageType::Private but got: {:?}", other),
                }
            },
            Err(e) => panic!("Failed to parse private message: {:?}", e)
        }
    }

    #[test]
    fn test_connect_message_parsing() {
        let connect_msg = json!({
            "type": "Connect",
            "payload": {
                "name": "TestUser"
            }
        });
        let ws_message = WsMessage::Text(Utf8Bytes::from(connect_msg.to_string()));
        let result = parse_incoming_message(ws_message);

        match result {
            Ok(parsed) => {
                match &parsed.message_type {
                    MessageType::Connect { name } => {
                        assert_eq!(name, "TestUser");
                    },
                    other => panic!("Expected MessageType::Connect but got: {:?}", other),
                }
            },
            Err(e) => panic!("Failed to parse connect message: {:?}", e)
        }
    }

    #[test]
    fn test_system_message_parsing() {
        let system_msg = json!({
            "type": "System",
            "payload": "System notification"
        });
        let ws_message = WsMessage::Text(Utf8Bytes::from(system_msg.to_string()));
        let result = parse_incoming_message(ws_message);

        match result {
            Ok(parsed) => {
                match &parsed.message_type {
                    MessageType::System(content) => {
                        assert_eq!(content, "System notification");
                    },
                    other => panic!("Expected MessageType::System but got: {:?}", other),
                }
            },
            Err(e) => panic!("Failed to parse system message: {:?}", e)
        }
    }

    #[test]
    fn test_error_cases() {
        // Missing type field
        let missing_type = json!({
            "payload": "Hello"
        });
        let ws_message = WsMessage::Text(Utf8Bytes::from(missing_type.to_string()));
        assert!(parse_incoming_message(ws_message).is_err(), "Should fail when type field is missing");

        // Missing payload
        let missing_payload = json!({
            "type": "Room"
        });
        let ws_message = WsMessage::Text(Utf8Bytes::from(missing_payload.to_string()));
        assert!(parse_incoming_message(ws_message).is_err(), "Should fail when payload field is missing");

        // Invalid message type
        let invalid_type = json!({
            "type": "InvalidType",
            "payload": "Hello"
        });
        let ws_message = WsMessage::Text(Utf8Bytes::from(invalid_type.to_string()));
        assert!(parse_incoming_message(ws_message).is_err(), "Should fail with invalid type");

        // Invalid JSON
        let ws_message = WsMessage::Text(Utf8Bytes::from("invalid json"));
        assert!(parse_incoming_message(ws_message).is_err(), "Should fail with invalid JSON");

        // Empty message
        let ws_message = WsMessage::Text(Utf8Bytes::from(""));
        assert!(parse_incoming_message(ws_message).is_err(), "Should fail with empty message");
    }

    #[test]
    fn test_player_message_creation() {
        let player1_id = Uuid::new_v4();
        let player2_id = Uuid::new_v4();
        let content = "Test message";

        // Test basic message creation
        let msg = PlayerMessage::new(content, player1_id, Some("anyone".into()), {
            let mut set = HashSet::new();
            set.insert(player2_id);
            set
        });

        assert_eq!(*msg.text(), content);
        assert_eq!(msg.from(), player1_id);
        assert!(msg.targets().contains(&player2_id));
        assert_eq!(msg.targets().len(), 1);
    }

    #[tokio::test]
    async fn test_message_sending() {
        let sender_id = Uuid::new_v4();
        let recipient_id = Uuid::new_v4();
        let content = "Test message";

        let mut senders = HashMap::new();
        let (sender1_conn, mut receiver1) = create_mock_connection();
        let (sender2_conn, mut receiver2) = create_mock_connection();

        senders.insert(sender_id, sender1_conn);
        senders.insert(recipient_id, sender2_conn);

        let msg = PlayerMessage::private(content, sender_id, "anyone".into(), recipient_id);
        msg.send(&senders);

        // Check that the message was received by the intended recipient
        if let Some(received) = receiver2.try_recv().ok() {
            match received {
                WsMessage::Text(text) => assert_eq!(text, content),
                _ => panic!("Unexpected message type"),
            }
        } else {
            panic!("No message received");
        }

        // Verify sender didn't receive the message
        assert!(receiver1.try_recv().is_err());
    }
}