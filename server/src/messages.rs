use std::collections::HashSet;
use bevy_simplenet::ChannelPack;
use tokio_tungstenite::tungstenite::{Message as WsMessage, Utf8Bytes};
use uuid::Uuid;
use shared::message_utils::{IncomingMessage, MessageType, PlayerConnection};
use crate::room::RoomManager;
use serde::de::Error as SerdeError;
use serde::{Deserialize, Serialize};

/// Clients send these when connecting to the server.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestConnectMsg(pub String);

/// Clients can send these at any time.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestClientMsg(pub u64);

/// Client requests are special messages that expect a response from the server.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestClientRequest(pub u64);

/// Servers can send these at any time.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestServerMsg(pub u64);

/// Servers send these in response to client requests.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TestServerResponse(pub u64);

#[derive(Debug, Clone)]
pub struct TestChannel;
impl ChannelPack for TestChannel
{
    type ConnectMsg = TestConnectMsg;
    type ServerMsg = TestServerMsg;
    type ServerResponse = TestServerResponse;
    type ClientMsg = TestClientMsg;
    type ClientRequest = TestClientRequest;
}

pub trait CommsMessage {
    fn text(&self) -> &Utf8Bytes;
    fn from(&self) -> Uuid;
    fn targets(&self) -> &HashSet<Uuid>;
    fn into_message_type(&self) -> MessageType;  // New method

    fn send(&self, senders: &std::collections::HashMap<Uuid, PlayerConnection>) {
        let message = IncomingMessage {
            message_type: self.into_message_type(),
        };

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

// Room-specific message
pub struct RoomMessage {
    pub content: Utf8Bytes,
    pub originator: Uuid,
    pub originator_name: String,
    pub target_players: HashSet<Uuid>,
}

impl CommsMessage for RoomMessage {
    fn text(&self) -> &Utf8Bytes { &self.content }
    fn from(&self) -> Uuid { self.originator }
    fn targets(&self) -> &HashSet<Uuid> { &self.target_players }
    fn into_message_type(&self) -> MessageType {
        MessageType::Room {
            sender: Option::from(self.originator_name.clone()),
            content: self.content.clone().parse().unwrap()
        }
    }
}

// Private message implementation
pub struct PrivateMessage {
    pub content: Utf8Bytes,
    pub originator: Uuid,
    pub originator_name: String,
    pub recipient: Uuid,
    pub recipient_name: String,
}

impl CommsMessage for PrivateMessage {
    fn text(&self) -> &Utf8Bytes { &self.content }
    fn from(&self) -> Uuid { self.originator }
    fn targets(&self) -> &HashSet<Uuid> {
        static mut SINGLE_TARGET: Option<HashSet<Uuid>> = None;
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
    fn into_message_type(&self) -> MessageType {
        MessageType::Private {
            sender: Option::from(self.originator_name.clone()),
            recipient: self.recipient_name.clone(),
            content: self.content.clone().parse().unwrap()
        }
    }
}

// System message implementation
pub struct SystemMessage {
    pub content: Utf8Bytes,
    pub target: Uuid,
}

impl CommsMessage for SystemMessage {
    fn text(&self) -> &Utf8Bytes { &self.content }
    fn from(&self) -> Uuid { Uuid::nil() }
    fn targets(&self) -> &HashSet<Uuid> {
        static mut SINGLE_TARGET: Option<HashSet<Uuid>> = None;
        unsafe {
            if SINGLE_TARGET.is_none() {
                SINGLE_TARGET = Some(HashSet::new());
            }
            let set = SINGLE_TARGET.as_mut().unwrap();
            set.clear();
            set.insert(self.target);
            SINGLE_TARGET.as_ref().unwrap()
        }
    }
    fn into_message_type(&self) -> MessageType {
        MessageType::System(self.content.clone().parse().unwrap())
    }
}

// Add this with your other message type structs
pub struct SystemBroadcastMessage {
    pub content: Utf8Bytes,
    pub targets: HashSet<Uuid>,
}

// Implement CommsMessage for it
impl CommsMessage for SystemBroadcastMessage {
    fn text(&self) -> &Utf8Bytes { &self.content }
    fn from(&self) -> Uuid { Uuid::nil() }
    fn targets(&self) -> &HashSet<Uuid> { &self.targets }
    fn into_message_type(&self) -> MessageType {
        MessageType::System(self.content.clone().parse().unwrap())
    }
}

#[macro_export]
macro_rules! systemmsg {
    ($content:expr, $target:expr) => {
        crate::messages::SystemMessage {
            content: $content.into(),
            target: $target,
        }
    };
}

#[macro_export]
macro_rules! systemmsg_multi {
    ($content:expr, $($target:expr),+) => {{
        let mut targets = HashSet::new();
        $(
            targets.insert($target);
        )+
        crate::messages::SystemBroadcastMessage {
            content: $content.into(),
            targets: targets,
        }
    }};
}

#[macro_export]
macro_rules! roommsg {
    ($content:expr, $from:expr, $from_name:expr, $target:expr) => {{
        let mut targets = std::collections::HashSet::new();
        targets.insert($from);  // Add sender
        targets.insert($target);  // Add recipient
        crate::messages::RoomMessage {
            content: $content.into(),
            originator: $from,
            originator_name: $from_name,
            target_players: targets
        }
    }};

    // Pattern for multiple individual targets
    ($content:expr, $from:expr, $from_name:expr, $($target:expr),+) => {{
        let mut targets = std::collections::HashSet::new();
        targets.insert($from);  // Add sender
        $(
            targets.insert($target);  // Add each recipient
        )+
        crate::messages::RoomMessage {
            content: $content.into(),
            originator: $from,
            originator_name: $from_name,
            target_players: targets
        }
    }};
}

#[macro_export]
macro_rules! privatemsg {
    ($content:expr, $from:expr, $from_name:expr, $to:expr, $to_name:expr) => {
        crate::messages::PrivateMessage {
            content: $content.into(),
            originator: $from,
            originator_name: $from_name,
            recipient: $to,
            recipient_name: $to_name,
        }
    };
}

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
        MessageType::Room{content, ..} => {
            if let (Some((_, other_id)), Some(sender_name)) = (
                room_manager.get_room_info(&sender_id),
                room_manager.get_player_name(&sender_id)
            ) {
                roommsg!(content, sender_id, sender_name, other_id).send(room_manager.player_connections());
            }
            else {
                systemmsg!("You are not in a room", sender_id)
                    .send(room_manager.player_connections());
            }
        },
        MessageType::Private { sender, recipient, content } => {
            let recopy = recipient.clone();
            if let Some(recipient_id) = room_manager.get_player_id(recipient) {
                let sender_name = sender.or_else(|| room_manager.get_player_name(&sender_id))
                    .unwrap_or_else(|| "SERVER ERROR".to_string());

                privatemsg!(content, sender_id, sender_name, recipient_id, recopy)
                    .send(room_manager.player_connections());
            } else {
                systemmsg!("Player is not online", sender_id)
                    .send(room_manager.player_connections());
            }
        },
        MessageType::System(content) => {
            systemmsg!(content, sender_id).send(room_manager.player_connections());
        },
        MessageType::Connect { name: _ } => {
            systemmsg!("Connected Successfully", sender_id).send(room_manager.player_connections());
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
                    MessageType::Room{content, .. } => {
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
        let msg = PrivateMessage {
            content: content.into(),
            originator: player1_id,
            originator_name: "player1".to_string(),
            recipient: player2_id,
            recipient_name: "player2".to_string(),
        };

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

        // TODO: get a room-manager stub working for these tests
        // let msg = PlayerTargetMessage::private(content, sender_id, "anyone".into(), recipient_id);
        // msg.send(&senders);

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