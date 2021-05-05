use async_trait::async_trait;
use async_tungstenite::tungstenite;
use serde::{Deserialize, Serialize};
use derive_more::{Display};


pub mod jsonrpc;

#[derive(Debug,Display)]
pub enum Error {
    WebsocketError(jsonrpsee::ws_client::Error),
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
    async fn open(url: String) -> Result<(), Error>;
    async fn close() -> Result<(), Error>;
    async fn ping() -> Result<(), Error>;

    async fn join(sid: String, offer: SessionDescription) -> Result<(), Error>;
}

pub struct Client<S: Signal> {
    signal: S,
}

impl<S: Signal> Client<S> {
    fn new(signal: S) -> Client<S> {
        Client { signal: signal }
    }

    fn join() -> Result<(), Error> {
        Ok(())
    }
}
