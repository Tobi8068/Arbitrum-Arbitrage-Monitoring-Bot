use ethers::prelude::*;
use std::sync::Arc;
use crate::config::WETH_ADDRESS;

pub async fn simulate_pancake_trade_with_slippage<M: Middleware + 'static>(
    mut token_in: Address,
    mut token_in_decimals: u8,
    mut token_out: Address,
    mut token_out_decimals: u8,
    amount_in: f64,
    fee: u32,
    direction: &str,
    provider: Arc<M>,
) -> Result<f64, Box<dyn std::error::Error + Send + Sync>> {
    let client = Arc::new(provider.clone());
    let quoter_address: Address = "0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997".parse()?;
    abigen!(
        QuoterV3,
        "./src/abis/QuoterV3.json",
    );
    match direction {
        "BUY" => {
            if hex::encode(token_in.as_bytes()) != WETH_ADDRESS.to_lowercase() {
                // Swap tokens if token_in is not WETH
                std::mem::swap(&mut token_in, &mut token_out);
                std::mem::swap(&mut token_in_decimals, &mut token_out_decimals);
            }
        },
        "SELL" => {
            if hex::encode(token_in.as_bytes()) == WETH_ADDRESS.to_lowercase() {
                // Swap tokens if token_in is WETH
                std::mem::swap(&mut token_in, &mut token_out);
                std::mem::swap(&mut token_in_decimals, &mut token_out_decimals);
            }
        },
        _ => return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Direction must be either 'BUY' or 'SELL'"
        ))),
    }

    let quoter: QuoterV3<Arc<M>> = QuoterV3::new(quoter_address, client.clone());
    // Simulate quote
    // let sqrt_price_limit_x96 = U256::zero();
    let sqrt_price_limit_x96 = get_price_limit(&token_in, &token_out);
    let adjusted_amount_in_unit = U256::from((1.0 * (10u64.pow(token_in_decimals as u32) as f64)) as u64);
    // Create the params struct for the quote
    let params_input = QuoteExactInputSingleParams {
        token_in,
        token_out,
        amount_in: adjusted_amount_in_unit,
        fee: fee as u32,
        sqrt_price_limit_x96,
    };
    let params_output = QuoteExactInputSingleParams {
        token_out,
        token_in,
        amount_in: adjusted_amount_in_unit,
        fee: fee as u32,
        sqrt_price_limit_x96,
    };
    // Call the quoter with the struct

    let amount_out_unit = match direction {
        "BUY" => {
            let (amount_out, _, _, _) = match quoter.quote_exact_input_single(params_input).call().await {
                Ok(quote_result) => quote_result,
                Err(e) => {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to get quote from PancakeSwap Buy: {}", e)
                    )));
                }
            };
            amount_out
        },
        "SELL" => {
            let (amount_out, _, _, _) = match quoter.quote_exact_input_single(params_output).call().await {
                Ok(quote_result) => quote_result,
                Err(e) => {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to get quote from PancakeSwap Sell: {}", e)
                    )));
                }
            };
            amount_out
        },
        _ => unreachable!()
    };

    let amount_with_decimals = amount_in * amount_out_unit.as_u128() as f64 / (10u128.pow(token_out_decimals as u32) as f64);
    Ok(amount_with_decimals)
}

pub fn get_price_limit(token_address_from: &Address, token_address_to: &Address) -> U256 {
    if *token_address_from < *token_address_to {
        U256::from(4295128740u64)
    } else {
        U256::from_str_radix("1461446703485210103287273052203988822378723970341", 10).unwrap()
    }
}