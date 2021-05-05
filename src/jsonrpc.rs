use async_trait::async_trait;
use async_tungstenite::{gio::connect_async, tungstenite::Message};
use futures::prelude::*;

use url::{Url};
use super::{Error, SessionDescription, Signal};

use jsonrpsee::{
	ws_client::{traits::Client, v2::params::JsonRpcParams, WsClientBuilder},
	ws_server::WsServer,
};
use std::net::SocketAddr;

use std::borrow::Cow;
pub struct JsonRPCSignaler {

}

#[async_trait(?Send)]
impl Signal for JsonRPCSignaler {
    async fn open(url: String) -> Result<(), Error> {

        let u = Url::parse(&url).unwrap();



        let client = WsClientBuilder::default().handshake_url(Cow::Borrowed(&u.path())).build(&url).await?;


        loop {
            println!("ping");
            let response: String = client.request("ping", JsonRpcParams::NoParams).await?;
            println!("got response: {}", response);
            async_std::task::sleep(std::time::Duration::from_secs(1)).await; 
        }


        Ok(())
    }

    async fn close() -> Result<(), Error> {
        Ok(())
    }
    async fn ping() -> Result<(), Error> {
        Ok(())
    }

    async fn join(sid: String, offer: SessionDescription) -> Result<(), Error> {
        Ok(())
    }
}
