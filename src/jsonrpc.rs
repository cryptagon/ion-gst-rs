use super::{Error, SessionDescription, Signal, TrickleCandidate};
use async_trait::async_trait;
use futures::select;
use jsonrpsee::ws_client::{
    traits::Client, traits::SubscriptionClient, v2::params::JsonRpcParams, Subscription, WsClient,
    WsClientBuilder,
};
use maplit::btreemap;
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::borrow::Cow;
use std::collections::BTreeMap;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct JoinMsg {
    pub sid: String,
    pub offer: SessionDescription,
}

#[derive(Serialize, Deserialize)]
pub struct JoinResponse {
    pub answer: SessionDescription,
}

pub struct JsonRPCSignaler {
    ws: Option<WsClient>,
}

impl JsonRPCSignaler {
    pub fn new() -> JsonRPCSignaler {
        JsonRPCSignaler {
            ws: Option::<WsClient>::None,
        }
    }
}

#[async_trait(?Send)]
impl Signal for JsonRPCSignaler {
    async fn open(&mut self, url: String) -> Result<(), Error> {
        let u = Url::parse(&url).unwrap();

        let client = WsClientBuilder::default()
            .handshake_url(Cow::Borrowed(&u.path()))
            .build(&url)
            .await?;

        let mut offer: Subscription<SessionDescription> = client.on_notification("offer").await?;
        let mut trickle: Subscription<SessionDescription> =
            client.on_notification("trickle").await?;

        glib::MainContext::default().spawn(async move {
            loop {
                if let Some(msg) = offer.next().await {
                    println!("got offer: {:?}", msg);
                }
            }
        });

        glib::MainContext::default().spawn(async move {
            loop {
                if let Some(msg) = trickle.next().await {
                    println!("got trickle: {:?}", msg);
                }
            }
        });

        self.ws = Some(client);

        Ok(())
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

            ws.notification("answer", JsonRpcParams::Map(msg)).await?;
            return Ok(());
        }

        Err(Error::NotConnected)
    }
}
