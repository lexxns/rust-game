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
            if content.is_empty() {
                return Err("Name cannot be empty. Usage: /connect <name>".into());
            }
            MessageType::Connect {
                name: content.to_string()
            }
        },
        "room" => {
            if content.is_empty() {
                return Err("Message cannot be empty. Usage: /room <message>".into());
            }
            MessageType::Room(content.to_string())
        },
        "private" | "pm" => {
            let mut parts = content.splitn(2, ' ');
            let recipient = parts.next()
                .ok_or("Missing recipient name. Usage: /private <name> <message>")?;
            let message = parts.next()
                .ok_or("Missing message. Usage: /private <name> <message>")?;

            MessageType::Private {
                recipient: recipient.to_string(),
                content: message.to_string()
            }
        },
        _ => return Err("Unknown command. Available commands: /connect, /room, /private".into())
    };

    // Convert the MessageType into a WebSocket message
    message_type.into_ws_message().map_err(Into::into)
}