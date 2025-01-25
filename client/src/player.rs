use crate::state::TurnPlayer;
use crate::client::Client;

/// A trait for checking if a player ID belongs to the current client
pub trait PlayerOwnership {
    /// Returns true if the given player ID matches the current client
    fn is_client_player(&self, client: &Client) -> bool;

    /// Returns true if the given player ID belongs to an opponent
    fn is_opponent(&self, client: &Client) -> bool {
        !self.is_client_player(client)
    }
}

// Implement for raw player IDs
impl PlayerOwnership for u128 {
    fn is_client_player(&self, client: &Client) -> bool {
        *self == client.id()
    }
}

// Implement for Option<u128> to handle cases where player ID might be None
impl PlayerOwnership for Option<u128> {
    fn is_client_player(&self, client: &Client) -> bool {
        self.map_or(false, |id| id == client.id())
    }
}

/// A helper function to check current turn ownership using the TurnPlayer resource
pub fn is_clients_turn(turn_player: &TurnPlayer, client: &Client) -> bool {
    // First check predicted state, then fall back to server-determined state
    turn_player.predicted_player_id
        .map_or_else(
            || turn_player.server_determined_player_id.is_client_player(client),
            |id| id.is_client_player(client)
        )
}
