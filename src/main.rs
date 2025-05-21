use std::sync::{Arc, RwLock};
use dotenv::dotenv;

mod config;
mod exchange;
mod websocket;
mod shm;
mod trade;
mod utils;
mod ipc_handle;
use websocket::create_ws_provider;
use websocket::stream_handler::handle_dex_streams;
use shm::SharedMemoryManager;

async fn init() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    // Initialize shared memory (10MB should be enough for pool data)
    let ws_provider = create_ws_provider().await?;
    let (categories, total_pools) = websocket::load_pair_categories()?;
    let shm_manager = SharedMemoryManager::new("/tmp/pool_data.shm", total_pools)?;
    let shm_manager = Arc::new(RwLock::new(shm_manager));
    handle_dex_streams(ws_provider, categories, shm_manager).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = init().await {
        eprintln!("Error during initialization: {}", e);
        std::process::exit(1);
    }
}