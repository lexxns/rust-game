use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::{Message as WsMessage};

pub type PlayerConnection = mpsc::UnboundedSender<WsMessage>;

