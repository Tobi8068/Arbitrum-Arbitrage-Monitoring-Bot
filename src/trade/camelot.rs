use crate::config::WETH_ADDRESS;
use ethers::prelude::*;
use std::sync::Arc;

pub async fn simulate_camelot_trade_with_slippage<M: Middleware + 'static>(
    mut token_in: Address,
    mut token_in_decimals: u8,
    mut token_out: Address,
    mut token_out_decimals: u8,
    amount_in: f64,
    direction: &str,
    provider: Arc<M>,
) -> Result<(f64, u32), Box<dyn std::error::Error + Send + Sync>> {
    let client = Arc::new(provider.clone());
    let quoter_address: Address = "0x0Fc73040b26E9bC8514fA028D998E73A254Fa76E".parse()?;
    abigen!(CamelotQuoter, "./src/abis/CamelotQuoter.json",);

    match direction {
        "BUY" => {
            if hex::encode(token_in.as_bytes()) != WETH_ADDRESS.to_lowercase() {
                // Swap tokens if token_in is not WETH
                std::mem::swap(&mut token_in, &mut token_out);
                std::mem::swap(&mut token_in_decimals, &mut token_out_decimals);
            }
        }
        "SELL" => {
            if hex::encode(token_in.as_bytes()) == WETH_ADDRESS.to_lowercase() {
                // Swap tokens if token_in is WETH
                std::mem::swap(&mut token_in, &mut token_out);
                std::mem::swap(&mut token_in_decimals, &mut token_out_decimals);
            }
        }
        _ => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Direction must be either 'BUY' or 'SELL'",
            )));
        }
    }

    let quoter: CamelotQuoter<Arc<M>> = CamelotQuoter::new(quoter_address, client.clone());

    // Simulate quote
    let sqrt_price_limit_x96 = U256::zero();
    let adjusted_amount_in_unit =
        U256::from((1.0 * (10u64.pow(token_in_decimals as u32) as f64)) as u64);

    let (amount_out_unit, fee) = match quoter
        .quote_exact_input_single(
            token_in,
            token_out,
            adjusted_amount_in_unit,
            sqrt_price_limit_x96,
        )
        .call()
        .await
    {
        Ok((amount_out, fee)) => (amount_out, fee),
        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get quote from Camelot: {}", e),
            )));
        }
    };

    let amount_with_decimals = amount_in * amount_out_unit.as_u128() as f64
        / (10u128.pow(token_out_decimals as u32) as f64);

    Ok((amount_with_decimals, fee.into()))
}
