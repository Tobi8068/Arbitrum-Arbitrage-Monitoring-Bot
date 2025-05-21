use serde::{Deserialize, Serialize};
use alloy_primitives::U160;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeInfo {
    #[serde(rename = "PairAddress")]
    pub pair_address: String,
    #[serde(rename = "Liquidity")]
    pub liquidity: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PairData {
    #[serde(rename = "Pair")]
    pub pair: String,
    #[serde(rename = "UniSwap")]
    pub uni_swap: Option<ExchangeInfo>,
    #[serde(rename = "Camelot")]
    pub camelot: Option<ExchangeInfo>,
    #[serde(rename = "PancakeSwap")]
    pub pancake_swap: Option<ExchangeInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    dex: [u8; 20],
    token_from: [u8; 20],
    token_to: [u8; 20],
    fee: u32,
    amount: [u8; 32],
    price: U160,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeResult {
    pub buy_dex: String,
    pub buy_address: String,
    pub input_amount: f64,
    pub sell_dex: String, 
    pub sell_address: String,
    pub output_amount: f64,
    pub profit: f64
}