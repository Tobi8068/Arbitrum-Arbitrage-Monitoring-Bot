pub fn determine_tier(liquidity: f64) -> u8 {
    if liquidity >= 1000000.0 {
        4 // Tier 4: $1,000,000+ (min. price difference 0.9%)
    } else if liquidity <= 1000000.0 && liquidity >= 200000.0 {
        3 // Tier 3: $100,000 - $1,000,000 (min. price difference 1.0%)
    } else if liquidity <= 200000.0 && liquidity >= 50000.0 {
        2 // Tier 2: $50,000 - $100,000 (min. price difference 1.6%)
    } else if liquidity <= 50000.0 && liquidity >= 20000.0 {
        1 // Tier 1: $20,000 - $50,000 (min. price difference 1.8%)
    } else if liquidity <= 20000.0 && liquidity >= 0.0 {
        0 // Below minimum tier
    } 
    else {
        5
    }
}