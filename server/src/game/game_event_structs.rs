use std::collections::{HashMap, HashSet, VecDeque};
use bevy::prelude::{Component, Entity, Event};
use shared::channel::{Card, GameMessage};
use shared::EntityID;

// Context that every game event must have
#[derive(Debug, Clone)]
pub struct GameEventContext {
    pub room_entity: Entity,
}

// Define all possible game events with context
#[derive(Debug, Clone, Event)]
pub struct GameEventWithContext {
    pub context: GameEventContext,
    pub event: GameEvent,
}

// Define all possible game events
#[derive(Debug, Clone, Event)]
pub enum GameEvent {
    StartGame {},
    AddCardsToDeck {
        player_id: EntityID,
        amount: u32,
    },
    DrawCard {
        player_id: EntityID,
        amount: u32,
    },
    PlayCard {
        player_id: EntityID,
        card_id: EntityID,
        target: Option<EntityID>,  // Optional target
    },
    EndTurn {
        player_id: EntityID, // Player ending their turn
    },
    StartTurn {
        player_id: EntityID,
    },
    SpecialAction {
        player_id: EntityID,
        action_type: SpecialActionType,
        targets: Vec<EntityID>,
    },
    GameStateChange {
        new_state: GameState,
    },
    EndGame {
        player_id: EntityID,
    }
}

#[derive(Debug)]
pub struct MessageContext {
    pub client_id: EntityID,
    pub room_entity: Entity,
}

pub trait IntoGameEvent {
    fn into_game_event(self, context: &MessageContext) -> Option<GameEventWithContext>;
}

impl IntoGameEvent for GameMessage {
    fn into_game_event(self, context: &MessageContext) -> Option<GameEventWithContext> {
        let event = match self {
            GameMessage::EndTurn => Some(GameEvent::EndTurn {
                player_id: context.client_id
            }),
            GameMessage::DrawCard(amount) => Some(GameEvent::DrawCard {
                player_id: context.client_id,
                amount,
            }),
            GameMessage::PlayCard { card_id, target } => Some(GameEvent::PlayCard {
                player_id: context.client_id,
                card_id,
                target,
            }),
            // Messages that don't convert to game events return None
            _ => None
        };

        event.map(|e| GameEventWithContext {
            context: GameEventContext {
                room_entity: context.room_entity,
            },
            event: e,
        })
    }
}

#[derive(Debug, Clone)]
pub enum SpecialActionType {
    DiscardCard,
    SwapCards,
    BlockAction,
}

#[derive(Debug, Clone)]
pub enum GameState {
    Starting,
    InProgress,
    Finished(Option<EntityID>), // Optional winner ID
}

// Component to track game state
#[derive(Component)]
pub struct GameStateComponent {
    pub state: GameState,
    pub player_decks: HashMap<EntityID, DeckComponent>,
    pub player_hands: HashMap<EntityID, HandComponent>,
    pub discard_pile: Vec<EntityID>,
}

#[derive(Component)]
pub struct DeckComponent {
    pub player_id: EntityID,
    pub cards: Vec<Entity>
}

#[derive(Component)]
pub struct HandComponent {
    pub player_id: EntityID,
    pub cards: Vec<Entity>,
}

impl DeckComponent {
    pub fn new(player_id: EntityID) -> Self {
        Self { cards: Vec::new(), player_id }
    }
}

impl HandComponent {
    pub(crate) fn default(player_id: EntityID) -> HandComponent {
        HandComponent {
            player_id,
            cards: Vec::new()
        }
    }
}

// Card component
#[derive(Component)]
pub struct CardComponent(Card);

impl CardComponent {

    pub fn new(card: Card) -> CardComponent {
        CardComponent(card)
    }

    pub(crate) fn as_card(&self) -> Card {
        self.0.clone()
    }

    // using a separate ID to the Entity ID of bevy
    pub(crate) fn get_id(&self) -> EntityID {
        self.0.card_id
    }

    pub(crate) fn get_name(&self) -> String {
        self.0.card_name.clone()
    }

    pub(crate) fn get_text(&self) -> String {
        self.0.card_text.clone()
    }
}

impl Default for GameStateComponent {
    fn default() -> Self {
        Self {
            state: GameState::Starting,
            player_decks: HashMap::new(),
            player_hands: HashMap::new(),
            discard_pile: Vec::new(),
        }
    }
}

#[derive(Component)]
pub struct GameEventQueue {
    pub(crate) current_events: VecDeque<GameEventWithContext>,
    pub(crate) next_events: VecDeque<GameEventWithContext>,
}

#[derive(Default)]
pub struct EventResult {
    pub next_events: Vec<GameEvent>,
    pub reset_timer: bool,
}

impl Default for GameEventQueue {
    fn default() -> Self {
        Self {
            current_events: VecDeque::new(),
            next_events: VecDeque::new(),
        }
    }
}

impl GameEventQueue {
    pub fn swap_queues(&mut self) {
        // Take ownership of next_events temporarily
        let next = std::mem::take(&mut self.next_events);
        // Move current_events into next_events
        self.next_events = std::mem::take(&mut self.current_events);
        // Move the old next_events into current_events
        self.current_events = next;
    }
}