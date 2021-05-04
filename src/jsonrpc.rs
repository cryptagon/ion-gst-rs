use async_trait::async_trait;
use async_tungstenite::{gio::connect_async, tungstenite::Message};
use futures::prelude::*;

use super::{Error, SessionDescription, Signal};

pub struct JsonRPCSignaler {}

#[async_trait(?Send)]
impl Signal for JsonRPCSignaler {
    async fn open(url: String) -> Result<(), Error> {
        let (mut stream, _) = connect_async(url).await?;



        let (mut tx, mut rx) = stream.split();

        glib::MainContext::default().spawn(async move {
            println!("rx task spawned");

            while let Ok(msg) = rx.next().await.unwrap() {
                println!("got message: {}", msg);
            }


        });


        loop {
            let text = r#"{"method":"ping"}"#;
            println!("Sending: \"{}\"", text);
            tx.send(Message::text(text)).await?;

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
