use bevy::prelude::*;
use std::collections::HashSet;
use std::time::Duration;
use crate::game::game_event_structs::{GameEvent, GameEventContext, GameEventQueue, GameEventWithContext, GameStateComponent};
use crate::room::room_components::{CurrentTurn, Players, Room, RoomState, TurnTimer};

#[derive(Resource)]
pub struct RoomManager {
    next_room_id: usize,
}

impl Default for RoomManager {
    fn default() -> Self {
        Self { next_room_id: 0 }
    }
}

impl RoomManager {
    pub fn find_or_create_room(
        &mut self,
        commands: &mut Commands,
        player_id: u128,
        rooms: &mut Query<(Entity, &mut Players, &mut GameStateComponent)>,
        event_queue: &mut EventWriter<GameEventWithContext>
    ) -> Entity {
        // Try to find existing room with space
        for (entity, mut players, _game_state) in rooms.iter_mut() {
            if players.set.len() < 2 {
                players.set.insert(player_id);

                // If we now have exactly 2 players, start the game
                if players.set.len() == 2 {
                    event_queue.send(GameEventWithContext {
                        context: GameEventContext {
                            room_entity: entity,
                        },
                        event: GameEvent::StartGame {},
                    });
                }
                return entity;
            }
        }

        // Create new room
        let room_id = format!("room_{}", self.next_room_id);
        self.next_room_id += 1;

        let room_entity = commands
            .spawn((
                Room { room_id },
                Players { set: HashSet::from([player_id]) },
                CurrentTurn { player: None },
                TurnTimer {
                    timer: Timer::new(Duration::from_secs(30), TimerMode::Once)
                },
                RoomState {
                    is_active: true,
                    last_update: 0.0,
                },
                GameStateComponent::default(),
                GameEventQueue::default()
            ))
            .id();

        room_entity
    }
}