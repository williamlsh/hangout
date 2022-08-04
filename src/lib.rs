mod session;
mod state;
mod utils;

use session::Session;
use state::State;
use worker::{
    console_debug, event, Context, Date, Env, Headers, Request, Response, Result, RouteContext,
    Router, WebSocket, WebSocketPair,
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
        .on_async("/", |req, ctx| async {
            let asset = handle_assets(req, ctx).await;
            Response::from_html(&asset)
        })
        .on_async("/pkg/peer.js", |req, ctx| async {
            let asset = handle_assets(req, ctx).await;
            let mut headers = Headers::new();
            headers
                .set("Content-Type", "application/javascript")
                .unwrap();
            Response::ok(&asset).map(|response| response.with_headers(headers))
        })
        .on_async("/pkg/peer_bg.wasm", |req, ctx| async {
            let asset = handle_wasm_asset(req, ctx).await;
            let mut headers = Headers::new();
            headers.set("Content-Type", "application/wasm").unwrap();
            Response::from_bytes(asset).map(|response| response.with_headers(headers))
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

async fn handle_assets(req: Request, ctx: RouteContext<()>) -> String {
    let extension = match req.path().as_str() {
        "/" => "html",
        "/pkg/peer.js" => "js",
        _ => "html",
    };
    console_debug!(
        "request path: {}, path extension: {}",
        req.path(),
        extension
    );

    let kv_store = ctx.kv("__STATIC_CONTENT").unwrap();
    let list = kv_store.list().execute().await.unwrap();
    let key = list.keys.iter().find(|&key| key.name.ends_with(extension));
    match key {
        Some(key) => kv_store
            .get(&key.name)
            .text()
            .await
            .unwrap()
            .unwrap_or_default(),
        None => "".into(),
    }
}

async fn handle_wasm_asset(_req: Request, ctx: RouteContext<()>) -> Vec<u8> {
    let kv_store = ctx.kv("__STATIC_CONTENT").unwrap();
    let list = kv_store.list().execute().await.unwrap();
    let key = list.keys.iter().find(|&key| key.name.ends_with("wasm"));
    kv_store
        .get(&key.unwrap().name)
        .bytes()
        .await
        .unwrap()
        .unwrap()
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
