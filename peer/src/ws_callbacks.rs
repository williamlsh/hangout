use crate::{console_error, console_log};
use futures_channel::mpsc::UnboundedSender;
use protocol::Message;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

pub(crate) fn set_onopen(ws: &WebSocket, message: String) {
    let ws_clone = ws.clone();
    let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        console_log!("WebSocket opened");

        // Send off the first message on open.
        match ws_clone.send_with_str(&message) {
            Ok(_) => console_log!("first message successfully sent"),
            Err(err) => console_error!("error sending first message: {:?}", err),
        }
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();
}

pub(crate) fn set_onmessage(ws: &WebSocket, sender: UnboundedSender<Message>) {
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Ok(msg) = e.data().dyn_into::<js_sys::JsString>() {
            let message: Message = serde_json::from_str(&msg.as_string().unwrap()).unwrap();
            sender.unbounded_send(message).unwrap();
        } else {
            console_log!("message event, received Unknown: {:?}", e.data());
        }
    });
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
}

pub(crate) fn set_onerror(ws: &WebSocket) {
    let onerror_callback =
        Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| console_error!("error event: {:?}", e));
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();
}

pub(crate) fn set_onclose(ws: &WebSocket) {
    let onclose_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        console_log!("WebSocket closed: {:?}", e)
    });
    ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();
}
