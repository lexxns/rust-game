use bevy::prelude::*;
use crate::game::game_event_structs::{CardComponent, EventResult, GameEvent, GameEventQueue, GameEventWithContext, GameStateComponent};
use crate::game::game_events;
use crate::room::room_components::{CurrentTurn, Players, TurnTimer};
use crate::types::Server;


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
    mut commands: Commands,
    mut card_query: Query<&mut CardComponent>
) {
    for (room_entity, players, mut current_turn, mut timer, mut game_state, mut event_queue) in rooms.iter_mut() {
        if !event_queue.current_events.is_empty() {
            println!("Processing events for room {:?}, events: {:?}", room_entity, event_queue.current_events.len());
        }
        let mut events_to_queue = Vec::new();

        if let Some(event) = event_queue.current_events.pop_front() {
            println!("Processing queued game event: {:?}", event);
            let context = event.context.clone();
            let result: EventResult = match event.event {
                GameEvent::StartGame {} => {
                    game_events::game_event_start_game(&mut game_state, players) }
                GameEvent::EndGame { player_id } => {
                    game_events::game_event_end_game()
                }
                GameEvent::StartTurn { player_id } => {
                    game_events::game_event_start_turn(&mut current_turn, players, player_id, &server)
                }
                GameEvent::EndTurn { player_id } => {
                    game_events::game_event_end_turn(players, &mut current_turn, player_id)
                }
                GameEvent::AddCardsToDeck { player_id, amount} => {
                    game_events::game_event_add_cards_to_decks(&mut commands, &server, &mut game_state, player_id, amount)
                }
                GameEvent::DrawCard { player_id, amount } => {
                    game_events::game_event_draw_card(&server, &card_query, &mut game_state, player_id, amount)
                }
                GameEvent::PlayCard { player_id, card_id, target } => {
                    game_events::game_event_play_card(players, player_id, card_id, &mut game_state)
                }
                GameEvent::GameStateChange { new_state } => {
                    game_events::game_event_game_state_change(&server, players, &mut game_state, new_state)
                }
                GameEvent::SpecialAction { player_id, action_type, targets } => {
                    game_events::game_event_special_action(&server, players, &player_id, &action_type, &targets)
                }
            };
            if result.reset_timer {
                timer.timer.reset();
            }
            // Convert the result events into GameEventWithContext using the original context
            events_to_queue.extend(
                result.next_events.into_iter().map(|event| GameEventWithContext {
                    context: context.clone(),
                    event,
                })
            );
        }

        // Queue up all the new events
        event_queue.next_events.extend(events_to_queue);


        // Only swap queues if current_events is empty
        if event_queue.current_events.is_empty() {
            event_queue.swap_queues();
        }
    }
}
