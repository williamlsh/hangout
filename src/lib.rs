mod session;
mod state;
mod utils;

use session::Session;
use state::State;
use worker::{
    console_debug, event, Context, Date, Env, Request, Response, Result, RouteContext, Router,
    WebSocket, WebSocketPair,
};

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    log_request(&req);
    utils::set_panic_hook();

    let router = Router::new();
    router
        .on_async("/signal", |req, ctx| async move {
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
            wasm_bindgen_futures::spawn_local(handle_websocket(server, ctx));

            Response::from_websocket(client)
        })
        .or_else_any_method_async("/", |_, _| async {
            Response::ok("Welcome to Hangout, a private person to person calling service.")
        })
        .run(req, env)
        .await
}

/// A WebSocket server handler.
async fn handle_websocket(ws: WebSocket, ctx: RouteContext<()>) {
    let upstash_redis_url = ctx
        .secret("UPSTASH_REDIS_URL")
        .expect("expect UPSTASH_REDIS_URL");
    let upstash_redis_token = ctx
        .secret("UPSTASH_REDIS_TOKEN")
        .expect("expect UPSTASH_REDIS_TOKEN");
    // Initiates state.
    let state = State::new(
        &upstash_redis_url.to_string(),
        &upstash_redis_token.to_string(),
    );
    let session = Session::new(ws, state);
    session.start().await;
}

fn log_request(req: &Request) {
    console_debug!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    )
}
