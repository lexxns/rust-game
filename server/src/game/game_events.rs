use std::fmt::format;
use bevy::prelude::{Commands, Entity, Mut, Query, Res};
use bevy::reflect::Set;
use tracing::warn;
use shared::card_details::build_default_deck;
use shared::channel::{Card, GameMessage};
use shared::EntityID;
use crate::game::game_event_structs::{CardComponent, DeckComponent, EventResult, GameEvent, GameState, GameStateComponent, HandComponent, SpecialActionType};
use crate::room::room_components::{CurrentTurn, Players};
use crate::types::Server;

pub fn game_event_start_game(game_state: &mut GameStateComponent, players: &Players) -> EventResult {
    // Verify we have exactly 2 players
    assert_eq!(players.set.len(), 2, "Must have exactly 2 players to initialize game");
    let mut result = EventResult::default();

    // Initialize decks and hands for both players
    for &player_id in players.set.iter() {
        result.next_events.push(GameEvent::AddCardsToDeck { player_id, amount:30});
        result.next_events.push(GameEvent::DrawCard { player_id, amount:5});
    }

    game_state.state = GameState::InProgress;

    result
}

pub fn game_event_end_game() -> EventResult {
    // TODO
    EventResult::default()
}

pub fn game_event_add_cards_to_decks(mut commands: &mut Commands, server: &Res<Server>, game_state: &mut GameStateComponent, player_id: EntityID, amount: u32) -> EventResult {
    let deck = game_state.player_decks.entry(player_id).or_insert_with(|| DeckComponent::new(player_id));
    let mut new_card_entities: Vec< Entity> = Vec::with_capacity(amount as usize); // Store Entity IDs

    // Get the default deck configuration
    let deck_cards = build_default_deck();

    // Create entities for each card
    for (card_id, card_name, card_text) in deck_cards {
        let new_card = CardComponent::new(
            Card {
                card_id,
                card_name,
                card_text,
            }
        );
        let entity = commands.spawn(new_card).id();
        new_card_entities.push(entity);
    }

    // Shuffle the new cards using rand
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    new_card_entities.shuffle(&mut rng);

    // Add shuffled cards to deck
    deck.cards.append(&mut new_card_entities);

    server.send(player_id, GameMessage::CardsInDeck(deck.cards.len() as u32));
    EventResult::default()
}

pub fn game_event_special_action(server: &Res<Server>, players: &Players, player_id: &EntityID, action_type: &SpecialActionType, targets: &Vec<EntityID>) -> EventResult {
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

pub fn game_event_game_state_change(server: &Res<Server>, players: &Players, game_state: &mut GameStateComponent, new_state: GameState) -> EventResult {
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

pub fn game_event_play_card(players: &Players, player_id: EntityID, card_id: EntityID, mut game_state: &mut GameStateComponent) -> EventResult {
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
    println!("Switching turn to player: {:?}", player_id);
    let mut result = EventResult {
        reset_timer: true,
        next_events: Vec::new(),
    };
    if let turn_player = Some(player_id) {
        current_turn.player = turn_player;
        // Notify all players
        for &player_id in &players.set {
            server.send(player_id, GameMessage::CurrentTurn(turn_player));
        }
        result.next_events.push(GameEvent::DrawCard { player_id: turn_player.unwrap(), amount: 1 });
    }
    result
}

pub fn game_event_end_turn(players: &Players, current_turn: &mut Mut<CurrentTurn>, player_id: EntityID) -> EventResult {
    let mut result = EventResult::default();
    if players.set.contains(&player_id) && current_turn.player == Some(player_id) {
        if let Some(&next_player) = players.set.iter()
            .find(|&&p| p != player_id) {
            result.next_events.push(GameEvent::StartTurn { player_id: next_player });
        }
    }
    result
}

pub fn game_event_draw_card(server: &Res<Server>, query: &Query<&mut CardComponent>, game_state: &mut Mut<GameStateComponent>, player_id: EntityID, amount: u32) -> EventResult {
    if let Some(deck) = game_state.player_decks.get_mut(&player_id) {
        if deck.cards.len() >= amount as usize {
            let mut drawn_card_entities = deck.cards.drain(..amount as usize).collect::<Vec<_>>();

            let mut drawn_cards: Vec<Card> = Vec::with_capacity(drawn_card_entities.len());
            for entity in &drawn_card_entities {
                if let Ok(card_component) = query.get(*entity) { // Use the passed-in query
                    drawn_cards.push(card_component.as_card());
                } else {
                    warn!("Entity {:?} does not have a CardComponent", entity);
                }
            }

            let hand = game_state.player_hands.entry(player_id)
                .or_insert(HandComponent::default(player_id));

            hand.cards.append(&mut drawn_card_entities);

            server.send(player_id, GameMessage::CardsDrawn(drawn_cards));
        }
    }
    EventResult::default()
}