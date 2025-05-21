pub mod camelot_uniswap;
pub mod camelot_pancakeswap;
pub mod pancakeswap_uniswap;
pub mod stream_handler;
pub mod categorize_pairs;

use ethers::providers::{Provider, Ws};
use std::{sync::Arc, fs::File, io::BufReader};
use crate::exchange::model::PairData;
use crate::websocket::stream_handler::PairCategories;

pub async fn create_ws_provider() -> Result<Arc<Provider<Ws>>, Box<dyn std::error::Error>> {
    let ws_url = std::env::var("WS_RPC_URL")
        .expect("RPC_URL must be set in environment variables");
    let ws = Ws::connect(ws_url).await?;
    let provider = Provider::new(ws);
    
    Ok(Arc::new(provider))
}

pub fn load_pair_categories() -> std::io::Result<(PairCategories, usize)> {
    let file = File::open("src/data.json")?;
    let reader = BufReader::new(file);
    let pairs: Vec<PairData> = serde_json::from_reader(reader)?;
    
    let mut current_index = 0;
    let mut categories = PairCategories {
        camelot_uniswap: Vec::new(),
        camelot_pancakeswap: Vec::new(),
        pancakeswap_uniswap: Vec::new(),
    };

    for pair in pairs {
        let pair_name = pair.pair.clone();
        // Check for Camelot + Uniswap pairs
        if let (Some(camelot), Some(uniswap)) = (&pair.camelot, &pair.uni_swap) {
            categories.camelot_uniswap.push((
                camelot.pair_address.clone(),
                uniswap.pair_address.clone(),
                pair_name.clone(),
                current_index
            ));
            current_index += 2;
        }
        
        // Check for Camelot + PancakeSwap pairs
        if let (Some(camelot), Some(pancake)) = (&pair.camelot, &pair.pancake_swap) {
            categories.camelot_pancakeswap.push((
                camelot.pair_address.clone(),
                pancake.pair_address.clone(),
                pair_name.clone(),
                current_index
            ));
            current_index += 2;
        }
        
        // Check for PancakeSwap + Uniswap pairs
        if let (Some(pancake), Some(uniswap)) = (&pair.pancake_swap, &pair.uni_swap) {
            categories.pancakeswap_uniswap.push((
                pancake.pair_address.clone(),
                uniswap.pair_address.clone(),
                pair_name.clone(),
                current_index
            ));
            current_index += 2;
        }
    }
    
    // Print summary of loaded pairs
    println!("Loaded {} Camelot-Uniswap pairs --> Will be allocated to WebSocket 1", categories.camelot_uniswap.len());
    println!("Loaded {} Camelot-PancakeSwap pairs --> Will be allocated to WebSocket 2", categories.camelot_pancakeswap.len());
    println!("Loaded {} PancakeSwap-Uniswap pairs --> Will be allocated to WebSocket 3", categories.pancakeswap_uniswap.len());

    Ok((categories, current_index))
}
