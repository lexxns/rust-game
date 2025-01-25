use std::collections::HashSet;
use rand::Rng;

#[derive(Default)]
pub struct Room {
    pub(crate) players: HashSet<u128>,
    pub(crate) current_turn: Option<u128>,
}

impl Room {
    pub fn get_opponent(&self, player_id: u128) -> Option<u128> {
        self.players.iter()
            .find(|&&p| p != player_id)
            .copied()
    }

    pub fn is_ready_to_start(&self) -> bool {
        self.players.len() == 2 && self.current_turn.is_none()
    }

    pub fn select_initial_player(&mut self) -> Option<u128> {
        if self.players.len() != 2 {
            return None;
        }

        let players: Vec<u128> = self.players.iter().copied().collect();
        let first_player = if rand::thread_rng().gen_bool(0.5) {
            players[0]
        } else {
            players[1]
        };

        self.current_turn = Some(first_player);
        Some(first_player)
    }

    pub fn switch_turn(&mut self) -> Option<u128> {
        if let Some(current) = self.current_turn {
            if let Some(opponent) = self.get_opponent(current) {
                self.current_turn = Some(opponent);
                return Some(opponent);
            }
        }
        None
    }

    pub fn get_current_turn(&self) -> Option<u128> {
        self.current_turn
    }
}