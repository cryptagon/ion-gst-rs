use async_trait::async_trait;
use async_tungstenite::tungstenite;
use derive_more::Display;
use gst::prelude::*;
use serde::{Deserialize, Serialize};

pub mod jsonrpc;

#[derive(Debug, Display)]
pub enum Error {
    WebsocketError(jsonrpsee::ws_client::Error),
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
    fn new(signal: S) -> Client<S> {
        let publisher =
            gst::ElementFactory::make("webrtcbin", None).expect("error creating webrtcbin");
        let subscriber =
            gst::ElementFactory::make("webrtcbin", None).expect("error creating webrtcbin");

        Client {
            signal: signal,
            publisher: publisher,
            subscriber: subscriber,
        }
    }

    async fn join(&self, sid: String) -> Result<(), Error> {
        Ok(())
    }
}
