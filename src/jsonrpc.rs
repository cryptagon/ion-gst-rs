use super::{Error, SessionDescription, Signal};
use async_trait::async_trait;
use async_tungstenite::{gio::connect_async, tungstenite::Message};
use futures::prelude::*;
use jsonrpsee::ws_client::{traits::Client, v2::params::JsonRpcParams, WsClient, WsClientBuilder};
use std::borrow::Cow;
use std::net::SocketAddr;
use url::Url;

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

        //        loop {
        //            println!("ping");
        //            let response: String = client.request("ping", JsonRpcParams::NoParams).await?;
        //            println!("got response: {}", response);
        //            async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        //        }

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
        }
        Ok(())
    }

    async fn join(&self, sid: String, offer: SessionDescription) -> Result<(), Error> {
        Ok(())
    }
}
