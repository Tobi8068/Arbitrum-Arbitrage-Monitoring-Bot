use ethers::providers::{Provider, Ws};
use std::sync::{Arc, RwLock};

use crate::websocket::{camelot_uniswap, camelot_pancakeswap, pancakeswap_uniswap};
use crate::shm::SharedMemoryManager;
use crate::ipc_handle::{BestTrade, StreamResults, handle_ipc_stream};

pub struct PairCategories {
    pub camelot_uniswap: Vec<(String, String, String, usize)>,      // (Camelot address, Uniswap address, Pair Name)
    pub camelot_pancakeswap: Vec<(String, String, String, usize)>,  // (Camelot address, PancakeSwap address, Pair Name)
    pub pancakeswap_uniswap: Vec<(String, String, String, usize)>,  // (PancakeSwap address, Uniswap address, Pair Name)
}

pub async fn handle_dex_streams(
    ws_provider: Arc<Provider<Ws>>,
    categories: PairCategories,
    shm_manager: Arc<RwLock<SharedMemoryManager>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Spawn tasks for each category
    let stream_results = Arc::new(StreamResults {
        best_trade: Arc::new(RwLock::new(BestTrade {
            profit_usdc: 0.0,
            buy_dex: [0u8; 20],
            buy_token_in: [0u8; 20],
            buy_token_out: [0u8; 20],
            buy_fee: 0,
            buy_amount: [0u8; 32],
            sell_dex: [0u8; 20],
            sell_token_in: [0u8; 20],
            sell_token_out: [0u8; 20],
            sell_fee: 0,
            sell_amount: [0u8; 32],
        }))
    });
    let camelot_uni_handle: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> = {
        let provider = ws_provider.clone();
        let shm = shm_manager.clone();
        let results = stream_results.clone();
        tokio::spawn(async move {
            match camelot_uniswap::monitor_pairs(
                provider,
                categories.camelot_uniswap,
                shm,
                results
            ).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("Error in camelot_uniswap monitor: {}", e);
                    Ok(())
                }
            }
        })
    };
    

    let camelot_pancake_handle: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> = {
        let provider = ws_provider.clone();
        let shm = shm_manager.clone();
        let results = stream_results.clone();
        tokio::spawn(async move {
            match camelot_pancakeswap::monitor_pairs(
                provider,
                categories.camelot_pancakeswap,
                shm,
                results
            ).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("Error in camelot_pancakeswap monitor: {}", e);
                    Ok(()) // Convert the error to () to avoid Send issues
                }
            }
        })
    };

    let pancake_uni_handle: tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> = {
        let provider = ws_provider.clone();
        let shm = shm_manager.clone();
        let results = stream_results.clone();
        tokio::spawn(async move {
            match pancakeswap_uniswap::monitor_pairs(
                provider,
                categories.pancakeswap_uniswap,
                shm,
                results
            ).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    eprintln!("Error in pancakeswap_uniswap monitor: {}", e);
                    Ok(()) // Convert the error to () to avoid Send issues
                }
            }
        })
    };

    let ipc_handle = {
        let results = stream_results.clone();
        tokio::spawn(async move {
            handle_ipc_stream(results).await
        })
    };

    // Wait for all tasks
    let _ = tokio::try_join!(
        camelot_uni_handle,
        camelot_pancake_handle, 
        pancake_uni_handle,
        ipc_handle
    )?;

    Ok(())
}