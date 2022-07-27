use futures::StreamExt;
use worker::{
    console_log, event, Context, Date, Env, Request, Response, Result, Router, WebSocket,
    WebSocketPair, WebsocketEvent,
};
mod utils;

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();

    let router = Router::new();
    router
        .on_async("/signal", |req, _ctx| async move {
            // For WebSocket connection flow, see: https://www.wallarm.com/what/a-simple-explanation-of-what-a-websocket-is#:~:text=In%20WebSocket%2C%20communication%20occurs%20at,party%20to%20terminate%20the%20connection.
            if !req
                .headers()
                .get("Upgrade")?
                .unwrap_or_default()
                .eq("websocket")
            {
                return Response::error("Expected Upgrade: websocket", 426);
            }

            let WebSocketPair { client, server } = WebSocketPair::new()?;

            server.accept()?;
            wasm_bindgen_futures::spawn_local(handle_websocket(server));

            Response::from_websocket(client)
        })
        .or_else_any_method("/", |_, _| {
            Response::ok("Welcome to Hangout, a private person to person calling service.")
        })
        .run(req, env)
        .await
}

/// A WebSocket server handler.
async fn handle_websocket(ws: WebSocket) {
    let mut event_stream = ws.events().expect("could not open stream");

    while let Some(event) = event_stream.next().await {
        match event.expect("received error in websocket") {
            WebsocketEvent::Message(msg) => console_log!("{:#?}", msg),
            WebsocketEvent::Close(code) => console_log!("Closed: {:#?}", code),
        }
    }
}

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    )
}
