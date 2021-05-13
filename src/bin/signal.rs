use glib;
use gst::prelude::*;
use ion_gst_rs::jsonrpc::JsonRPCSignaler;
use ion_gst_rs::{Client, SessionDescription, Signal, TrickleCandidate};
use log::LevelFilter;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = JsonRPCSignaler::new("ws://localhost:7000/session/test");

    let pipeline = gst::Pipeline::new(Some("test"));
    let fakesink = gst::ElementFactory::make("fakesink", None)?;
    pipeline.add(&fakesink)?;

    let src_bin = gst::parse_bin_from_description(
        "videotestsrc is-live=true ! 
        queue ! 
        vtenc_h264 realtime=true allow-frame-reordering=false max-keyframe-interval=60 ! 
        video/x-h264, profile=baseline ! 
        h264parse config-interval=1 ! 
        rtph264pay pt=96 ! application/x-rtp,clock-rate=90000 ! queue",
        true,
    )?;

    pipeline.add(&src_bin)?;
    pipeline.set_state(gst::State::Ready)?;

    let mut client = Client::new(rpc, pipeline.clone());

    src_bin.sync_state_with_parent()?;
    src_bin.link(&client.publisher)?;

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
