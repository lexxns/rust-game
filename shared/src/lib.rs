pub mod messages;
pub mod core;

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