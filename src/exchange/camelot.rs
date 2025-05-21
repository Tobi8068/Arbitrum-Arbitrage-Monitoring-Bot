use alloy_primitives::hex;
use ethers::prelude::{abigen, Address as EthersAddress, U256};
use ethers::providers::Middleware;
use std::sync::Arc;

use crate::config::WETH_ADDRESS;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CamelotPoolInfo {
    pub token0: EthersAddress,
    pub token1: EthersAddress,
    pub token0_decimals: u8,
    pub token1_decimals: u8,
    pub token0_amount: U256,
    pub token1_amount: U256,
    pub token0_amount_adjusted: f64,
    pub token1_amount_adjusted: f64,
    pub pair_address: String,
    pub liquidity: u128,
    pub price: f64,
    pub tick: i32,
}

pub async fn get_camelot_info<M: Middleware + 'static>(
    camelot_pool_address: String,
    provider: Arc<M>,
) -> Result<CamelotPoolInfo, Box<dyn std::error::Error>> {
    let client = Arc::new(provider.clone());

    // Generate bindings for Camelot pair contract
    abigen!(
        CamelotPair,
        "./src/abis/CamelotPool.json",
    );
    
    abigen!(
        ERC20,
        r#"[
            function decimals() external view returns (uint8)
            function balanceOf(address) view returns (uint256)
        ]"#,
    );
    
    // Convert address string to ethers Address
    let pool_address: EthersAddress = match camelot_pool_address.parse() {
        Ok(addr) => addr,
        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput, 
                format!("Invalid pool address format: {}", e)
            )));
        }
    };
    
    
    // Create contract instance
    let camelot_pool = CamelotPair::new(pool_address, client.clone());

    // Get token addresses - add error handling
    let token0 = match camelot_pool.token_0().call().await {
        Ok(addr) => addr,
        Err(e) => {
            println!("Failed to call token_0(): {}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("token_0() call failed: {}", e)
            )));
        }
    };

    // Check if token0 is WETH
    let token0_is_weth = hex::encode(token0.as_bytes()) == WETH_ADDRESS.to_lowercase();
    
    let token1 = match camelot_pool.token_1().call().await {
        Ok(addr) => addr,
        Err(e) => {
            println!("Failed to call token_1(): {}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("token_1() call failed: {}", e)
            )));
        }
    };
    
    // Get token decimals
    let token0_contract = ERC20::new(token0, client.clone());
    let token1_contract = ERC20::new(token1, client.clone());
    
    let token0_decimals_bytes = match token0_contract.decimals().call().await {
        Ok(dec) => dec,
        Err(e) => {
            println!("Failed to get token0 decimals: {}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("token0 decimals call failed: {}", e)
            )));
        }
    };
    
    let token1_decimals_bytes = match token1_contract.decimals().call().await {
        Ok(dec) => dec,
        Err(e) => {
            println!("Failed to get token1 decimals: {}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("token1 decimals call failed: {}", e)
            )));
        }
    };
    
    // Get global state (includes price, tick, and fees)
    let state = match camelot_pool.global_state().call().await {
        Ok(s) => s,
        Err(e) => {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other, 
                format!("global_state() call failed: {}", e)
            )));
        }
    };
    
    let sqrt_price_x96 = state.0;  // price
    let sqrt_price_f64 = sqrt_price_x96.to_string().parse::<f64>().unwrap_or(0.0);
    let price_x96_squared = sqrt_price_f64.powi(2);
    let two_pow_192 = 2.0f64.powi(192);

    let mut raw_price = price_x96_squared / two_pow_192;

    let token0_decimals = token0_decimals_bytes.to_string().parse::<u8>().unwrap_or(18);
    let token1_decimals = token1_decimals_bytes.to_string().parse::<u8>().unwrap_or(18);

    // Calculation Liquidity using ERC20 Contract

    let amount0 = token0_contract.balance_of(pool_address).call().await?;
    let amount1 = token1_contract.balance_of(pool_address).call().await?;

    let amount0_f64 = amount0.to_string().parse::<f64>().unwrap_or(0.0);
    let amount1_f64 = amount1.to_string().parse::<f64>().unwrap_or(0.0);

    let amount0_adjusted = amount0_f64 / 10.0f64.powi(token0_decimals as i32);
    let amount1_adjusted = amount1_f64 / 10.0f64.powi(token1_decimals as i32);

    let decimal_adjustment = if token0_is_weth {
        10_u128.pow(token0_decimals as u32 - token1_decimals as u32)
    } else {
        10_u128.pow(token1_decimals as u32 - token0_decimals as u32)
    };

    let adjustment_factor_f64 = decimal_adjustment.to_string().parse::<f64>().unwrap_or(1.0);

    if token0_is_weth {
        raw_price = 1.0 / raw_price / adjustment_factor_f64;
    } else {
        raw_price = raw_price / adjustment_factor_f64;
    }

    let liquidity = if token0_is_weth {
        amount0_adjusted * 1.0 + amount1_adjusted * raw_price
    } else {
        amount0_adjusted * raw_price + amount1_adjusted * 1.0
    };

    let tick = state.1;   // tick

    Ok(CamelotPoolInfo {
        token0,
        token1,
        token0_decimals,
        token1_decimals,
        token0_amount: amount0,
        token1_amount: amount1,
        token0_amount_adjusted: amount0_adjusted,
        token1_amount_adjusted: amount1_adjusted,
        pair_address: camelot_pool_address,
        liquidity: liquidity as u128,
        price: raw_price,
        tick: tick as i32,
    })
}

pub async fn camelot_weth_to_usdc<M: Middleware + 'static> (price: f64, liquidity: f64, provider: Arc<M>) -> Result<(f64, f64, f64), Box<dyn std::error::Error>> {
    let weth_usdc_pair = "0xb1026b8e7276e7ac75410f1fcbbe21796e8f7526".to_lowercase();

    let weth_usdc_pair_info = get_camelot_info(weth_usdc_pair.to_string(), provider).await?;
    let weth_usdc_price = 1.0 / weth_usdc_pair_info.price;
    let usdc_price = price * weth_usdc_price;
    let usdc_liquidity = liquidity * weth_usdc_price;
    Ok((usdc_price, usdc_liquidity, weth_usdc_price))
}