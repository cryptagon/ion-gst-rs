use async_mutex::Mutex;
use async_trait::async_trait;
use derive_more::Display;
use futures::channel::mpsc;
use futures::stream::StreamExt;
use gst::prelude::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionDescription {
    #[serde(rename = "type")]
    pub t: String,
    pub sdp: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickleCandidate {
    pub candidate: String,
    #[serde(rename = "sdpMid")]
    pub sdp_mid: Option<String>,
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_mline_index: u32,
}

#[derive(Debug)]
pub enum SignalNotification {
    Negotiate {
        offer: SessionDescription,
    },
    Trickle {
        target: u32,
        candidate: TrickleCandidate,
    },
}

enum WebrtcBinEvent {
    NegotiationNeeded,
    IceCandidate(TrickleCandidate),
}

#[async_trait]
pub trait Signal {
    async fn open(&mut self) -> Result<mpsc::Receiver<SignalNotification>, Error>;
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

pub struct Client<S: Signal + Send + Sync + 'static> {
    signal: Arc<Mutex<S>>,

    pub publisher: gst::Element,
    pub subscriber: gst::Element,
}

impl<S: Signal + Send + Sync> Client<S> {
    pub fn new<'a>(
        signal: S,
        pipeline: gst::Pipeline,
        publisher: &'a str,
        subscriber: &'a str,
    ) -> Client<S> {
        let publisher = pipeline.get_by_name(publisher).unwrap();
        let subscriber = pipeline.get_by_name(subscriber).unwrap();
        //let publisher = gst::ElementFactory::make("webrtcbin", Some("publisher"))
        //    .expect("error creating webrtcbin");
        publisher.set_property_from_str("stun-server", STUN_SERVER);
        publisher.set_property_from_str("bundle-policy", "max-bundle");

        //let subscriber = gst::ElementFactory::make("webrtcbin", Some("subscriber"))
        //    .expect("error creating webrtcbin");
        subscriber.set_property_from_str("stun-server", STUN_SERVER);
        subscriber.set_property_from_str("bundle-policy", "max-bundle");

        //pipeline
        //    .add_many(&[&publisher, &subscriber])
        //    .expect("error adding transports to pipeline");

        //publisher.sync_state_with_parent().unwrap();
        //subscriber.sync_state_with_parent().unwrap();

        Client {
            signal: Arc::new(Mutex::new(signal)),
            publisher: publisher,
            subscriber: subscriber,
        }
    }

    pub async fn join(&mut self, sid: String) -> Result<(), Error> {
        let mut rx = { self.signal.lock().await.open().await? };

        let pub_clone = self.publisher.clone();
        let sub_clone = self.subscriber.clone();
        let signal = self.signal.clone();

        glib::MainContext::default().spawn(async move {
            use SignalNotification::*;
            while let Some(notification) = rx.next().await {
                match notification {
                    Trickle { target, candidate } => {
                        let pc: &gst::Element = match target {
                            0 => &pub_clone,
                            1 => &sub_clone,
                            _ => panic!("got unexpected trickle target = {}", target),
                        };
                        debug!("adding ice candidate");

                        pc.emit(
                            "add-ice-candidate",
                            &[&candidate.sdp_mline_index, &candidate.candidate],
                        )
                        .expect("could not call add-ice-candidate on pc");
                    }

                    Negotiate { offer } => {
                        let pc = &sub_clone;

                        let ret = gst_sdp::SDPMessage::parse_buffer(offer.sdp.as_bytes())
                            .map_err(|_| Error::SDPError)
                            .expect("error parsing inbound offer");
                        let offer = gst_webrtc::WebRTCSessionDescription::new(
                            gst_webrtc::WebRTCSDPType::Offer,
                            ret,
                        );

                        pc.emit("set-remote-description", &[&offer, &None::<gst::Promise>])
                            .expect("sub failed setting remote description");

                        let (promise, fut) = gst::Promise::new_future();

                        pc.emit("create-answer", &[&None::<gst::Structure>, &promise])
                            .expect("sub failed to emit create-answer signal on");

                        let reply = fut.await;

                        // Check if we got a valid offer
                        let reply = match reply {
                            Ok(Some(reply)) => reply,
                            Ok(None) => {
                                error!("sub answer creation got no reponse");
                                continue;
                            }
                            Err(err) => {
                                error!("sub answer creation got error reponse: {:?}", err);
                                continue;
                            }
                        };

                        let answer = reply
                            .get_value("answer")
                            .expect("Invalid argument")
                            .get::<gst_webrtc::WebRTCSessionDescription>()
                            .expect("Invalid argument")
                            .unwrap();

                        pc.emit("set-local-description", &[&answer, &None::<gst::Promise>])
                            .expect("sub answer error set-local-description");

                        let answer = SessionDescription {
                            t: "answer".to_string(),
                            sdp: answer.get_sdp().as_text().unwrap(),
                        };

                        // signal lock exists for this scope
                        signal
                            .lock()
                            .await
                            .answer(answer)
                            .await
                            .expect("sub error sending answer over signal");
                    }
                }
            }
        });

        //        self.publisher
        //            .emit(
        //                "create-data-channel",
        //                &[&"ion-sfu", &None::<gst::Structure>],
        //            )
        //            .unwrap();
        //
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

        let offer = reply
            .get_value("offer")
            .expect("Invalid argument")
            .get::<gst_webrtc::WebRTCSessionDescription>()
            .expect("Invalid argument")
            .unwrap();

        debug!("Created pub offer {:#?}", offer.get_sdp());

        self.publisher
            .emit("set-local-description", &[&offer, &None::<gst::Promise>])
            .expect("Failed to emit set-local-description signal");

        let offer = SessionDescription {
            t: "offer".to_string(),
            sdp: offer.get_sdp().as_text().unwrap(),
        };

        // send join offer to server and await answer
        let answer = self.signal.lock().await.join(sid, offer).await?;

        trace!("Received pub answer");

        let ret = gst_sdp::SDPMessage::parse_buffer(answer.sdp.as_bytes())
            .map_err(|_| Error::SDPError)?;
        let answer =
            gst_webrtc::WebRTCSessionDescription::new(gst_webrtc::WebRTCSDPType::Answer, ret);

        self.publisher
            .emit("set-remote-description", &[&answer, &None::<gst::Promise>])
            .unwrap();

        let (tx, mut rx) = mpsc::unbounded();
        let tx_clone = tx.clone();
        let signal = self.signal.clone();
        let pub_clone = self.publisher.clone();

        self.publisher
            .connect("on-negotiation-needed", false, move |_| {
                info!("pub negotiation needed");
                tx.unbounded_send(WebrtcBinEvent::NegotiationNeeded)
                    .unwrap();
                None
            })
            .unwrap();

        self.publisher
            .connect("on-ice-candidate", false, move |values| {
                let _webrtc = values[0].get::<gst::Element>().expect("Invalid argument");
                let mlineindex = values[1].get_some::<u32>().expect("Invalid argument");
                let candidate = values[2]
                    .get::<String>()
                    .expect("Invalid argument")
                    .unwrap();

                tx_clone
                    .unbounded_send(WebrtcBinEvent::IceCandidate(TrickleCandidate {
                        sdp_mline_index: mlineindex,
                        sdp_mid: None,
                        candidate: candidate,
                    }))
                    .unwrap();

                None
            })
            .unwrap();

        glib::MainContext::default().spawn(async move {
            while let Some(evt) = rx.next().await {
                match evt {
                    WebrtcBinEvent::NegotiationNeeded => {
                        Client::on_pub_negotiation_needed(&signal, &pub_clone)
                            .await
                            .unwrap()
                    }
                    WebrtcBinEvent::IceCandidate(candidate) => {
                        //send pub ice candidate to server
                        debug!("publisher sending ice candidate");
                        signal.lock().await.trickle(0, candidate).await.unwrap()
                    }
                }
            }

            panic!("done");
        });

        Ok(())
    }

    async fn on_pub_negotiation_needed(
        signal: &Arc<Mutex<S>>,
        publisher: &gst::Element,
    ) -> Result<(), Error> {
        info!("pub negotiations, creating offer");
        let (promise, fut) = gst::Promise::new_future();
        publisher
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

        let offer = reply
            .get_value("offer")
            .expect("Invalid argument")
            .get::<gst_webrtc::WebRTCSessionDescription>()
            .expect("Invalid argument")
            .unwrap();

        debug!("Created pub offer {:#?}", offer.get_sdp());

        publisher
            .emit("set-local-description", &[&offer, &None::<gst::Promise>])
            .expect("Failed to emit set-local-description signal");

        let offer = SessionDescription {
            t: "offer".to_string(),
            sdp: offer.get_sdp().as_text().unwrap(),
        };

        // send join offer to server and await answer
        let answer = signal.lock().await.offer(offer).await?;

        debug!("Received pub answer");

        let ret = gst_sdp::SDPMessage::parse_buffer(answer.sdp.as_bytes())
            .map_err(|_| Error::SDPError)?;
        let answer =
            gst_webrtc::WebRTCSessionDescription::new(gst_webrtc::WebRTCSDPType::Answer, ret);

        publisher
            .emit("set-remote-description", &[&answer, &None::<gst::Promise>])
            .unwrap();

        info!("pub negotiation completed");
        Ok(())
    }

    pub async fn ping(&self) -> Result<(), Error> {
        self.signal.lock().await.ping().await
    }
}
