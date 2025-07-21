# Scanner-Bot

A DEX arbitrage scanner bot that monitors price differences between Uniswap, PancakeSwap, and Camelot exchanges.

## Prerequisites

- Rust toolchain installed
- Cargo package manager

## Quick Start

1. Clone the repository:
```bash
git clone https://github.com/Tobi8068/Arbitrum-Arbitrage-Monitoring-Bot.git
```

```bash
cd Scanner-Bot
```

2. Create a `.env` file in the root directory with the following variables:
```
# RPC Provider URL
RPC_URL=your_rpc_url_here

# Flag to enable simulation logging

IS_SIMULATION_LOGGING_ENABLED = false

# Trading Tiers Configuration
START_AMOUNT_TIER1=100  # USDC
START_AMOUNT_TIER2=200  # USDC
START_AMOUNT_TIER3=300  # USDC

STEP_TIER1=10  # 10 USDC increment
STEP_TIER2=20  # 20 USDC increment
STEP_TIER3=30  # 30 USDC increment

STEP_NUMBER_TIER1=10  # Number of steps for tier 1
STEP_NUMBER_TIER2=10  # Number of steps for tier 2
STEP_NUMBER_TIER3=10  # Number of steps for tier 3
```

3. Build and run the project:
```bash
cargo run --bin Scanner_Bot
```

## Features

### Real-time Price Monitoring
- Simultaneous monitoring of token pairs across three major DEXes:
  - Uniswap V3
  - PancakeSwap V3
  - Camelot V3
- Websocket connections for instant price updates
- Timestamp precision tracking to millisecond level

### Comprehensive Arbitrage Detection
- Cross-exchange price difference calculation
- Percentage-based price differential analysis
- Three distinct arbitrage routes monitored:
  - Uniswap ↔ PancakeSwap
  - Camelot ↔ PancakeSwap
  - Camelot ↔ Uniswap

### Liquidity Analysis
- Minimum liquidity determination between exchanges
- Liquidity-aware trading strategy
- Protection against low-liquidity situations

### Multi-tier Trading Strategy
- Four configurable trading tiers based on:
  - Available liquidity
  - Price difference percentage
- Dynamic trade size adjustment
- Incremental trade simulation within each tier

### Advanced Simulation
- Simulated trade execution without capital commitment
- Step-by-step trade size increments
- Profit calculation accounting for:
  - Gas costs
  - Slippage
  - Exchange fees

### Detailed Logging
- Configurable logging system
- Price and liquidity information recording
- Timestamp-based event tracking
- Performance metrics collection
- Optional simulation logging for detailed analysis

## Configuration

The bot monitors:
- Uniswap V3
- PancakeSwap V3
- Camelot V3

## Environment Variables Explained

- `RPC_URL`: Your Ethereum node RPC endpoint
- `IS_SIMULATION_LOGGING_ENABLED`: Flag to enable simulation logging

### Trading Tiers Configuration
- `START_AMOUNT_TIER*`: Initial amount for each trading tier in Wei
- `STEP_TIER*`: Amount to increment in each step for the respective tier
- `STEP_NUMBER_TIER*`: Number of incremental steps to try in each tier

## Logging

Simulation results are logged to `simulation.log` when enabled.
