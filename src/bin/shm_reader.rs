use std::fs::File;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use memmap2::MmapOptions;
use serde_json::from_slice;
use serde::{Serialize, Deserialize};
use std::thread::sleep;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoolFeed {
    pub block_number: u64,
    pub price: f64,
    pub liquidity: u128,
    pub tick: i32,
    pub timestamp: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting shared memory reader...");
    
    // Open the shared memory file
    let file = File::open("/tmp/pool_data.shm")?;
    
    // Create a read-only memory map
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    
    println!("Connected to shared memory. Size: {} bytes", mmap.len());
    println!("Waiting for data...");
    
    // Continuously read from shared memory
    loop {
        // Try to deserialize the data
        match from_slice::<HashMap<[u8; 20], PoolFeed>>(&mmap) {
            Ok(pool_data) => {
                println!("\n--- Current Pool Data ({} pools) ---", pool_data.len());
                
                for (addr, feed) in &pool_data {
                    // Convert address bytes to hex string
                    let addr_hex = format!("0x{}", hex::encode(addr));
                    
                    // Calculate how many seconds ago this data was updated
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let age = now.saturating_sub(feed.timestamp);
                    
                    println!(
                        "Pool: {} | Block: {} | Price: ${:.6} | Liquidity: {} | Tick: {} | Age: {}s",
                        addr_hex, feed.block_number, feed.price, feed.liquidity, feed.tick, age
                    );
                }
            },
            Err(e) => {
                println!("Error reading shared memory: {}", e);
            }
        }
        
        // Wait before reading again
        sleep(Duration::from_secs(2));
    }
}
