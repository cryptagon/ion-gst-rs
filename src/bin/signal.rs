use glib;
use ion_gst_rs::jsonrpc::JsonRPCSignaler;
use ion_gst_rs::{SessionDescription, TrickleCandidate, Signal};



async fn run() -> Result<(), Box<dyn std::error::Error>> {
    JsonRPCSignaler::open("ws://localhost:7000/session/test".to_string()).await?;


    Ok(())
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {

    // Get the default main context and run our async function on it
    let main_context = glib::MainContext::default();
    main_context.block_on(run())
}
