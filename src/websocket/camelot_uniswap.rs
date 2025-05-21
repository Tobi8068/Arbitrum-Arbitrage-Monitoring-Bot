use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use ethers::prelude::U256;
use ethers::providers::{Provider as EtherProvider, Ws};
use futures_util::StreamExt;
use std::fs::OpenOptions;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{
    // TIER0_PRICE_DIFF, TIER1_PRICE_DIFF, TIER2_PRICE_DIFF, TIER3_PRICE_DIFF, TIER4_PRICE_DIFF,
    CAMELOT_V3_SWAP_ROUTER_ADDRESS,
    IS_LOGGING_ENABLED,
    UNISWAP_V3_SWAP_ROUTER_ADDRESS,
    WETH_ADDRESS,
    get_trade_config,
    is_simulation_logging_enabled,
};
use crate::exchange::camelot::{camelot_weth_to_usdc, get_camelot_info};
use crate::exchange::uniswap::{get_uniswap_info, uniswap_weth_to_usdc};
use crate::ipc_handle::StreamResults;
use crate::shm::SharedMemoryManager;
use crate::trade::camelot::simulate_camelot_trade_with_slippage;
use crate::trade::uniswap::simulate_uniswap_trade_with_slippage;
use crate::utils::write_log::{log_fee_data, log_price_liquidity, log_simulation};
use crate::websocket::categorize_pairs::determine_tier;

pub async fn monitor_pairs(
    provider: Arc<EtherProvider<Ws>>,
    pairs: Vec<(String, String, String, usize)>,
    _shm_manager: Arc<RwLock<SharedMemoryManager>>,
    stream_results: Arc<StreamResults>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rpc_url = std::env::var("WS_RPC_URL").expect("WS_RPC_URL must be set");
    let ws = WsConnect::new(rpc_url);
    let provider_alloy = ProviderBuilder::new().on_ws(ws).await?;

    let is_logging = is_simulation_logging_enabled();

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("liquidity_price.log")?;

    // Subscribe to new blocks.
    let mut block_stream = provider_alloy
        .subscribe_blocks()
        .await
        .expect("failed to subscribe on new blocks")
        .into_stream();

    // let mut block_stream = provider.subscribe_blocks().await?;
    while let Some(block) = block_stream.next().await {
        println!(
            "---------------------------Latest block number: {} ----- Pair Length: {}",
            block.number,
            pairs.len()
        );
        // let block_number = block.number;
        // let timestamp = SystemTime::now()
        //     .duration_since(UNIX_EPOCH)
        //     .unwrap_or_default()
        //     .as_secs();
        let timestamp_duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp_ms =
            timestamp_duration.as_secs() * 1000 + timestamp_duration.subsec_millis() as u64;
        let ms_part = timestamp_ms % 1000;
        let tasks = pairs
            .iter()
            .map(|(camelot_addr, uni_addr, pair_name, _pool_index)| {
                let provider = provider.clone();
                let camelot_addr = camelot_addr.clone();
                let uni_addr = uni_addr.clone();
                let pair_name = pair_name.clone();
                let mut log_file = log_file.try_clone().unwrap();
                // let shm_manager = shm_manager.clone();
                let stream_results = stream_results.clone();

                async move {
                    // Get latest prices
                    let camelot_data =
                        match get_camelot_info(camelot_addr.to_string(), provider.clone()).await {
                            Ok(price) => price,
                            Err(_) => return Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()),
                        };
                    let uni_data =
                        match get_uniswap_info(uni_addr.to_string(), provider.clone()).await {
                            Ok(price) => price,
                            Err(_) => return Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()),
                        };
                    log_fee_data("Uniswap", &pair_name, &uni_addr, uni_data.fee).map_err(|e| {
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        )) as Box<dyn std::error::Error + Send + Sync>
                    })?;
                    let (camelot_usdc_price, camelot_usdc_liquidity, _) = camelot_weth_to_usdc(
                        camelot_data.price,
                        camelot_data.liquidity as f64,
                        provider.clone(),
                    )
                    .await
                    .map_err(|e| {
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            e.to_string(),
                        )) as Box<dyn std::error::Error + Send + Sync>
                    })?;
                    let (uniswap_usdc_price, uniswap_usdc_liquidity, weth_usdc) =
                        uniswap_weth_to_usdc(
                            uni_data.price,
                            uni_data.liquidity as f64,
                            provider.clone(),
                        )
                        .await
                        .map_err(|e| {
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                e.to_string(),
                            ))
                                as Box<dyn std::error::Error + Send + Sync>
                        })?;

                    // Rest of your existing logic here...
                    let min_liquidity = camelot_usdc_liquidity.min(uniswap_usdc_liquidity);

                    let max_price = camelot_usdc_price.max(uniswap_usdc_price);
                    let price_diff_pct =
                        ((camelot_usdc_price - uniswap_usdc_price) / max_price).abs();
                    let tier = determine_tier(min_liquidity);

                    let is_camelot = if camelot_usdc_price < uniswap_usdc_price {
                        true
                    } else {
                        false
                    };

                    if IS_LOGGING_ENABLED {
                        if let Err(e) = log_price_liquidity(
                            &mut log_file,
                            ms_part,
                            &pair_name,
                            &camelot_addr,
                            &uni_addr,
                            min_liquidity,
                            camelot_usdc_price,
                            uniswap_usdc_price,
                            price_diff_pct,
                            "Camelot",
                            "Uniswap",
                        ) {
                            eprintln!("Failed to log price and liquidity: {}", e);
                        }
                    }

                    if tier < 6 {
                        // if tier == 0 && price_diff_pct < TIER0_PRICE_DIFF {
                        //     return Ok(());
                        // } else if tier == 1 && price_diff_pct < TIER1_PRICE_DIFF {
                        //     return Ok(());
                        // } else if tier == 2 && price_diff_pct < TIER2_PRICE_DIFF {
                        //     return Ok(());
                        // } else if tier == 3 && price_diff_pct < TIER3_PRICE_DIFF {
                        //     return Ok(());
                        // } else if tier == 4 && price_diff_pct < TIER4_PRICE_DIFF {
                        //     return Ok(());
                        // }

                        let (start_amount, step, step_number) = get_trade_config(tier);

                        let test_amounts: Vec<f64> = (0..step_number)
                            .map(|i| start_amount + (step * i as f64))
                            .collect();
                        let mut current_best_profit = 0.0;
                        let mut best_trade_data = None;

                        for amount in test_amounts {
                            let amount_weth = amount / weth_usdc;
                            // Simulate trades in parallel
                            if !is_camelot {
                                let buy_result = simulate_uniswap_trade_with_slippage(
                                    uni_data.token0,
                                    uni_data.token0_decimals,
                                    uni_data.token1,
                                    uni_data.token1_decimals,
                                    amount_weth,
                                    uni_data.fee,
                                    "BUY",
                                    provider.clone(),
                                )
                                .await?;
                                let buy_result_weth = buy_result;
                                let sell_result = simulate_camelot_trade_with_slippage(
                                    camelot_data.token0,
                                    camelot_data.token0_decimals,
                                    camelot_data.token1,
                                    camelot_data.token1_decimals,
                                    buy_result_weth,
                                    "SELL",
                                    provider.clone(),
                                )
                                .await?;
                                let sell_result_weth = sell_result.0;
                                let profit = sell_result_weth * weth_usdc - amount;
                                log_fee_data("Camelot", &pair_name, &camelot_addr, sell_result.1)
                                    .map_err(|e| {
                                    Box::new(std::io::Error::new(
                                        std::io::ErrorKind::Other,
                                        e.to_string(),
                                    ))
                                        as Box<dyn std::error::Error + Send + Sync>
                                })?;
                                // profit >= 0.0 && buy_result.1 < 1.0
                                // if profit > current_best_profit {
                                if true {
                                    if is_logging {
                                        if let Err(e) = log_simulation(
                                            "simulation.log",
                                            "Uniswap",
                                            &uni_addr,
                                            uni_data.token0_amount_adjusted,
                                            uni_data.token1_amount_adjusted,
                                            uni_data.price,
                                            uniswap_usdc_price,
                                            "Camelot",
                                            &camelot_addr,
                                            camelot_data.token0_amount_adjusted,
                                            camelot_data.token1_amount_adjusted,
                                            camelot_data.price,
                                            camelot_usdc_price,
                                            amount_weth,
                                            amount,
                                            buy_result_weth,
                                            sell_result_weth,
                                            sell_result_weth * weth_usdc,
                                            sell_result_weth - amount_weth,
                                            profit,
                                            tier,
                                        ) {
                                            eprintln!("Failed to log simulation: {}", e);
                                        }
                                    }
                                    current_best_profit = profit;
                                    best_trade_data = Some((
                                        profit,
                                        amount_weth,
                                        buy_result_weth,
                                        uni_data.fee,
                                        sell_result.1,
                                        UNISWAP_V3_SWAP_ROUTER_ADDRESS.to_string(),
                                        CAMELOT_V3_SWAP_ROUTER_ADDRESS.to_string(),
                                        uni_data.token0.clone(),
                                        uni_data.token1.clone(),
                                        camelot_data.token0.clone(),
                                        camelot_data.token1.clone(),
                                        uni_data.token0_decimals,
                                        uni_data.token1_decimals,
                                        camelot_data.token0_decimals,
                                        camelot_data.token1_decimals,
                                    ));
                                }
                            } else {
                                let buy_result = simulate_camelot_trade_with_slippage(
                                    camelot_data.token0,
                                    camelot_data.token0_decimals,
                                    camelot_data.token1,
                                    camelot_data.token1_decimals,
                                    amount_weth,
                                    "BUY",
                                    provider.clone(),
                                )
                                .await?;
                                let buy_result_weth = buy_result.0;
                                let sell_result = simulate_uniswap_trade_with_slippage(
                                    uni_data.token0,
                                    uni_data.token0_decimals,
                                    uni_data.token1,
                                    uni_data.token1_decimals,
                                    buy_result_weth,
                                    uni_data.fee,
                                    "SELL",
                                    provider.clone(),
                                )
                                .await?;
                                let sell_result_weth = sell_result;
                                let profit = sell_result_weth * weth_usdc - amount;
                                log_fee_data("Camelot", &pair_name, &camelot_addr, buy_result.1)
                                    .map_err(|e| {
                                        Box::new(std::io::Error::new(
                                            std::io::ErrorKind::Other,
                                            e.to_string(),
                                        ))
                                            as Box<dyn std::error::Error + Send + Sync>
                                    })?;
                                // profit >= 0.0 && buy_result.1 < 1.0
                                // if profit > current_best_profit {
                                if true {
                                    if is_logging {
                                        if let Err(e) = log_simulation(
                                            "simulation.log",
                                            "Camelot",
                                            &camelot_addr,
                                            camelot_data.token0_amount_adjusted,
                                            camelot_data.token1_amount_adjusted,
                                            camelot_data.price,
                                            camelot_usdc_price,
                                            "Uniswap",
                                            &uni_addr,
                                            uni_data.token0_amount_adjusted,
                                            uni_data.token1_amount_adjusted,
                                            uni_data.price,
                                            uniswap_usdc_price,
                                            amount_weth,
                                            amount,
                                            buy_result_weth,
                                            sell_result_weth,
                                            sell_result_weth * weth_usdc,
                                            sell_result_weth - amount_weth,
                                            profit,
                                            tier,
                                        ) {
                                            eprintln!("Failed to log simulation: {}", e);
                                        }
                                    }
                                    current_best_profit = profit;
                                    best_trade_data = Some((
                                        profit,
                                        amount_weth,
                                        buy_result_weth,
                                        buy_result.1,
                                        uni_data.fee,
                                        CAMELOT_V3_SWAP_ROUTER_ADDRESS.to_string(),
                                        UNISWAP_V3_SWAP_ROUTER_ADDRESS.to_string(),
                                        camelot_data.token0.clone(),
                                        camelot_data.token1.clone(),
                                        uni_data.token0.clone(),
                                        uni_data.token1.clone(),
                                        camelot_data.token0_decimals,
                                        camelot_data.token1_decimals,
                                        uni_data.token0_decimals,
                                        uni_data.token1_decimals,
                                    ));
                                }
                            }
                        }

                        if let Some((
                            profit,
                            amount_weth,
                            buy_amount,
                            buy_fee,
                            sell_fee,
                            buy_dex,
                            sell_dex,
                            buy_token0,
                            buy_token1,
                            sell_token0,
                            sell_token1,
                            buy_token0_decimals,
                            buy_token1_decimals,
                            sell_token0_decimals,
                            sell_token1_decimals,
                        )) = best_trade_data
                        {
                            if profit > stream_results.best_trade.read().unwrap().profit_usdc {
                                let mut best_trade = stream_results.best_trade.write().unwrap();
                                best_trade.profit_usdc = profit;

                                let (buy_token, buy_decimals) =
                                    if hex::encode(buy_token0.as_bytes())
                                        == WETH_ADDRESS.to_lowercase()
                                    {
                                        (buy_token1, buy_token0_decimals)
                                    } else {
                                        (buy_token0, buy_token1_decimals)
                                    };

                                let (_, sell_decimals) = if hex::encode(sell_token0.as_bytes())
                                    == WETH_ADDRESS.to_lowercase()
                                {
                                    (sell_token1, sell_token1_decimals)
                                } else {
                                    (sell_token0, sell_token0_decimals)
                                };

                                let mut bytes = [0u8; 32];

                                let buy_amount_wei = amount_weth * 10f64.powi(buy_decimals as i32);
                                let buy_amount_u256 = U256::from(buy_amount_wei as u128);
                                buy_amount_u256.to_big_endian(&mut bytes);
                                println!(
                                    "C-U Buy amount: {} {} {} {:?}",
                                    amount_weth, buy_amount_wei, buy_amount_u256, bytes
                                );
                                best_trade
                                    .buy_dex
                                    .copy_from_slice(&hex::decode(&buy_dex[2..]).unwrap());
                                best_trade
                                    .buy_token_in
                                    .copy_from_slice(&hex::decode(WETH_ADDRESS).unwrap());
                                best_trade.buy_token_out.copy_from_slice(&buy_token.0);
                                best_trade.buy_fee = buy_fee;
                                best_trade.buy_amount.copy_from_slice(&bytes);
                                best_trade
                                    .sell_dex
                                    .copy_from_slice(&hex::decode(&sell_dex[2..]).unwrap());
                                best_trade.sell_token_in.copy_from_slice(&buy_token.0);
                                best_trade
                                    .sell_token_out
                                    .copy_from_slice(&hex::decode(WETH_ADDRESS).unwrap());
                                best_trade.sell_fee = sell_fee;

                                let sell_amount_wei = buy_amount * 10f64.powi(sell_decimals as i32);
                                let sell_amount_u256 = U256::from(sell_amount_wei as u128);
                                sell_amount_u256.to_big_endian(&mut bytes);
                                best_trade.sell_amount.copy_from_slice(&bytes);
                            }
                        }
                    }
                    // Update shared memory
                    // let mut shm = shm_manager.write().unwrap();
                    // let camelot_feed = PoolFeed {
                    //     block_number,
                    //     price: camelot_usdc_price,
                    //     liquidity: camelot_usdc_liquidity as u128,
                    //     tick: camelot_data.tick,
                    //     timestamp,
                    // };
                    // let uni_feed = PoolFeed {
                    //     block_number,
                    //     price: uniswap_usdc_price,
                    //     liquidity: uniswap_usdc_liquidity as u128,
                    //     tick: uni_data.tick,
                    //     timestamp,
                    // };

                    // shm.update_pool(*pool_index, camelot_feed)?;
                    // shm.update_pool(*pool_index + 1, uni_feed)?;

                    Ok(())
                }
            });

        // Execute all futures in parallel
        let results = futures::future::join_all(tasks).await;
        for result in results {
            if let Err(e) = result {
                eprintln!("Error processing pair: {}", e);
            }
        }
    }
    Ok(())
}
