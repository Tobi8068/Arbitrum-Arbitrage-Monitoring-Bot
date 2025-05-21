use std::env;
use core::time::Duration;

pub const CHAIN_ID: u64 = 42161;
pub const UNISWAP_V3_FACTORY_ADDRESS: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
pub const IS_LOGGING_ENABLED: bool = false;

pub fn is_simulation_logging_enabled() -> bool {
    env::var("IS_SIMULATION_LOGGING_ENABLED")
        .map(|val| val.to_lowercase() == "true")
        .unwrap_or(false)
}

pub const WETH_ADDRESS: &str = "82aF49447D8a07e3bd95BD0d56f35241523fBab1";

pub const UNISWAP_V3_SWAP_ROUTER_ADDRESS: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
pub const PANCAKESWAP_V3_SWAP_ROUTER_ADDRESS: &str = "0x1b81D678ffb9C0263b24A97847620C99d213eB14";
pub const CAMELOT_V3_SWAP_ROUTER_ADDRESS: &str = "0x1F721E2E82F6676FCE4eA07A5958cF098D339e18";

pub const IPC_CYCLE_TIME: Duration = Duration::from_millis(200);

pub const TIER0_PRICE_DIFF: f64 = 0.035;
pub const TIER1_PRICE_DIFF: f64 = 0.016;
pub const TIER2_PRICE_DIFF: f64 = 0.014;
pub const TIER3_PRICE_DIFF: f64 = 0.011;
pub const TIER4_PRICE_DIFF: f64 = 0.004;

pub fn get_trade_config(tier: u8) -> (f64, f64, u32) {
    let tier_suffix = format!("_TIER{}", tier);
    
    let start_amount = env::var(format!("START_AMOUNT{}", tier_suffix))
        .unwrap_or_else(|_| match tier {
            0 => "50".to_string(),
            1 => "100".to_string(),
            2 => "300".to_string(),
            3 => "500".to_string(),
            4 => "1000".to_string(),
            _ => "1".to_string(),
        })
        .parse::<f64>()
        .unwrap_or(1.0);
    
    let step = env::var(format!("STEP{}", tier_suffix))
        .unwrap_or_else(|_| match tier {
            0 => "50".to_string(),
            1 => "100".to_string(),
            2 => "100".to_string(),
            3 => "150".to_string(),
            4 => "1250".to_string(),
            _ => "1".to_string(),
        })
        .parse::<f64>()
        .unwrap_or(1.0);
    
    let step_number = env::var(format!("STEP_NUMBER{}", tier_suffix))
        .unwrap_or_else(|_| match tier {
            0 => "4".to_string(),
            1 => "5".to_string(),
            2 => "5".to_string(),
            3 => "4".to_string(),
            4 => "4".to_string(),
            _ => "4".to_string(),
        })
        .parse::<u32>()
        .unwrap_or(4);

    (start_amount, step, step_number)
}
