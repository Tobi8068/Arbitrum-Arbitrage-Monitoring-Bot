use iceoryx2::prelude::*;
use iceoryx2_bb_container::vec::FixedSizeVec;
use bincode::{Encode, Decode};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let node = NodeBuilder::new().create::<ipc::Service>()?;

    let service = node
        .service_builder(&"ScanOpportunities".try_into()?)
        .publish_subscribe::<FixedSizeVec<u8, 262>>()
        .open()?;

    let subscriber = service.subscriber_builder().create()?;

    println!("Subscriber started. Waiting for messages...");

    loop {
        if let Ok(Some(sample)) = subscriber.receive() {
            let data = sample.payload();
            
            match bincode::decode_from_slice::<Opportunity, _>(data, bincode::config::standard()) {
                Ok((opportunity, _)) => {
                    println!("Received opportunity:");
                    println!("First Transaction:");
                    println!("  DEX: {:?}", opportunity.first_transaction.dex);
                    println!("  Token From: {:?}", opportunity.first_transaction.token_from);
                    println!("  Token To: {:?}", opportunity.first_transaction.token_to);
                    println!("  Fee: {}", opportunity.first_transaction.fee);
                    println!("  Amount: {:?}", opportunity.first_transaction.amount);
                    
                    println!("\nSecond Transaction:");
                    println!("  DEX: {:?}", opportunity.second_transaction.dex);
                    println!("  Token From: {:?}", opportunity.second_transaction.token_from);
                    println!("  Token To: {:?}", opportunity.second_transaction.token_to);
                    println!("  Fee: {}", opportunity.second_transaction.fee);
                    println!("  Amount: {:?}", opportunity.second_transaction.amount);
                }
                Err(e) => println!("Failed to decode message: {}", e),
            }
        }
    }
}
