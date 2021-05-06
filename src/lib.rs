use async_trait::async_trait;
use async_tungstenite::tungstenite;
use derive_more::Display;
use gst::prelude::*;
use log::*;
use serde::{Deserialize, Serialize};

pub mod jsonrpc;

const STUN_SERVER: &str = "stun://stun.l.google.com:19302";

#[derive(Debug, Display)]
pub enum Error {
    WebsocketError(jsonrpsee::ws_client::Error),
    SDPError,
    NotConnected,
}

impl From<jsonrpsee::ws_client::Error> for Error {
    fn from(i: jsonrpsee::ws_client::Error) -> Error {
        Error::WebsocketError(i)
    }
}

impl std::error::Error for Error {}

#[derive(Serialize, Deserialize)]
pub struct SessionDescription {
    #[serde(rename = "type")]
    pub t: String,
    pub sdp: String,
}

#[derive(Serialize, Deserialize)]
pub struct TrickleCandidate {
    pub candidate: String,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: String,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_mline_index: u16,
}

pub enum SignalNotification {
    OnNegotiate {
        offer: SessionDescription,
    },
    OnTrickle {
        target: i32,
        candidate: TrickleCandidate,
    },
}

#[async_trait(?Send)]
pub trait Signal {
    async fn open(&mut self, url: String) -> Result<(), Error>;
    async fn close(&mut self) -> Result<(), Error>;
    async fn ping(&self) -> Result<(), Error>;

    async fn join(
        &self,
        sid: String,
        offer: SessionDescription,
    ) -> Result<SessionDescription, Error>;

    async fn offer(&self, offer: SessionDescription) -> Result<SessionDescription, Error>;
    async fn answer(&self, answer: SessionDescription) -> Result<(), Error>;
    async fn trickle(&self, target: u32, candidate: TrickleCandidate) -> Result<(), Error>;
}

pub struct Client<S: Signal> {
    signal: S,

    publisher: gst::Element,
    subscriber: gst::Element,
}

impl<S: Signal> Client<S> {
    pub fn new(signal: S, pipeline: gst::Pipeline) -> Client<S> {
        let publisher = gst::ElementFactory::make("webrtcbin", Some("publisher"))
            .expect("error creating webrtcbin");
        publisher.set_property_from_str("stun-server", STUN_SERVER);
        publisher.set_property_from_str("bundle-policy", "max-bundle");

        let subscriber =
            gst::ElementFactory::make("webrtcbin", None).expect("error creating webrtcbin");

        pipeline
            .add_many(&[&publisher, &subscriber])
            .expect("error adding transports to pipeline");

        pipeline.set_state(gst::State::Playing).unwrap();

        Client {
            signal: signal,
            publisher: publisher,
            subscriber: subscriber,
        }
    }

    pub async fn join(&self, sid: String) -> Result<(), Error> {
        self.publisher
            .emit(
                "create-data-channel",
                &[&"ion-sfu", &None::<gst::Structure>],
            )
            .unwrap();

        let (promise, fut) = gst::Promise::new_future();
        self.publisher
            .emit("create-offer", &[&None::<gst::Structure>, &promise])
            .expect("Failed to emit create-offer signal");

        let reply = fut.await;

        // Check if we got a valid offer
        let reply = match reply {
            Ok(Some(reply)) => reply,
            Ok(None) => {
                error!("Offer creation got no reponse");
                return Err(Error::SDPError);
            }
            Err(err) => {
                error!("Offer creation got error reponse: {:?}", err);
                return Err(Error::SDPError);
            }
        };

        println!("got webrtcbin offer: {:?}", reply);

        let offer = reply
            .get_value("offer")
            .expect("Invalid argument")
            .get::<gst_webrtc::WebRTCSessionDescription>()
            .expect("Invalid argument")
            .unwrap();

        trace!("Created offer {:#?}", offer.get_sdp());

        self.publisher
            .emit("set-local-description", &[&offer, &None::<gst::Promise>])
            .expect("Failed to emit set-local-description signal");

        let offer = SessionDescription {
            t: "offer".to_string(),
            sdp: offer.get_sdp().as_text().unwrap(),
        };

        // send join offer to server and await answer
        let answer = self.signal.join(sid, offer).await?;

        let ret = gst_sdp::SDPMessage::parse_buffer(answer.sdp.as_bytes())
            .map_err(|_| Error::SDPError)?;
        let answer =
            gst_webrtc::WebRTCSessionDescription::new(gst_webrtc::WebRTCSDPType::Answer, ret);

        self.publisher
            .emit("set-remote-description", &[&answer, &None::<gst::Promise>])
            .unwrap();

        Ok(())
    }

    pub async fn ping(&self) -> Result<(), Error> {
        self.signal.ping().await
    }
}
