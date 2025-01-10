use tokio_tungstenite::tungstenite::{Message as WsMessage, Utf8Bytes};
use shared::message_utils::{IncomingMessage, MessageType};

// This trait defines how to convert something into a WebSocket message
pub trait IntoWebSocketMessage {
    fn into_ws_message(self) -> Result<WsMessage, serde_json::Error>;
}

// Implement for MessageType enum directly
impl IntoWebSocketMessage for MessageType {
    fn into_ws_message(self) -> Result<WsMessage, serde_json::Error> {
        // Create an IncomingMessage with this MessageType
        let message = IncomingMessage {
            message_type: self,
        };

        // Serialize to JSON and convert to WebSocket message
        let json = serde_json::to_string(&message)?;
        Ok(WsMessage::Text(Utf8Bytes::from(json)))
    }
}

// Helper function to parse command strings into WebSocket messages
pub fn parse_command(input: &str) -> Result<WsMessage, Box<dyn std::error::Error>> {
    // If no command prefix, treat as a room message
    let input = input.trim();
    if !input.starts_with('/') {
        return MessageType::Room(input.to_string()).into_ws_message().map_err(Into::into);
    }

    // Split into command and content
    let mut parts = input[1..].splitn(2, ' ');
    let command = parts.next().unwrap_or("");
    let content = parts.next().unwrap_or("").trim();

    // Convert command into appropriate MessageType
    let message_type = match command {
        "connect" => {
            MessageType::new_connect(content.to_string())?
        },
        "room" => {
            MessageType::new_room(content.to_string())?
        },
        "private" | "pm" => {
            let mut parts = content.splitn(2, ' ');
            let recipient = parts.next().unwrap_or("").trim();
            let message = parts.next().unwrap_or("").trim();
            MessageType::new_private(recipient.to_string(), message.to_string())?
        },
        _ => return Err("Unknown command. Available commands: /connect, /room, /private".into())
    };

    // Convert the MessageType into a WebSocket message
    message_type.into_ws_message().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    // Helper function to extract JSON from WebSocket message
    fn extract_json(msg: WsMessage) -> Result<Value, Box<dyn std::error::Error>> {
        match msg {
            WsMessage::Text(text) => Ok(serde_json::from_str(text.as_ref())?),
            _ => Err("Expected text message".into()),
        }
    }

    #[test]
    fn test_room_message_default() {
        // Non-command messages should be treated as room messages
        let result = parse_command("Hello, room!").unwrap();
        let json = extract_json(result).unwrap();

        assert_eq!(json["type"], "Room");
        assert_eq!(json["payload"], "Hello, room!");
    }

    #[test]
    fn test_room_message_explicit() {
        let result = parse_command("/room Hello, room!").unwrap();
        let json = extract_json(result).unwrap();

        assert_eq!(json["type"], "Room");
        assert_eq!(json["payload"], "Hello, room!");
    }

    #[test]
    fn test_connect_message() {
        let result = parse_command("/connect Alice").unwrap();
        let json = extract_json(result).unwrap();

        assert_eq!(json["type"], "Connect");
        assert_eq!(json["payload"]["name"], "Alice");
    }

    #[test]
    fn test_private_message() {
        let result = parse_command("/private Bob secret message").unwrap();
        let json = extract_json(result).unwrap();

        assert_eq!(json["type"], "Private");
        assert_eq!(json["payload"]["recipient"], "Bob");
        assert_eq!(json["payload"]["content"], "secret message");

        // Test alternative command format
        let result = parse_command("/pm Charlie another message").unwrap();
        let json = extract_json(result).unwrap();

        assert_eq!(json["type"], "Private");
        assert_eq!(json["payload"]["recipient"], "Charlie");
        assert_eq!(json["payload"]["content"], "another message");
    }

    #[test]
    fn test_error_cases() {
        // Empty connect command
        let result = parse_command("/connect");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Name cannot be empty"
        );

        // Empty room message
        let result = parse_command("/room");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Content cannot be empty"
        );

        // Private message without recipient
        let result = parse_command("/private");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Recipient name cannot be empty"
        );

        // Private message without content
        let result = parse_command("/private Bob");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Content cannot be empty"
        );

        // Unknown command
        let result = parse_command("/unknown");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unknown command. Available commands: /connect, /room, /private"
        );
    }

    #[test]
    fn test_message_type_conversion() {
        // Test direct MessageType to WebSocket message conversion
        let room_type = MessageType::Room("test message".to_string());
        let result = room_type.into_ws_message().unwrap();
        let json = extract_json(result).unwrap();
        assert_eq!(json["type"], "Room");
        assert_eq!(json["payload"], "test message");

        let connect_type = MessageType::Connect { name: "David".to_string() };
        let result = connect_type.into_ws_message().unwrap();
        let json = extract_json(result).unwrap();
        assert_eq!(json["type"], "Connect");
        assert_eq!(json["payload"]["name"], "David");

        let private_type = MessageType::Private {
            recipient: "Eve".to_string(),
            content: "private test".to_string()
        };
        let result = private_type.into_ws_message().unwrap();
        let json = extract_json(result).unwrap();
        assert_eq!(json["type"], "Private");
        assert_eq!(json["payload"]["recipient"], "Eve");
        assert_eq!(json["payload"]["content"], "private test");

        let system_type = MessageType::System("system message".to_string());
        let result = system_type.into_ws_message().unwrap();
        let json = extract_json(result).unwrap();
        assert_eq!(json["type"], "System");
        assert_eq!(json["payload"], "system message");
    }

    #[test]
    fn test_whitespace_handling() {
        // Leading/trailing whitespace in command
        let result = parse_command("  /connect   Alice  ").unwrap();
        let json = extract_json(result).unwrap();
        assert_eq!(json["payload"]["name"], "Alice");

        // Multiple spaces between parts
        let result = parse_command("/private   Bob    secret   message").unwrap();
        let json = extract_json(result).unwrap();
        assert_eq!(json["payload"]["recipient"], "Bob");
        assert_eq!(json["payload"]["content"], "secret   message");

        // Preserve whitespace in room messages
        let result = parse_command("   Hello,    room!   ").unwrap();
        let json = extract_json(result).unwrap();
        assert_eq!(json["payload"], "Hello,    room!"); // Inner whitespace preserved, outer trimmed
    }
}