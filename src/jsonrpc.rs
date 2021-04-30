use async_trait::async_trait;
use async_tungstenite::{gio::connect_async, tungstenite::Message};
use futures::prelude::*;

use super::{Error, SessionDescription, Signal};

pub struct JsonRPCSignaler {}

#[async_trait(?Send)]
impl Signal for JsonRPCSignaler {
    async fn open(url: String) -> Result<(), Error> {
        let (mut stream, _) = connect_async(url).await?;

        let text = "Hello, World!";

        println!("Sending: \"{}\"", text);
        stream.send(Message::text(text)).await?;

        let msg = stream
            .next()
            .await
            .ok_or("didn't receive anything")
            .unwrap();

        println!("Received: {:?}", msg);

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
