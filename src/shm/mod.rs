use std::fs::OpenOptions;
use std::io::{self};
use memmap2::{MmapMut, MmapOptions};
use serde::{Serialize, Deserialize};

// Define the pool data structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoolFeed {
    pub block_number: u64,
    pub price: f64,
    pub liquidity: u128,
    pub tick: i32,
    pub timestamp: u64,
}

// Shared memory manager
#[allow(dead_code)]
pub struct SharedMemoryManager {
    mmap: MmapMut,
    pool_count: usize,
}

impl SharedMemoryManager {
    pub fn new(path: &str, pool_count: usize) -> io::Result<Self> {
        let size = pool_count * std::mem::size_of::<PoolFeed>();
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
            
        file.set_len(size as u64)?;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        
        Ok(Self { mmap, pool_count })
    }
    
    pub fn _update_pool(&mut self, index: usize, data: PoolFeed) -> io::Result<()> {
        if index >= self.pool_count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Pool index {} out of bounds (max {})", index, self.pool_count)
            ));
        }
    
        let serialized = serde_json::to_vec(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
        // Calculate exact size needed
        let struct_size = std::mem::size_of::<PoolFeed>();

        if serialized.len() > struct_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Serialized data size ({} bytes) exceeds allocated space ({} bytes)", 
                        serialized.len(), struct_size)
            ));
        }
        let mut padded_data = vec![0u8; struct_size];
        padded_data[..serialized.len()].copy_from_slice(&serialized);
    
        let offset = index * struct_size;
        let end_offset = offset + struct_size;
        
        self.mmap[offset..end_offset].copy_from_slice(&padded_data);
        self.mmap.flush_async()?;
        
        Ok(())
    }
}