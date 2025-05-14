# Solana Meteora Sniper Bot

A high-performance Solana bot designed to detect new liquidity pool creations on the Meteora AMM and execute swift token purchases as soon as new pools are detected.

## Features

- **Real-time Pool Detection**: Utilizes Yellowstone gRPC to efficiently monitor the blockchain for new Meteora pool creation in real-time
- **WSOL Pair Focus**: Specifically targets pools that include WSOL (Wrapped SOL) in trading pairs
- **Multi-RPC Broadcasting**: Supports sending transactions through multiple RPCs in parallel, including:
  - Standard Solana RPC
  - Jito MEV
  - bloXroute Trader API
  - NextBlock transaction API
- **Transaction Optimization**: Configurable compute unit price and limit for optimal transaction execution
- **Simulation Mode**: Test transaction execution without actually submitting transactions to the blockchain
- **Flexible Configuration**: Easily configure bot parameters through a YAML file

## How It Works

1. The bot connects to a Yellowstone gRPC endpoint to receive real-time transaction data from the Solana blockchain
2. When a new Meteora pool initialization transaction is detected, the bot analyzes it to determine:
   - If one of the tokens in the pair is WSOL
   - The token mint address of the other token in the pair
3. For each new WSOL pair detected, the bot constructs a swap transaction to buy the newly listed token
4. The transaction is then either:
   - Simulated (if `simulate` is set to `true`)
   - Broadcast to all configured RPC endpoints in parallel

## Setup

1. Clone the repository:

   ```
   git clone https://github.com/deadraid/solana-test-bot.git
   cd solana-test-bot
   ```

2. Copy the example config file and edit it:

   ```
   cp config.yaml.example config.yaml
   ```

3. Edit `config.yaml` with your settings:

   - Set your Solana private key (in base58 format)
   - Configure your RPC endpoints
   - Set transaction parameters
   - Optionally, enable simulation mode

4. Build the project:
   ```
   cargo build --release
   ```

## Configuration

The `config.yaml` file has the following structure:

```yaml
rpc:
  "rpc1":
    url: "https://api.mainnet-beta.solana.com"
    rpc_type: "solanarpc"
  "jito-rpc1":
    url: "https://ny.mainnet.block-engine.jito.wtf/api/v1/transactions"
    rpc_type: "jito"
  "bloxroute1":
    url: "https://ny.solana.dex.blxrbdn.com/api/v2/submit"
    rpc_type: "bloxroute"
    auth: "YOUR_BLOXROUTE_API_KEY"
  "nextblock1":
    url: "https://beta.nextblock.xyz/api/v1/submit"
    rpc_type: "nextblock"
    auth: "YOUR_NEXTBLOCK_API_KEY"

geyser_url: ""
geyser_x_token: ""

http_rpc: "https://api.mainnet-beta.solana.com"
ws_rpc: "wss://api.mainnet-beta.solana.com"

private_key: "" # Your base58 encoded private key

compute_unit_price: 10000000
compute_unit_limit: 100000

tip: 0.001 # Optional SOL tip for MEV inclusion
buy_amount: 0.0001 # Amount of SOL to swap
min_amount_out: 100 # Minimum tokens to receive

simulate: true # Set to false for actual transactions
```

### Configuration Parameters

| Parameter            | Description                                              |
| -------------------- | -------------------------------------------------------- |
| `rpc`                | Map of RPC endpoints to broadcast transactions to        |
| `geyser_url`         | Yellowstone gRPC endpoint for receiving transaction data |
| `geyser_x_token`     | Authentication token for Yellowstone (if required)       |
| `http_rpc`           | Standard HTTP RPC endpoint for general operations        |
| `ws_rpc`             | WebSocket RPC endpoint for subscriptions                 |
| `private_key`        | Base58 encoded private key for transaction signing       |
| `compute_unit_price` | Price per compute unit in lamports                       |
| `compute_unit_limit` | Maximum compute units for transactions                   |
| `tip`                | Optional SOL tip for MEV services like Jito              |
| `buy_amount`         | Amount of SOL to swap for new tokens                     |
| `min_amount_out`     | Minimum number of tokens to receive                      |
| `simulate`           | If true, transactions are simulated but not sent         |

## Running the Bot

The project supports two launch modes:

### 1. Live Trading Bot Mode

Run the bot to listen for real-time transactions and execute swaps:

```
RUST_LOG=info ./target/release/meteora-sniper-bot
```

### 2. Historical Simulation Mode

Test the bot against historical transaction signatures:

#### B. Simulate Bot Response to Historical Transactions

Then, simulate how the bot would respond to one of those transactions:

```
RUST_LOG=debug ./target/release/meteora-sniper-bot --bin inject_sim [TRANSACTION_SIGNATURE]
```

or

```
cargo run --release --bin inject_sim -- [TRANSACTION_SIGNATURE]
```

If you don't provide a transaction signature, it will use a default example signature.

### Logging Levels

The bot uses the standard Rust logging framework and supports different logging levels via the `RUST_LOG` environment variable:

```
# Debug level logging (verbose)
RUST_LOG=debug ./target/release/meteora-sniper-bot

# Info level logging (recommended for production)
RUST_LOG=info ./target/release/meteora-sniper-bot

# Only show warnings and errors
RUST_LOG=warn ./target/release/meteora-sniper-bot
```

You can also set more specific logging filters:

```
# Debug level for meteora modules only, info for everything else
RUST_LOG=info,meteora_sniper_bot::meteora=debug ./target/release/meteora-sniper-bot
```

## Security Considerations

- **Private Key**: Your private key is used to sign transactions. Never share your config file.
- **Simulation Mode**: Start with `simulate: true` to test your setup without risking funds.
- **Risk Management**: Sniping new pools carries significant risk - only use funds you can afford to lose.

## Dependencies

- Solana SDK and related crates (v2.2.2+)
- Yellowstone gRPC for transaction monitoring
- Tokio for async runtime
- Additional utility crates (see Cargo.toml for full list)

## License

This project is available as open source under the terms of the MIT License.
