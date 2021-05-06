use glib;
use gst::prelude::*;
use ion_gst_rs::jsonrpc::JsonRPCSignaler;
use ion_gst_rs::{Client, SessionDescription, Signal, TrickleCandidate};
use log::LevelFilter;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut rpc = JsonRPCSignaler::new();

    let pipeline = gst::Pipeline::new(Some("test"));
    let fakesink = gst::ElementFactory::make("fakesink", None)?;
    pipeline.add(&fakesink)?;

    rpc.open("ws://localhost:7000/session/test".to_string())
        .await?;
    let client = Client::new(rpc, pipeline);
    client.join("test".to_string()).await?;

    loop {
        client.ping().await?;
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
    }
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::init();
    log::set_max_level(LevelFilter::Trace);
    gst::init()?;
    // Get the default main context and run our async function on it
    let main_context = glib::MainContext::default();
    main_context.block_on(run())
}
