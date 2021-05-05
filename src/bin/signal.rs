use glib;
use ion_gst_rs::jsonrpc::JsonRPCSignaler;
use ion_gst_rs::{SessionDescription, Signal, TrickleCandidate};

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut rpc = JsonRPCSignaler::new();

    rpc.open("ws://localhost:7000/session/test".to_string())
        .await?;

    loop {
        rpc.ping().await?;
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
    }
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the default main context and run our async function on it
    let main_context = glib::MainContext::default();
    main_context.block_on(run())
}
