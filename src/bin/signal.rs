use glib;
use gst::prelude::*;
use ion_gst_rs::jsonrpc::JsonRPCSignaler;
use ion_gst_rs::{Client, SessionDescription, Signal, TrickleCandidate};
use log::*;

use enclose::enc;

async fn run() -> Result<(), anyhow::Error> {
    pretty_env_logger::init();
    log::set_max_level(LevelFilter::Trace);
    gst::init()?;

    let rpc = JsonRPCSignaler::new("ws://localhost:7000/session/test");
    let pipeline = gst::parse_launch(
        "
        webrtcbin name=subscriber 
        webrtcbin name=publisher
        videotestsrc is-live=true ! 
        video/x-raw,width=640,height=480 !
        queue ! 
        x264enc speed-preset=ultrafast !
        queue !
        video/x-h264,profile=baseline ! 
        h264parse config-interval=-1 ! 
        rtph264pay ! application/x-rtp,clock-rate=90000,media=video,encoding=H264 ! publisher. 
        ",
    )?
    .downcast::<gst::Pipeline>()
    .unwrap();

    let mut client = Client::new(rpc, pipeline.clone(), "publisher", "subscriber");

    client
        .subscriber
        .connect_pad_added(enc!( (pipeline) move |_webrtc, subscriber_pad| {
            debug!("pad added!");
            let decodebin =
                gst::ElementFactory::make("decodebin", None).expect("could not make decodebin");

            decodebin.connect_pad_added(enc!( (pipeline) move |_webrtc, decoded_pad| {
                let caps = decoded_pad.get_current_caps().unwrap();
                let name = caps.get_structure(0).unwrap().get_name();

                let sink = if name.starts_with("video/") {
                    gst::parse_bin_from_description(
                        "queue ! glcolorscale ! glimagesink ",
                        true,
                    ).unwrap()
                } else if name.starts_with("audio/") {
                    gst::parse_bin_from_description(
                        "queue ! audioconvert ! audioresample ! autoaudiosink",
                        true,
                    ).unwrap()
                } else {
                    println!("Unknown pad {:?}, ignoring", decoded_pad);
                    return
                };

                pipeline.add(&sink).unwrap();
                sink.sync_state_with_parent().unwrap();

                let sinkpad = sink.get_static_pad("sink").unwrap();
                decoded_pad.link(&sinkpad).unwrap();
            }));

            pipeline.add(&decodebin).unwrap();
            decodebin.sync_state_with_parent().unwrap();
            let sink = decodebin.get_static_pad("sink").unwrap();
            subscriber_pad.link(&sink).unwrap();
        }));

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

pub fn main() -> Result<(), anyhow::Error> {
    ion_gst_rs::macos::run(|| {
        let main_context = glib::MainContext::default();
        main_context.block_on(run())
    })
}
