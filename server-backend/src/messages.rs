use std::collections::HashSet;
use uuid::Uuid;
use shared::message_utils::{CommsMessage, IncomingMessage, MessageType, PlayerMessage};
use crate::room::RoomManager;

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
                Box::new(PlayerMessage::room_broadcast(content, sender_id, members)).send(&room_manager.connections())
            } else {
                Box::new(PlayerMessage::system("You are not in a room", sender_id)).send(&room_manager.connections())
            }
        },
        MessageType::Private { recipient, content } => {
            Box::new(PlayerMessage::private(content, sender_id, recipient)).send(&room_manager.connections())
        },
        MessageType::System(content) => {
            Box::new(PlayerMessage::system(content, sender_id)).send(&room_manager.connections())
        },
        MessageType::Connect { .. } => {
            Box::new(PlayerMessage::system("Connected successfully", sender_id)).send(&room_manager.connections())
        }
    }
}
