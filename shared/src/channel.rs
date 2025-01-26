use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageType {
    Room {
        #[serde(skip_serializing_if = "Option::is_none")]
        sender: Option<String>,
        content: String,
    },
    Private {
        #[serde(skip_serializing_if = "Option::is_none")]
        sender: Option<String>,
        recipient: String,
        content: String
    },
    System(String),
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMsg
{
    /// Turn Player.
    Current(Option<u128>),
    /// Chat message from server
    Chat(MessageType)
}

//-------------------------------------------------------------------------------------------------------------------

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub enum ClientRequest
    {
        /// Send a chat message
        Chat(MessageType),
        EndTurn,
    }

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GameChannel;
impl bevy_simplenet::ChannelPack for GameChannel
{
    type ConnectMsg = ();
    type ServerMsg = ServerMsg;
    type ServerResponse = ();
    type ClientMsg = ();
    type ClientRequest = ClientRequest;
}