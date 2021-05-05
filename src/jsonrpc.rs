use super::{Error, SessionDescription, Signal};
use async_trait::async_trait;
use async_tungstenite::{gio::connect_async, tungstenite::Message};
use futures::prelude::*;
use jsonrpsee::ws_client::{traits::Client, v2::params::JsonRpcParams, WsClient, WsClientBuilder};
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
}
