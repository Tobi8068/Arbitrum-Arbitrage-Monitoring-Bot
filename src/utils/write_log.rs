use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::sync::Mutex;
use lazy_static::lazy_static;
use chrono::{Local, Utc};

lazy_static! {
    static ref LOG_MUTEX: Mutex<()> = Mutex::new(());
}

// Function to log price and liquidity information
pub fn log_price_liquidity(
    file: &mut File,
    timestamp: u64,
    pair_name: &str,
    pool_a_addr: &str,
    pool_b_addr: &str,
    min_liquidity: f64,
    price_a: f64,
    price_b: f64,
    price_diff_pct: f64,
    exchange_a: &str,
    exchange_b: &str,
) -> std::io::Result<()> {
    let datetime = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    
    // Choose emoji based on price difference
    let emoji = if price_diff_pct > 0.01 {
        "🔥" // Hot opportunity
    } else if price_diff_pct > 0.005 {
        "⚡" // Potential opportunity
    } else if price_diff_pct > 0.002 {
        "👀" // Worth watching
    } else {
        "🧊" // Cold, minimal difference
    };
    
    // Choose liquidity emoji
    let liq_emoji = if min_liquidity > 1_000_000.0 {
        "💰" // High liquidity
    } else if min_liquidity > 100_000.0 {
        "💵" // Medium liquidity
    } else {
        "💸" // Low liquidity
    };
    
    // Format the log entry
    let log_entry = format!(
        "{} | {} {} | Pair: {} | {} Pool: {} | {} Pool: {} | Min Liquidity: ${:.2} {} | {} Price: ${:.6} | {} Price: ${:.6} | Diff: {:.4}% {}\n",
        datetime,
        timestamp,
        emoji,
        pair_name,
        exchange_a,
        pool_a_addr,
        exchange_b,
        pool_b_addr,
        min_liquidity,
        liq_emoji,
        exchange_a,
        price_a,
        exchange_b,
        price_b,
        price_diff_pct * 100.0,
        emoji
    );
    
    file.write_all(log_entry.as_bytes())?;
    file.flush()?;
    
    Ok(())
}

pub fn log_simulation(
    file_path: &str, 
    buy_dex: &str,
    buy_addr: &str,
    buy_dex_token0_amount: f64,
    buy_dex_token1_amount: f64,
    buy_weth_price: f64,
    buy_usdc_price: f64,
    sell_dex: &str,
    sell_addr: &str,
    sell_dex_token0_amount: f64,
    sell_dex_token1_amount: f64,
    sell_weth_price: f64,
    sell_usdc_price: f64,
    amount_in_weth: f64,
    amount_in_usdc: f64,
    buy_amount_out: f64,
    sell_amount_out: f64,
    sell_amount_out_usdc: f64,
    profit_weth: f64,
    profit_usdc: f64,
    tier: u8,
) -> std::io::Result<()> {
    let _lock = LOG_MUTEX.lock().unwrap(); // Lock the mutex to ensure thread safety
    let now = Utc::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let tier_indicator = match tier {
        4 => "🔴 TIER 4",
        3 => "🟣 TIER 3",
        2 => "🟡 TIER 2",
        1 => "🟢 TIER 1",
        0 => "⚪ TIER 0",
        _ => "⚫ NO TIER",
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path)?;

    writeln!(file, "🕒 [{timestamp}] Trade Simulation Started")?;
    writeln!(file, "📊 {tier_indicator}")?;
    writeln!(file, "💰 Buy on {buy_dex}")?;
    writeln!(file, "   ├─ Pool Address: {buy_addr} 📍")?;
    writeln!(file, "   ├─ Token0 Amount: {:.6} 💎 Token1 Amount: {:.6} 💎", buy_dex_token0_amount, buy_dex_token1_amount)?;
    writeln!(file, "   ├─ Price WETH: {:.6} 🔷 USDC: {:.6} 💵", buy_weth_price, buy_usdc_price)?;
    writeln!(file, "   ├─ Amount In: {:.6} WETH {:.6} USDC 📥", amount_in_weth, amount_in_usdc)?;
    writeln!(file, "   └─ Amount Out: {:.6} TOKEN 📤", buy_amount_out)?;
    writeln!(file, "💱 Sell on {sell_dex}")?;
    writeln!(file, "   ├─ Pool Address: {sell_addr} 📍")?;
    writeln!(file, "   ├─ Token0 Amount: {:.6} 💎 Token1 Amount: {:.6} 💎", sell_dex_token0_amount, sell_dex_token1_amount)?;
    writeln!(file, "   ├─ Price WETH: {:.6} 🔷 USDC: {:.6} 💵", sell_weth_price, sell_usdc_price)?;
    writeln!(file, "   ├─ Amount In: {:.6} TOKEN 📥", buy_amount_out)?;
    writeln!(file, "   └─ Amount Out: {:.6} WETH {:.6} USDC 📤", sell_amount_out, sell_amount_out_usdc)?;
    writeln!(file, "💫 Profit: {:.6} WETH {:.6} USDC {}", profit_weth, profit_usdc, if profit_weth > 0.0 { "📈" } else { "📉" })?;
    writeln!(file, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")?;

    file.flush()?;

    Ok(())
}

pub fn log_fee_data(
    dex: &str,
    pair_name: &str, 
    pair_address: &str,
    fee: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = "fee.log";
    let _lock = LOG_MUTEX.lock().unwrap(); // Add thread safety
    
    // Validate inputs
    if dex.is_empty() || pair_name.is_empty() || pair_address.len() != 42 {
        return Err("Invalid input parameters".into());
    }

    // Read existing entries with proper error handling
    let mut entries = Vec::new();
    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                // Skip malformed lines
                if line.split(',').count() != 4 {
                    continue;
                }
                entries.push(line);
            }
        }
    }

    // Remove existing entry for this pair if exists
    entries.retain(|line| !line.contains(pair_address));
    
    // Add new entry with proper formatting
    let new_entry = format!("{},{},{},{}%", 
        dex.trim(),
        pair_name.trim(),
        pair_address.trim(),
        fee as f64 / 10000.0
    );
    entries.push(new_entry);

    // Write entries with proper error handling
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;

    for entry in entries {
        writeln!(file, "{}", entry)?;
    }
    file.flush()?;

    Ok(())
}
