use serde::{Deserialize, Serialize};
use crate::EntityID;

#[derive(Serialize, Deserialize, Clone, Debug)]
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GameMessage {
    // Game state updates (server -> client)
    CurrentTurn(Option<EntityID>),     // Who's turn is it
    CardsDrawn(u32),                   // How many cards were drawn
    CardPlayed(EntityID, EntityID),    // Who played what card
    CardDiscarded(EntityID, EntityID), // Who discarded what card
    GameOver(Option<EntityID>),        // Game ended, optional winner

    // Player actions (client -> server)
    EndTurn,                           // Player wants to end their turn
    DrawCard(u32),                     // Player wants to draw N cards
    PlayCard {
        card_id: EntityID,
        target: Option<EntityID>,      // Optional target for card effects
    },

    // Chat functionality (bidirectional)
    Chat(MessageType),                 // Chat messages work both ways

    // Game setup and management
    JoinGame,                          // Player wants to join a game
    LeaveGame,                         // Player wants to leave

    // Error handling
    Error(String),                     // Generic error message
}

#[derive(Debug, Clone)]
pub struct GameChannel;
impl bevy_simplenet::ChannelPack for GameChannel
{
    type ConnectMsg = ();
    type ServerMsg = GameMessage;
    type ServerResponse = ();
    type ClientMsg = ();
    type ClientRequest = GameMessage;
}