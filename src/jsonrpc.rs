use super::{Error, SessionDescription, Signal, SignalNotification, TrickleCandidate};
use async_trait::async_trait;
use futures::{channel::mpsc, future, future::Either, pin_mut};
use jsonrpsee::ws_client::{
    traits::Client, traits::SubscriptionClient, v2::params::JsonRpcParams, NotificationHandler,
    WsClient, WsClientBuilder,
};
use log::*;
use maplit::btreemap;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::borrow::Cow;
use std::collections::BTreeMap;
use url::Url;

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinMsg {
    pub sid: String,
    pub offer: SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JoinResponse {
    pub answer: SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NegotiateMsg {
    pub desc: SessionDescription,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrickleNotification {
    pub target: u32,
    pub candidate: TrickleCandidate,
}

pub struct JsonRPCSignaler<'a> {
    url: &'a str,
    ws: Option<WsClient>,
}

impl<'a> JsonRPCSignaler<'a> {
    pub fn new(url: &'a str) -> JsonRPCSignaler {
        JsonRPCSignaler {
            url: url,
            ws: Option::<WsClient>::None,
        }
    }
}

#[async_trait]
impl<'a> Signal for JsonRPCSignaler<'a> {
    async fn open(&mut self) -> Result<mpsc::Receiver<SignalNotification>, Error> {
        let u = Url::parse(&self.url).unwrap();

        let client = WsClientBuilder::default()
            .handshake_url(Cow::Borrowed(&u.path()))
            .build(&self.url)
            .await?;

        let mut offer: NotificationHandler<SessionDescription> =
            client.register_notification("offer").await?;
        let mut trickle: NotificationHandler<TrickleNotification> =
            client.register_notification("trickle").await?;

        let (mut tx, rx) = mpsc::channel(16);
        let mut tx_clone = tx.clone();

        glib::MainContext::default().spawn(async move {
            loop {
                if let Some(msg) = offer.next().await {
                    trace!("got offer: {:?}", msg);
                    tx_clone
                        .try_send(SignalNotification::Negotiate { offer: msg })
                        .expect("could not send offer notification to channel");
                }
            }
        });
        glib::MainContext::default().spawn(async move {
            loop {
                if let Some(msg) = trickle.next().await {
                    println!("got trickle: {:?}", msg);
                    tx.try_send(SignalNotification::Trickle {
                        target: msg.target,
                        candidate: msg.candidate,
                    })
                    .expect("could not send trickle notification to channel");
                }
            }
        });
        self.ws = Some(client);

        Ok(rx)
    }

    async fn close(&mut self) -> Result<(), Error> {
        Ok(())
    }
    async fn ping(&self) -> Result<(), Error> {
        if let Some(ws) = &self.ws {
            println!("sending ping");

            let response: String = ws.request("ping", JsonRpcParams::NoParams).await?;
            println!("got response: {}", response);

            return Ok(());
        }
        Err(Error::NotConnected)
    }

    async fn join(
        &self,
        sid: String,
        offer: SessionDescription,
    ) -> Result<SessionDescription, Error> {
        if let Some(ws) = &self.ws {
            let msg: BTreeMap<&str, Value> = btreemap! {
                "sid" => serde_json::to_value(sid).unwrap(),
                "offer" => serde_json::to_value(offer).unwrap(),
            };

            let answer: SessionDescription = ws.request("join", JsonRpcParams::Map(msg)).await?;
            return Ok(answer);
        }

        Err(Error::NotConnected)
    }

    async fn offer(&self, offer: SessionDescription) -> Result<SessionDescription, Error> {
        if let Some(ws) = &self.ws {
            let msg: BTreeMap<&str, Value> = btreemap! {
                "desc" => serde_json::to_value(offer).unwrap(),
            };

            let answer: SessionDescription = ws.request("offer", JsonRpcParams::Map(msg)).await?;
            return Ok(answer);
        }

        Err(Error::NotConnected)
    }

    async fn answer(&self, answer: SessionDescription) -> Result<(), Error> {
        if let Some(ws) = &self.ws {
            let msg: BTreeMap<&str, Value> = btreemap! {
                "desc" => serde_json::to_value(answer).unwrap(),
            };

            ws.notification("answer", JsonRpcParams::Map(msg)).await?;
            return Ok(());
        }

        Err(Error::NotConnected)
    }

    async fn trickle(&self, target: u32, candidate: TrickleCandidate) -> Result<(), Error> {
        if let Some(ws) = &self.ws {
            let msg: BTreeMap<&str, Value> = btreemap! {
                "target" => serde_json::to_value(target).unwrap(),
                "candidate" => serde_json::to_value(candidate).unwrap(),
            };

            ws.notification("trickle", JsonRpcParams::Map(msg)).await?;
            return Ok(());
        }

        Err(Error::NotConnected)
    }
}
