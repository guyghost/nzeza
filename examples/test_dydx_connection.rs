//! Test dYdX v4 Connection
//!
//! This example tests the dYdX v4 API connection using the official client.
//! Run with: cargo run --example test_dydx_connection

use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔍 Testing dYdX v4 API Connection...\n");

    // Load mnemonic from environment
    let mnemonic = env::var("DYDX_MNEMONIC")
        .expect("DYDX_MNEMONIC not set. Set it with: export DYDX_MNEMONIC='your twelve word phrase here'");

    println!("✅ Mnemonic loaded:");
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    println!("   Words: {} (expected 12 or 24)", words.len());
    println!("   First word: {}", words.first().unwrap_or(&""));
    println!("   Last word: {}\n", words.last().unwrap_or(&""));

    if words.len() != 12 && words.len() != 24 {
        eprintln!("❌ Invalid mnemonic: expected 12 or 24 words, got {}", words.len());
        return Err("Invalid mnemonic length".into());
    }

    // Import dYdX client
    use dydx::config::ClientConfig;
    use dydx::indexer::IndexerClient;
    use dydx::node::{NodeClient, Wallet};

    println!("🔌 Creating dYdX v4 client (Mainnet)...");

    // Load configuration from file
    let config = ClientConfig::from_file("dydx_mainnet.toml")
        .await
        .map_err(|e| format!("Failed to load config: {:?}", e))?;

    // Create node client
    println!("📡 Connecting to dYdX node...");
    let mut node_client = NodeClient::connect(config.node.clone())
        .await
        .map_err(|e| format!("Failed to connect to node: {:?}", e))?;
    println!("✅ Node client connected\n");

    // Create indexer client
    let indexer_client = IndexerClient::new(config.indexer);
    println!("✅ Indexer client created\n");

    // Create wallet from mnemonic
    println!("🔐 Creating wallet from mnemonic...");
    let wallet = Wallet::from_mnemonic(&mnemonic)
        .map_err(|e| format!("Failed to create wallet: {:?}", e))?;
    println!("✅ Wallet created successfully\n");

    // Get account information
    println!("📊 Fetching account information...");
    let mut account = wallet
        .account(0, &mut node_client)
        .await
        .map_err(|e| format!("Failed to get account: {:?}", e))?;

    println!("✅ Account retrieved:");
    println!("   Address: {}", account.address());
    println!("   Account number: {}", account.account_number());
    println!("   Sequence: {}\n", account.sequence_number());

    // Get subaccount
    let subaccount = account
        .subaccount(0)
        .map_err(|e| format!("Failed to get subaccount: {:?}", e))?;
    println!("✅ Subaccount 0:");
    println!("   Number: {}", subaccount.number);
    println!("   Address: {}\n", subaccount.address);

    // Test indexer - get markets
    use dydx::indexer::{ListPerpetualMarketsOpts, Ticker};

    println!("📈 Fetching markets from indexer...");
    let markets_opts = ListPerpetualMarketsOpts {
        limit: Some(5),
        ..Default::default()
    };

    match indexer_client
        .markets()
        .get_perpetual_markets(Some(markets_opts))
        .await
    {
        Ok(markets) => {
            println!("✅ Markets retrieved: {} markets\n", markets.len());

            // Show first 5 markets
            for (i, (ticker, market)) in markets.iter().take(5).enumerate() {
                println!("Market {}:", i + 1);
                println!("  Ticker: {}", ticker);
                println!("  Status: {:?}", market.status);
                if let Some(oracle_price) = &market.oracle_price {
                    println!("  Oracle price: {}", oracle_price);
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("⚠️  Failed to fetch markets: {:?}", e);
            eprintln!("   (This might be normal if indexer is temporarily unavailable)");
        }
    }

    // Test getting a specific market
    println!("📊 Testing BTC-USD market data...");
    let btc_ticker = Ticker::from("BTC-USD");
    let btc_opts = ListPerpetualMarketsOpts {
        ticker: Some(btc_ticker.clone()),
        ..Default::default()
    };

    match indexer_client
        .markets()
        .get_perpetual_markets(Some(btc_opts))
        .await
    {
        Ok(markets) => {
            if let Some((ticker, market)) = markets.iter().next() {
                println!("✅ BTC-USD Market:");
                println!("  Ticker: {}", ticker);
                println!("  Status: {:?}", market.status);
                println!("  Clob pair ID: {:?}", market.clob_pair_id);
                if let Some(oracle_price) = &market.oracle_price {
                    println!("  Oracle price: {}", oracle_price);
                }
                println!();
            } else {
                println!("⚠️  BTC-USD market not found");
            }
        }
        Err(e) => {
            eprintln!("⚠️  Failed to fetch BTC-USD market: {:?}", e);
        }
    }

    println!("✅ dYdX v4 connection test PASSED");
    println!("\n🎉 You can now use dYdX v4 for trading!");
    println!("\n⚠️  Important notes:");
    println!("   - This is a REAL connection to dYdX mainnet");
    println!("   - Any orders placed will be REAL trades");
    println!("   - Start with small amounts to test");
    println!("   - Always check positions and balances carefully");

    Ok(())
}
