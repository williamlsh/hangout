use crate::state::{Response as StateResponse, Result as StateResult, State};
use futures::StreamExt;
use futures_channel::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use worker::{console_debug, console_error, console_log, Delay, WebSocket, WebsocketEvent};

#[derive(Debug)]
pub(crate) struct Session {
    websocket: WebSocket,
    state: State,

    signal_sender: Sender<()>,
    signal_receiver: Receiver<()>,
}

/// A session registry.
#[derive(Debug, Clone)]
struct Registry {
    /// The passphrase key to be set on Redis.
    passphrase_key: String,
    /// Sender channel key to be set on Redis.
    send_channel_key: String,
    /// Receiver channel key to be set on Redis.
    receive_channel_key: String,
}

#[derive(Debug)]
enum Role {
    Caller,
    Callee,
}

impl Session {
    /// Creates a new session.
    pub(crate) fn new(websocket: WebSocket, state: State) -> Session {
        let (tx, rx) = mpsc::channel(0);
        Session {
            websocket,
            state,
            signal_sender: tx,
            signal_receiver: rx,
        }
    }

    pub(crate) async fn start(mut self) {
        let mut registry = None;

        // Read the first message to get passphrase.
        let mut event_stream = self.websocket.events().expect("could not open stream");
        let first_event = event_stream.next().await.expect("expect first message");
        match first_event.expect("received error in websocket") {
            WebsocketEvent::Message(msg) => {
                // Get the passphrase as text.
                let passphrase = msg.text().expect("expect a passphrase");
                console_debug!("got passphrase: {:#?}", passphrase);

                // Try to set this passphrase as key in Redis.
                match self
                    .state
                    .set_nx(&Registry::passphrase_key(&passphrase))
                    .await
                {
                    StateResponse::Result(result) => {
                        console_debug!("got set result: {:#?}", result);

                        // Get the role.
                        // Response to client according to result.
                        let role = match result {
                            StateResult::Str(value) if value.eq("OK") => {
                                // The passphrase doesn't exist before.
                                console_debug!("this is caller");
                                Role::Caller
                            }
                            StateResult::Null => {
                                // The passphrase already exists.
                                console_debug!("this is callee");
                                Role::Callee
                            }
                            _ => {
                                console_debug!("unknown result: {:#?}", result);
                                return;
                            }
                        };
                        Self::response_based_on_role(&role, &self.websocket);
                        registry = Some(Registry::new(passphrase, role));
                    }
                    StateResponse::Error(error) => {
                        console_error!("could not execute set command on state: {}", error);
                        // Signal subscription task to exit.
                        self.signal_sender.close_channel();
                        return;
                    }
                }

                // Once set the passphrase, subscribe to the other party's state channel immediately.
                wasm_bindgen_futures::spawn_local(Self::subscribe(
                    self.state.clone(),
                    self.websocket.clone(),
                    registry.clone().expect("expect a registry"),
                    self.signal_receiver,
                ));
            }
            WebsocketEvent::Close(_) => console_log!("WebSocket connection closed"),
        }

        // Forward client messages to sate channel.
        while let Some(event) = event_stream.next().await {
            match event.expect("received error in websocket") {
                WebsocketEvent::Message(msg) => {
                    console_debug!("received message: {:#?}", msg);

                    if let Some(content) = msg.text() {
                        let channel_key = &registry
                            .as_ref()
                            .expect("expect a registry")
                            .send_channel_key;
                        // Send message to state channel.
                        let response = self.state.send(channel_key, &content).await;
                        if let StateResponse::Error(error) = response {
                            console_error!("failed to send message to state channel: {}", error);

                            // Signal subscription task to exit.
                            self.signal_sender.close_channel();
                            return;
                        }
                        console_debug!("a message is forwarded to state channel");
                    }
                }
                WebsocketEvent::Close(_) => {
                    console_log!("WebSocket connection closed");

                    // Signal subscription task to exit.
                    self.signal_sender.close_channel();
                    return;
                }
            }
        }
    }

    async fn subscribe(
        state: State,
        websocket: WebSocket,
        registry: Registry,
        mut rx: Receiver<()>,
    ) {
        loop {
            let response = state.receive(&registry.receive_channel_key).await;
            match response {
                StateResponse::Error(error) => {
                    console_error!("error occurred when receive from state channel: {}", error);
                    return;
                }
                StateResponse::Result(result) => match result {
                    StateResult::Str(msg) => websocket.send_with_str(msg).unwrap(),
                    StateResult::Null => {
                        // Sleep for 1000 ms.
                        Delay::from(Duration::from_millis(1000)).await;
                    }
                    _ => {}
                },
            }

            // Exit subscription after parent task exists.
            if let Ok(None) = rx.try_next() {
                // Delete the passphrase stored in Redis during session.
                let keys = [
                    registry.passphrase_key.as_str(),
                    registry.send_channel_key.as_str(),
                ];
                state.del_keys(&keys).await;
                break;
            }
        }
    }

    fn response_based_on_role(role: &Role, ws: &WebSocket) {
        let message = match role {
            Role::Caller => protocol::Message {
                event: protocol::Event::Passphrase,
                data: "1".into(),
            },
            Role::Callee => protocol::Message {
                event: protocol::Event::Passphrase,
                data: "0".into(),
            },
        };
        let serialized_message = serde_json::to_string(&message).unwrap();
        ws.send_with_str(&serialized_message).unwrap();
    }
}

impl Registry {
    fn new(passphrase: String, role: Role) -> Registry {
        // A peer sends on its onw channel, receives on the other party's channel.
        let (send_channel_key, receive_channel_key) = match role {
            Role::Caller => (
                Self::caller_channel_key(&passphrase),
                Self::callee_channel_key(&passphrase),
            ),
            Role::Callee => (
                Self::callee_channel_key(&passphrase),
                Self::caller_channel_key(&passphrase),
            ),
        };
        let passphrase_key = Self::passphrase_key(&passphrase);
        Registry {
            passphrase_key,
            send_channel_key,
            receive_channel_key,
        }
    }

    fn passphrase_key(passphrase: &str) -> String {
        format!("passphrase:{}", passphrase)
    }

    fn caller_channel_key(passphrase: &str) -> String {
        format!("channel:caller:{}", passphrase)
    }

    fn callee_channel_key(passphrase: &str) -> String {
        format!("channel:callee:{}", passphrase)
    }
}
