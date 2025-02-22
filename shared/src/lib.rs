pub mod message_utils;
pub mod channel;
pub mod card_details;

pub type EntityID = u128;

pub mod models {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct User {
        pub id: u64,
        pub name: String,
    }
}

pub mod api {
    pub const API_VERSION: &str = "v0.0.1";
}