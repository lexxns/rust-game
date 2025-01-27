use bevy::prelude::*;
use shared::channel::{GameMessage};
use shared::EntityID;
use crate::room::room_components::{CurrentTurn, NextTurn, Players, TurnTimer};
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
    pub deck_size: u32,
    pub discard_pile: Vec<EntityID>,
}

// System to handle game events
pub fn handle_game_events(
    mut commands: Commands,
    mut game_events: EventReader<GameEvent>,
    mut rooms: Query<(
        Entity,
        &Players,
        &mut CurrentTurn,
        &mut TurnTimer,
        &mut GameStateComponent
    )>,
    server: Res<Server>,
) {
    for event in game_events.read() {
        println!("Handling game event: {:?}", event);
        match event {
            GameEvent::DrawCard { player_id, amount } => {
                // Handle drawing cards
                for (_, players, _, _, mut game_state) in rooms.iter_mut() {
                    if players.set.contains(player_id) {
                        // Check if there are enough cards in the deck
                        if game_state.deck_size >= *amount {
                            game_state.deck_size -= amount;
                            // Notify player of new cards
                            server.send(*player_id, GameMessage::CardsDrawn(*amount));
                        }
                    }
                }
            }
            GameEvent::PlayCard { player_id, card_id, target } => {
                // Handle playing a card
                for (_, players, _, _, mut game_state) in rooms.iter_mut() {
                    if players.set.contains(player_id) {
                        // Add card to discard pile
                        game_state.discard_pile.push(*card_id);
                        // Notify all players in the room
                        for &p in &players.set {
                            server.send(p, GameMessage::CardPlayed(*player_id, *card_id));
                        }
                    }
                }
            }
            GameEvent::EndTurn { player_id } => {
                println!("Handling end turn request for : {:?}", player_id);

                // Debug: Check how many rooms are found
                let room_count = rooms.iter().count();
                println!("Number of rooms found: {}", room_count);
                // Handle end of turn
                for (room_entity, players, current_turn, mut timer, _) in rooms.iter_mut() {
                    println!("Checking room: {:?}", room_entity);
                    println!("Players in room: {:?}", players.set);
                    println!("Current turn holder: {:?}", current_turn.player);
                    if players.set.contains(player_id) {
                        println!("Player {:?} is in the room.", player_id);
                        if current_turn.player == Some(*player_id) {
                            println!("Player {:?} is the current turn holder.", player_id);
                            let entity = commands.spawn(NextTurn).id();
                            println!("NextTurn entity spawned with ID: {:?}", entity);
                            timer.timer.reset();
                        } else {
                            println!("Player {:?} is NOT the current turn holder.", player_id);
                        }
                    } else {
                        println!("Player {:?} is NOT in the room.", player_id);
                    }
                }
            }
            GameEvent::SpecialAction { player_id, action_type, targets } => {
                // Handle special actions
                for (_, players, _, _, _) in rooms.iter() {
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
                }
            }
            GameEvent::GameStateChange { new_state } => {
                // Handle game state changes
                for (_, players, _, _, mut game_state) in rooms.iter_mut() {
                    game_state.state = new_state.clone();
                    // Notify all players of the state change
                    if let GameState::Finished(winner) = new_state {
                        for &p in &players.set {
                            server.send(p, GameMessage::GameOver(*winner));
                        }
                    }
                }
            }
        }
    }
}


pub fn handle_next_turn(
    mut commands: Commands,
    next_turn_query: Query<Entity, With<NextTurn>>,
    mut rooms: Query<(
        Entity,
        &Players,
        &mut CurrentTurn,
        &mut TurnTimer
    )>,
    server: Res<Server>,
) {
    for next_turn_entity in next_turn_query.iter() {
        println!("Processing next turn");
        // Process turn change
        for (_, players, mut current_turn, mut timer) in rooms.iter_mut() {
            if let Some(current_player) = current_turn.player {
                if let Some(&next_player) = players.set.iter()
                    .find(|&&p| p != current_player) {
                    // Update current player
                    current_turn.player = Some(next_player);
                    timer.timer.reset();

                    println!("Switching turn to player: {:?}", next_player);
                    // Notify all players
                    for &player_id in &players.set {
                        server.send(player_id, GameMessage::CurrentTurn(Some(next_player)));
                    }
                }
            }
        }
        // Remove the NextTurn entity after processing
        commands.entity(next_turn_entity).despawn();
    }
}