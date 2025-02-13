use std::collections::{HashMap, HashSet, VecDeque};
use bevy::prelude::*;
use shared::channel::{GameMessage};
use shared::EntityID;
use crate::room::room_components::{CurrentTurn, Players, TurnTimer};
use crate::types::{Server};

// Define all possible game events
#[derive(Debug, Clone, Event)]
pub enum GameEvent {
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
}

#[derive(Debug)]
pub struct MessageContext {
    pub client_id: EntityID,
    pub room_entity: Option<Entity>,
}

pub trait IntoGameEvent {
    fn into_game_event(self, context: &MessageContext) -> Option<GameEvent>;
}

impl IntoGameEvent for GameMessage {
    fn into_game_event(self, context: &MessageContext) -> Option<GameEvent> {
        match self {
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
        }
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
    pub player_decks: HashMap<EntityID, u32>,
    pub player_hands: HashMap<EntityID, u32>,
    pub discard_pile: Vec<EntityID>,
}

impl GameStateComponent {
    pub fn initialize_for_two_players(&mut self, players: &HashSet<EntityID>) {
        // Verify we have exactly 2 players
        assert_eq!(players.len(), 2, "Must have exactly 2 players to initialize game");

        // Initialize decks and hands for both players
        for &player_id in players {
            self.player_decks.insert(player_id, 30);  // 30 card deck
            self.player_hands.insert(player_id, 5);   // 5 card initial hand
        }

        self.state = GameState::InProgress;
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
    current_events: VecDeque<GameEvent>,
    pub(crate) next_events: VecDeque<GameEvent>,
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


pub fn process_game_events(
    mut rooms: Query<(
        Entity,
        &Players,
        &mut CurrentTurn,
        &mut TurnTimer,
        &mut GameStateComponent,
        &mut GameEventQueue
    )>,
    server: Res<Server>,
) {
    for (_room_entity, players, mut current_turn, mut timer, mut game_state, mut event_queue) in rooms.iter_mut() {
        let mut events_to_queue = Vec::new();

        while let Some(event) = event_queue.current_events.pop_front() {
            let result: EventResult = match event {
                GameEvent::StartTurn { player_id } => {
                    game_event_start_turn(&mut current_turn, players, player_id, &server)
                }
                GameEvent::EndTurn { player_id } => {
                    game_event_end_turn(players, &mut current_turn, player_id)
                }
                GameEvent::DrawCard { player_id, amount } => {
                    game_event_draw_card(&server, players, &mut game_state, player_id, amount)
                }
                GameEvent::PlayCard { player_id, card_id, target } => {
                    game_event_play_card(players, player_id, card_id, &mut game_state)
                }
                GameEvent::GameStateChange { new_state } => {
                    game_event_game_state_change(&server, players, &mut game_state, new_state)
                }
                GameEvent::SpecialAction { player_id, action_type, targets } => {
                    game_event_special_action(&server, players, &player_id, &action_type, &targets)
                }
            };
            if result.reset_timer {
                timer.timer.reset();
            }
            events_to_queue.extend(result.next_events);
        }

        // Queue up all the new events
        event_queue.next_events.extend(events_to_queue);

        // Swap queues at the end of processing
        event_queue.swap_queues();
    }
}

fn game_event_special_action(server: &Res<Server>, players: &Players, player_id: &EntityID, action_type: &SpecialActionType, targets: &Vec<EntityID>) -> EventResult {
    // Handle special actions
    if players.set.contains(player_id) {
        match action_type {
            SpecialActionType::DiscardCard => {
                // Handle discard action
                for &target in targets {
                    // Notify players of discarded cards
                    for &p in &players.set {
                        server.send(p, GameMessage::CardDiscarded(*player_id, target));
                    }
                }
            }
            // Handle other special actions
            _ => {}
        }
    }
    EventResult::default()
}

fn game_event_game_state_change(server: &Res<Server>, players: &Players, game_state: &mut GameStateComponent, new_state: GameState) -> EventResult {
    // Handle game state changes
    game_state.state = new_state.clone();
    // Notify all players of the state change
    if let GameState::Finished(winner) = new_state {
        for &p in &players.set {
            server.send(p, GameMessage::GameOver(winner));
        }
    }
    EventResult::default()
}

fn game_event_play_card(players: &Players, player_id: EntityID, card_id: EntityID, mut game_state: &mut GameStateComponent) -> EventResult {
    // Handle playing a card
    if players.set.contains(&player_id) {
        // Add card to discard pile
        game_state.discard_pile.push(card_id);
        // Notify all players in the room
        for &p in &players.set {
            // TODO
            // server.send(p, GameMessage::CardPlayed(*player_id, *card_id));
        }
    }
    EventResult::default()
}


pub fn game_event_start_turn(
    current_turn: &mut CurrentTurn,
    players: &Players,
    player_id: EntityID,
    server: &Res<Server>,
) -> EventResult {
    current_turn.player = Some(player_id);
    println!("Switching turn to player: {:?}", player_id);
    // Notify all players
    for &player_id in &players.set {
        server.send(player_id, GameMessage::CurrentTurn(Some(player_id)));
    }
    EventResult {
        reset_timer: true,
        next_events: Vec::new(),
    }
}

fn game_event_end_turn(players: &Players, current_turn: &mut Mut<CurrentTurn>, player_id: EntityID) -> EventResult {
    let mut result = EventResult::default();
    if players.set.contains(&player_id) && current_turn.player == Some(player_id) {
        if let Some(&next_player) = players.set.iter()
            .find(|&&p| p != player_id) {
            result.next_events.push(GameEvent::StartTurn { player_id: next_player });
        }
    }
    result
}

fn game_event_draw_card(server: &Res<Server>, players: &Players, game_state: &mut Mut<GameStateComponent>, player_id: EntityID, amount: u32) -> EventResult {
    if players.set.contains(&player_id) {
        if let Some(deck_size) = game_state.player_decks.get_mut(&player_id) {
            if *deck_size >= amount {
                *deck_size -= amount;
                *game_state.player_hands.get_mut(&player_id).unwrap() += amount;
                server.send(player_id, GameMessage::CardsDrawn(amount));
            }
        }
    }
    EventResult::default()
}

