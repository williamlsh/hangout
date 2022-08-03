use session::Session;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

mod pc_callbacks;
mod session;
mod utils;
mod ws_callbacks;

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    utils::set_panic_hook();

    let session = Session::new("ws://localhost:8787/signal".into());
    session.start().await
}
