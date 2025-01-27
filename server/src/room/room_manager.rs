use bevy::prelude::*;
use std::collections::HashSet;
use std::time::Duration;
use crate::game::game_events::{GameState, GameStateComponent};
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
        rooms: &mut Query<(Entity, &mut Players)>,
    ) -> Entity {
        // Try to find existing room with space
        for (entity, mut players) in rooms.iter_mut() {
            if players.set.len() < 2 {
                players.set.insert(player_id);
                return entity;
            }
        }

        // Create new room
        let room_id = format!("room_{}", self.next_room_id);
        self.next_room_id += 1;

        commands
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
                GameStateComponent {
                    state: GameState::Starting,
                    deck_size: 30,
                    discard_pile: Vec::new(),
                },
            ))
            .id()
    }
}