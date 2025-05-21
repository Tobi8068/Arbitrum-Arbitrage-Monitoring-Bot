use ethers::prelude::*;
use std::sync::Arc;
use crate::config::WETH_ADDRESS;

pub async fn simulate_uniswap_trade_with_slippage<M: Middleware + 'static>(
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
    let quoter_address: Address = "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6".parse()?;
    abigen!(
        UniswapQuoter,
        r#"[
            function quoteExactInputSingle(address tokenIn, address tokenOut, uint24 fee, uint256 amountIn, uint160 sqrtPriceLimitX96) external returns (uint256 amountOut)
        ]"#,
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

    let quoter: UniswapQuoter<Arc<M>> = UniswapQuoter::new(quoter_address, client.clone());

    // Simulate quote
    let sqrt_price_limit_x96 = U256::zero();
    let adjusted_amount_in_unit = U256::from((1.0 * (10u64.pow(token_in_decimals as u32) as f64)) as u64);

    let amount_out_unit = quoter
        .quote_exact_input_single(token_in, token_out, fee, adjusted_amount_in_unit, sqrt_price_limit_x96)
        .call()
        .await?;

    let amount_with_decimals = amount_in * amount_out_unit.as_u128() as f64 / (10u128.pow(token_out_decimals as u32) as f64);
    Ok(amount_with_decimals)
}
