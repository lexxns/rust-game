use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use std::time::Duration;
use std::collections::HashMap;
use bevy::ecs::system::SystemParam;
use crate::game::GameState;
use crate::game::systems::send_room_state;
use crate::types::{Server, PlayerTurns};

// Constant for turn duration
const TURN_DURATION: Duration = Duration::from_secs(30);

// Component to track turn timer for each room
#[derive(Component)]
pub struct TurnTimer {
    pub room_id: String,
    pub timer: Timer,
}

// Resource to track active timers
#[derive(Resource, Default)]
pub struct ActiveTurnTimers(pub HashMap<String, Entity>);

// System to spawn a new timer when a turn starts
pub fn spawn_turn_timer(
    mut commands: Commands,
    player_turns: ReactRes<PlayerTurns>,
    mut active_timers: ResMut<ActiveTurnTimers>,
) {
    // Check each room for new turns starting
    for (room_id, &current_player) in &player_turns.0 {
        // Only process rooms with active turns
        if let Some(current_player) = current_player {
            // If there's no timer for this room yet, spawn one
            if !active_timers.0.contains_key(room_id) {
                let timer_entity = commands
                    .spawn(TurnTimer {
                        room_id: room_id.to_string(),
                        timer: Timer::new(TURN_DURATION, TimerMode::Once),
                    })
                    .id();

                active_timers.0.insert(room_id.to_string(), timer_entity);

                tracing::info!(
                    "Started turn timer for room {} (player {})",
                    room_id, current_player
                );
            }
        }
    }
}

// System to update timers and force turn end when time runs out
pub fn update_turn_timers(
    mut commands: Commands,
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
    server: ResMut<Server>,
    mut player_turns: ReactResMut<PlayerTurns>,
    mut timers: Query<(Entity, &mut TurnTimer)>,
    mut active_timers: ResMut<ActiveTurnTimers>,
) {
    let mut completed_timers = Vec::new();

    // Update all active timers
    for (entity, mut timer) in timers.iter_mut() {
        timer.timer.tick(time.delta());

        // Check if timer finished
        if timer.timer.finished() {
            let room_id = &timer.room_id;

            // Force end the current turn
            if let Some(room) = game_state.rooms.get_mut(room_id) {
                if let Some(current_player) = room.current_turn {
                    if let Some(next_player) = room.switch_turn() {
                        // Update room state
                        player_turns
                            .get_mut(&mut commands)
                            .0
                            .insert(room_id.to_string(), Some(next_player));

                        send_room_state(&server, &game_state, room_id);

                        tracing::info!(
                            "Turn timer expired - Force ending turn for player {} in room {}",
                            current_player, room_id
                        );
                    }
                }
            }

            // Mark timer for removal
            completed_timers.push((entity, room_id.to_string()));
        }
    }

    // Clean up completed timers
    for (entity, room_id) in completed_timers {
        commands.entity(entity).despawn();
        active_timers.0.remove(&room_id);
    }
}

// Function to cancel timer when player ends turn manually
pub fn cancel_turn_timer(
    mut commands: Commands,
    room_id: &str,
    active_timers: &mut ActiveTurnTimers,
) {
    if let Some(&timer_entity) = active_timers.0.get(room_id) {
        commands.entity(timer_entity).despawn();
        active_timers.0.remove(room_id);
        tracing::info!("Cancelled turn timer for room {}", room_id);
    }
}


#[derive(SystemParam)]
pub struct TimerCancellation<'w, 's> {
    commands: Commands<'w, 's>,
    active_timers: ResMut<'w, ActiveTurnTimers>,
}

impl<'w, 's> TimerCancellation<'w, 's> {
    pub fn cancel_timer(&mut self, room_id: &str) {
        if let Some(&timer_entity) = self.active_timers.0.get(room_id) {
            self.commands.entity(timer_entity).despawn();
            self.active_timers.0.remove(room_id);
            tracing::info!("Cancelled turn timer for room {}", room_id);
        }
    }
}