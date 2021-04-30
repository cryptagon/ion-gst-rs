use glib;
use ion_cluster_gst::jsonrpc::JsonRPCSignaler;
use ion_cluster_gst::Signal;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let j = JsonRPCSignaler {};
    JsonRPCSignaler::open("wss://sfu.dogfood.tandem.chat".to_string()).await?;

    Ok(())
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the default main context and run our async function on it
    let main_context = glib::MainContext::default();
    main_context.block_on(run())
}
