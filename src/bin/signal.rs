use glib;
use gst::prelude::*;
use ion_gst_rs::jsonrpc::JsonRPCSignaler;
use ion_gst_rs::{Client, SessionDescription, Signal, TrickleCandidate};
use log::LevelFilter;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = JsonRPCSignaler::new("ws://localhost:7000/session/test");

    //let pipeline = gst::Pipeline::new(Some("test"));
    let pipeline = gst::parse_launch(
        //let src_bin = gst::parse_bin_from_description(
        "
        webrtcbin name=subscriber ! fakesink
        webrtcbin name=publisher
        videotestsrc is-live=true ! 
        video/x-raw,width=640,height=480 !
        queue ! 
        x264enc !
        queue !
        video/x-h264,profile=baseline ! 
        h264parse config-interval=-1 ! 
        rtph264pay ! application/x-rtp,clock-rate=90000,media=video,encoding=H264 ! publisher. 
        ",
    )?;

    let mut client = Client::new(
        rpc,
        pipeline.clone().downcast::<gst::Pipeline>().unwrap(),
        "publisher",
        "subscriber",
    );

    pipeline.set_state(gst::State::Playing)?;

    //@todo maybe use the gst_bus to trigger the client.join instead of this sleep hack
    // we need to wait for the ssrc caps event to arrive at webrtcbin before calling create-offer
    std::thread::sleep_ms(2000);

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
