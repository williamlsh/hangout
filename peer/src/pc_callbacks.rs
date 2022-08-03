use crate::console_log;
use protocol::{Event, Message};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{
    HtmlMediaElement, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcTrackEvent, WebSocket,
};

pub(crate) fn set_onicecandidate(pc: &RtcPeerConnection, ws: WebSocket) {
    let onicecandidate_callback =
        Closure::<dyn FnMut(_)>::new(move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
            Some(candidate) => {
                console_log!("pc.onicecandidate: {}", candidate.candidate());
                let message = Message {
                    event: Event::IceCandidate,
                    data: candidate.candidate(),
                };
                ws.send_with_str(&serde_json::to_string(&message).unwrap())
                    .unwrap();
                console_log!("successfully sent a candidate");
            }
            None => {}
        });
    pc.set_onicecandidate(Some(onicecandidate_callback.as_ref().unchecked_ref()));
    onicecandidate_callback.forget();
}

pub(crate) fn set_onconnectionstatechange(pc: &RtcPeerConnection) {
    let pc_clone = pc.clone();
    let onconnectionstatechange_callback = Closure::<dyn FnMut()>::new(move || {
        console_log!("pc state: {:?}", pc_clone.ice_connection_state());
    });
    pc.set_oniceconnectionstatechange(Some(
        onconnectionstatechange_callback.as_ref().unchecked_ref(),
    ));
}

pub(crate) fn set_ontrack(pc: &RtcPeerConnection) {
    let ontrack_callback = Closure::<dyn FnMut(_)>::new(move |ev: RtcTrackEvent| {
        let first_remote_stream = ev.streams().pop();
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("remoteVideo")
            .unwrap()
            .unchecked_into::<HtmlMediaElement>()
            .set_src_object(first_remote_stream.dyn_ref());
    });
    pc.set_ontrack(Some(ontrack_callback.as_ref().unchecked_ref()));
}
