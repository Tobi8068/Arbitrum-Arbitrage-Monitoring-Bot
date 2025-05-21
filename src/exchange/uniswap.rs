use ethers::prelude::{abigen, Address as EthersAddress, U256};
use alloy_primitives::{Address as AlloyAddress, hex};
use ethers::providers::Middleware;
use uniswap_v3_sdk::prelude::{FeeAmount, NoTickDataProvider, Pool, get_pool};
use std::sync::Arc;
use crate::config::{CHAIN_ID, UNISWAP_V3_FACTORY_ADDRESS, WETH_ADDRESS};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UniswapPoolInfo {
    pub pool: Pool<NoTickDataProvider>,
    pub price: f64,
    pub liquidity: u128,
    pub tick: i32,
    pub token0: EthersAddress,
    pub token1: EthersAddress,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub token0_amount: U256,
    pub token1_amount: U256,
    pub token0_amount_adjusted: f64,
    pub token1_amount_adjusted: f64,
    #[allow(dead_code)]
    pub pair_address: String,
    pub fee: u32,
}

pub async fn get_uniswap_info<M: Middleware + 'static>(
    uni_v3_pool_address: String,
    provider: Arc<M>,
) -> Result<UniswapPoolInfo, Box<dyn std::error::Error>> {
    let client = Arc::new(provider.clone());
    abigen!(
        UniswapV3Pool,
        "./src/abis/UniswapV3Pool.json",
    );
    
    abigen!(
        ERC20,
        r#"[
            function decimals() external view returns (uint8)
            function balanceOf(address) view returns (uint256)
        ]"#,
    );
    
    let v3_pool = UniswapV3Pool::new(
        uni_v3_pool_address.parse::<EthersAddress>()?,
        client.clone()
    );
    
    let token0 = v3_pool.token_0().call().await?;
    let token1 = v3_pool.token_1().call().await?;
    
    // Get token decimals
    let token0_contract = ERC20::new(token0, client.clone());
    let token1_contract = ERC20::new(token1, client.clone());
    
    let token0_decimals_bytes = token0_contract.decimals().call().await?;
    let token1_decimals_bytes = token1_contract.decimals().call().await?;
    
    let token0_decimals = token0_decimals_bytes.to_string().parse::<u8>().unwrap_or(18);
    let token1_decimals = token1_decimals_bytes.to_string().parse::<u8>().unwrap_or(18);

    // Calculation Liquidity using ERC20 Contract
    let pair_address = uni_v3_pool_address.parse()?;

    let amount0 = token0_contract.balance_of(pair_address).call().await?;
    let amount1 = token1_contract.balance_of(pair_address).call().await?;

    let amount0_f64 = amount0.to_string().parse::<f64>().unwrap_or(0.0);
    let amount1_f64 = amount1.to_string().parse::<f64>().unwrap_or(0.0);

    let amount0_adjusted = amount0_f64 / 10.0f64.powi(token0_decimals as i32);
    let amount1_adjusted = amount1_f64 / 10.0f64.powi(token1_decimals as i32);

    let fee_amount_bytes = v3_pool.fee().call().await?;
    let fee_amount = match fee_amount_bytes {
        100 => FeeAmount::LOWEST,
        500 => FeeAmount::LOW,
        3000 => FeeAmount::MEDIUM,
        10000 => FeeAmount::HIGH,
        _ => panic!("invalid fee amount")
    };

    // get pool
    let factory_address_str = UNISWAP_V3_FACTORY_ADDRESS;

    let pool = get_pool(
        CHAIN_ID,
        factory_address_str.parse::<AlloyAddress>()?,
        token0.to_fixed_bytes().into(),
        token1.to_fixed_bytes().into(),
        fee_amount,
        Arc::new(provider),
        None
    ).await?;

    let tick = pool.tick_current;

    // Check if token0 is WETH
    let token0_is_weth = hex::encode(token0.as_bytes()) == WETH_ADDRESS.to_lowercase();

    let decimal_adjustment = if token0_is_weth {
        10_u128.pow(token0_decimals as u32 - token1_decimals as u32)
    } else {
        10_u128.pow(token1_decimals as u32 - token0_decimals as u32)
    };

    let adjustment_factor_f64 = decimal_adjustment.to_string().parse::<f64>().unwrap_or(1.0);

    let sqrt_price_x96 = pool.sqrt_ratio_x96;
    let price_x192 = sqrt_price_x96 * sqrt_price_x96;

    let price_x192_f64 = price_x192.to_string().parse::<f64>().unwrap_or(0.0);

    let two_pow_192_f64 = 2.0f64.powi(192);
    let mut price_ratio = price_x192_f64 / two_pow_192_f64;

    // If token0 is not WETH, invert the price
    if token0_is_weth {
        price_ratio = 1.0 / price_ratio / adjustment_factor_f64;
    } else {
        price_ratio = price_ratio / adjustment_factor_f64;
    }

    let liquidity = if token0_is_weth {
        amount0_adjusted * 1.0 + amount1_adjusted * price_ratio
    } else {
        amount0_adjusted * price_ratio + amount1_adjusted * 1.0
    };

    Ok(UniswapPoolInfo {
        pool,
        price: price_ratio,
        liquidity: liquidity as u128,
        tick,
        token0,
        token1,
        token0_decimals,
        token1_decimals,
        token0_amount: amount0,
        token1_amount: amount1,
        token0_amount_adjusted: amount0_adjusted,
        token1_amount_adjusted: amount1_adjusted,
        pair_address: uni_v3_pool_address,
        fee: fee_amount_bytes.to_string().parse::<u32>().unwrap_or(0),
    })
}

pub async fn uniswap_weth_to_usdc<M: Middleware + 'static> (price: f64, liquidity: f64, provider: Arc<M>) -> Result<(f64, f64, f64), Box<dyn std::error::Error>> {
    let weth_usdc_pair = "0xC6962004f452bE9203591991D15f6b388e09E8D0".to_lowercase();

    let weth_usdc_pair_info = get_uniswap_info(weth_usdc_pair.to_string(), provider).await?;
    let weth_usdc_price = 1.0 / weth_usdc_pair_info.price;
    let usdc_price = price * weth_usdc_price;
    let usdc_liquidity = liquidity * weth_usdc_price;
    Ok((usdc_price, usdc_liquidity, weth_usdc_price))
}