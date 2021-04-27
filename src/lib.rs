use serde::{Deserialize, Serialize};

pub enum Error {}

#[derive(Serialize, Deserialize)]
pub struct SessionDescription {
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
pub trait Signal {
    fn open(url: String) -> Result<(), Error>;
    fn close() -> Result<(), Error>;
    fn ping() -> Result<(), Error>;

    fn join(sid: String, offer: SessionDescription) -> Result<(), Error>;
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
