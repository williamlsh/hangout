use serde::{Deserialize, Serialize};

/// A general Message used by WebSocket data exchange.
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub event: Event,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    Passphrase,
    Offer,
    Answer,
    IceCandidate,
}
