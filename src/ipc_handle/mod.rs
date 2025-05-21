use iceoryx2::prelude::*;
use iceoryx2_bb_container::vec::FixedSizeVec;
use std::sync::{Arc, RwLock, mpsc};
use bincode::{Encode, Decode};
use crate::config::IPC_CYCLE_TIME;
use std::io;
use std::thread;

#[derive(Clone)]
pub struct BestTrade {
    pub profit_usdc: f64,
    pub buy_dex: [u8; 20],
    pub buy_token_in: [u8; 20],
    pub buy_token_out: [u8; 20],
    pub buy_fee: u32,
    pub buy_amount: [u8; 32],
    pub sell_dex: [u8; 20],
    pub sell_token_in: [u8; 20],
    pub sell_token_out: [u8; 20],
    pub sell_fee: u32,
    pub sell_amount: [u8; 32],
}

pub struct StreamResults {
    pub best_trade: Arc<RwLock<BestTrade>>,
}

#[derive(Encode, Decode, Debug, Clone, Default)]
#[repr(C)]

struct ArbTran {
    dex: [u8; 20],
    token_from: [u8; 20],
    token_to: [u8; 20],
    fee: u32,
    amount: [u8; 32],
}

#[derive(Encode, Decode, Debug, Clone, Default)]
#[repr(C)]
struct Opportunity {
    first_transaction: ArbTran,
    second_transaction: ArbTran,
}

// Message type for the channel
const DATA_SIZE: usize = std::mem::size_of::<Opportunity>();

type IpcMessage = FixedSizeVec<u8, DATA_SIZE>;

// Function to run the publisher in a dedicated thread
fn run_publisher_thread(rx: mpsc::Receiver<IpcMessage>) -> Result<(), Box<dyn std::error::Error>> {
    let node = NodeBuilder::new().create::<ipc::Service>()?;


    let service = node
        .service_builder(&"arbiscan_bot".try_into()?)
        .publish_subscribe::<FixedSizeVec<u8, DATA_SIZE>>()
        .open_or_create()?;

    let publisher = service.publisher_builder().create()?;

    while let Ok(msg) = rx.recv() {
        let sample = publisher.loan_uninit()?;
        let sample = sample.write_payload(msg);
        sample.send()?;
    }

    Ok(())
}

// Modified handle_ipc_stream to use channel
pub async fn handle_ipc_stream(stream_results: Arc<StreamResults>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, rx) = mpsc::channel();

    println!("🚀 Starting IPC publisher thread");
    
    // Spawn publisher thread
    let _publisher_thread = thread::spawn(move || {
        if let Err(e) = run_publisher_thread(rx) {
            eprintln!("❌ IPC publisher thread error: {}", e);
        }
    });

    println!("📡 IPC Stream initialized and ready to transmit");

    loop {
        let should_send = {
            let best_trade = stream_results.best_trade.read()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            
            // Check if trade is non-initial state
            best_trade.profit_usdc > 0.0 || 
            best_trade.buy_fee > 0 ||
            best_trade.sell_fee > 0 ||
            best_trade.buy_dex.iter().any(|&x| x != 0) ||
            best_trade.sell_dex.iter().any(|&x| x != 0)
        };
        // Create opportunity message inside a block to drop the read guard before await
        if should_send {
            let msg = {
                let best_trade = stream_results.best_trade.read()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
                
                let mut msg = FixedSizeVec::<u8, DATA_SIZE>::new();
                let mut slice = [0u8; DATA_SIZE];
                
                let opportunity = pack_trade_data(&best_trade);
                bincode::encode_into_slice(&opportunity, &mut slice, bincode::config::standard())?;

                println!("💹 Trade Opportunity Detected:");
                println!("🔄 First Transaction:");
                println!("   📍 DEX: 0x{}", hex::encode(&opportunity.first_transaction.dex));
                println!("   💱 From: 0x{} -> To: 0x{}", 
                    hex::encode(&opportunity.first_transaction.token_from),
                    hex::encode(&opportunity.first_transaction.token_to));
                println!("   💰 Amount: 0x{}", hex::encode(&opportunity.first_transaction.amount));
                println!("   🏷️ Fee: {}", opportunity.first_transaction.fee);
                
                println!("🔄 Second Transaction:");
                println!("   📍 DEX: 0x{}", hex::encode(&opportunity.second_transaction.dex));
                println!("   💱 From: 0x{} -> To: 0x{}", 
                    hex::encode(&opportunity.second_transaction.token_from),
                    hex::encode(&opportunity.second_transaction.token_to));
                println!("   💰 Amount: 0x{}", hex::encode(&opportunity.second_transaction.amount));
                println!("   🏷️ Fee: {}", opportunity.second_transaction.fee);
                println!("📊 Profit (USDC): {}", best_trade.profit_usdc);
                
                msg.extend_from_slice(&slice);
                msg
            };

            // Send message through channel
            if tx.send(msg).is_err() {
                return Err("IPC publisher thread terminated".into());
            }
        }
        tokio::time::sleep(IPC_CYCLE_TIME).await;
    }
}

// Helper function to pack trade data
fn pack_trade_data(trade: &BestTrade) -> Opportunity {
    Opportunity {
        first_transaction: ArbTran {
            dex: trade.buy_dex,
            token_from: trade.buy_token_in,
            token_to: trade.buy_token_out,
            fee: trade.buy_fee,
            amount: trade.buy_amount,
        },
        second_transaction: ArbTran {
            dex: trade.sell_dex,
            token_from: trade.sell_token_in,
            token_to: trade.sell_token_out,
            fee: trade.sell_fee,
            amount: trade.sell_amount,
        }
    }
}