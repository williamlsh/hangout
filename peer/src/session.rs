use crate::{console_log, pc_callbacks, ws_callbacks};
use futures_channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use js_sys::{Array, Object, Reflect};
use protocol::{Event, Message};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    HtmlVideoElement, MediaStream, MediaStreamConstraints, RtcConfiguration, RtcIceCandidate,
    RtcIceCandidateInit, RtcPeerConnection, RtcSdpType, RtcSessionDescriptionInit, WebSocket,
};

pub(crate) struct Session {
    ws_addr: String,
    sender: UnboundedSender<Message>,
    receiver: UnboundedReceiver<Message>,
}

impl Session {
    pub(crate) fn new(ws_addr: String) -> Session {
        let (sender, receiver) = mpsc::unbounded();
        Session {
            ws_addr,
            sender,
            receiver,
        }
    }

    pub(crate) async fn start(self) -> Result<(), JsValue> {
        let ws = WebSocket::new(&self.ws_addr)?;
        ws_callbacks::set_onopen(&ws, "passphrase".into());
        ws_callbacks::set_onerror(&ws);
        ws_callbacks::set_onclose(&ws);

        let pc = RtcPeerConnection::new_with_configuration(&{
            let ice_servers = Array::new();
            let server_entry = Object::new();
            Reflect::set(
                &server_entry,
                &"urls".into(),
                &"stun:stun.l.google.com:19302".into(),
            )?;
            ice_servers.push(&server_entry);

            let mut rtc_configuration = RtcConfiguration::new();
            rtc_configuration.ice_servers(&ice_servers);
            rtc_configuration
        })?;
        console_log!("created pc");

        Self::init_local_stream(&pc).await.unwrap();

        pc_callbacks::set_ontrack(&pc);
        pc_callbacks::set_onconnectionstatechange(&pc);
        pc_callbacks::set_onicecandidate(&pc, ws.clone());

        wasm_bindgen_futures::spawn_local(Self::handle_message(
            self.receiver,
            ws.clone(),
            pc.clone(),
        ));

        ws_callbacks::set_onmessage(&ws, self.sender);

        Ok(())
    }

    async fn init_local_stream(pc: &RtcPeerConnection) -> Result<(), JsValue> {
        let local_stream = {
            let promise = web_sys::window()
                .unwrap()
                .navigator()
                .media_devices()?
                .get_user_media_with_constraints(&{
                    let mut media_stream_constraints = MediaStreamConstraints::new();
                    media_stream_constraints
                        .video(&JsValue::from_bool(true))
                        .audio(&JsValue::from_bool(false));
                    media_stream_constraints
                })?;
            let local_stream = JsFuture::from(promise).await?;
            // local_stream.dyn_into().unwrap()
            MediaStream::from(local_stream)
        };
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("localVideo")
            .unwrap()
            .dyn_into::<HtmlVideoElement>()
            .unwrap()
            .set_src_object(Some(&local_stream));
        local_stream
            .get_tracks()
            .for_each(&mut |track: JsValue, _, _| {
                pc.add_track_0(track.dyn_ref().unwrap(), &local_stream);
                console_log!("added a local track");
            });

        console_log!("initialized local stream");

        Ok(())
    }

    async fn handle_message(
        mut receiver: UnboundedReceiver<Message>,
        ws: WebSocket,
        pc: RtcPeerConnection,
    ) {
        loop {
            if let Ok(Some(message)) = receiver.try_next() {
                match message.event {
                    Event::Passphrase => {
                        // If peer's role is caller, send its offer to callee.
                        if message.data.eq("1") {
                            console_log!("this is a caller");

                            Self::send_offer(&ws, &pc).await.unwrap();
                            console_log!("caller sent offer");
                        }
                    }
                    Event::Offer => {
                        // Callee receives offer from caller.
                        console_log!("callee received offer");

                        let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
                        offer_obj.sdp(&message.data);
                        let srd_promise = pc.set_remote_description(&offer_obj);
                        JsFuture::from(srd_promise).await.unwrap();
                        console_log!("pc: state {:?}", pc.signaling_state());

                        // Callee returns answer to caller.
                        Self::send_answer(&ws, &pc).await.unwrap();

                        console_log!("callee sent answer back");
                    }
                    Event::Answer => {
                        // Caller receives answer from callee.
                        console_log!("caller received answer");

                        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
                        answer_obj.sdp(&message.data);
                        let srd_promise = pc.set_remote_description(&answer_obj);
                        JsFuture::from(srd_promise).await.unwrap();
                        console_log!("pc: state {:?}", pc.signaling_state());
                    }
                    Event::IceCandidate => {
                        console_log!("received a candidate");

                        let candidate =
                            RtcIceCandidate::new(&RtcIceCandidateInit::new(&message.data)).unwrap();
                        let promise =
                            pc.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate));
                        JsFuture::from(promise).await.unwrap();
                    }
                }
            }
        }
    }

    async fn send_offer(ws: &WebSocket, pc: &RtcPeerConnection) -> Result<(), JsValue> {
        let offer = JsFuture::from(pc.create_offer()).await?;
        let offer_sdp = Reflect::get(&offer, &JsValue::from_str("sdp"))?
            .as_string()
            .unwrap();
        console_log!("offer {:?}", offer_sdp);

        let mut offer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
        offer_obj.sdp(&offer_sdp);
        let sld_promise = pc.set_local_description(&offer_obj);
        JsFuture::from(sld_promise).await?;
        console_log!("pc: state {:?}", pc.signaling_state());

        let message = Message {
            event: Event::Offer,
            data: offer_sdp,
        };
        ws.send_with_str(&serde_json::to_string(&message).unwrap())
    }

    async fn send_answer(ws: &WebSocket, pc: &RtcPeerConnection) -> Result<(), JsValue> {
        let answer = JsFuture::from(pc.create_answer()).await.unwrap();
        let answer_sdp = Reflect::get(&answer, &JsValue::from_str("sdp"))
            .unwrap()
            .as_string()
            .unwrap();
        console_log!("pc: answer {:?}", answer_sdp);

        let mut answer_obj = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
        answer_obj.sdp(&answer_sdp);
        let sld_promise = pc.set_local_description(&answer_obj);
        JsFuture::from(sld_promise).await.unwrap();
        console_log!("pc: state {:?}", pc.signaling_state());

        let message = Message {
            event: Event::Answer,
            data: answer_sdp,
        };
        ws.send_with_str(&serde_json::to_string(&message).unwrap())
    }
}
