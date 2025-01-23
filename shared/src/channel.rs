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
    /// Current owner.
    Current(Option<u128>),
    /// Chat message from server
    Chat(MessageType)
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientRequest
{
    /// Select the button.
    ///
    /// Will be acked by the server.
    Select,
    /// Send a chat message
    Chat(MessageType)
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ChatChannel;
impl bevy_simplenet::ChannelPack for ChatChannel
{
    type ConnectMsg = ();
    type ServerMsg = ServerMsg;
    type ServerResponse = ();
    type ClientMsg = ();
    type ClientRequest = ClientRequest;
}