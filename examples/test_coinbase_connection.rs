//! Test Coinbase Connection
//!
//! This example tests the Coinbase API connection and authentication.
//! Run with: cargo run --example test_coinbase_connection

use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load .env file
    if let Err(e) = dotenvy::dotenv() {
        println!("⚠️  Could not load .env file: {}", e);
        println!("   Continuing with environment variables from system\n");
    }

    println!("🔍 Testing Coinbase API Connection...\n");

    // Load credentials from environment
    let api_key = env::var("COINBASE_API_KEY")
        .expect("COINBASE_API_KEY not set. Set it with: export COINBASE_API_KEY=your_key");

    let api_secret = env::var("COINBASE_API_SECRET")
        .expect("COINBASE_API_SECRET not set. Set it with: export COINBASE_API_SECRET=your_secret");

    let passphrase = env::var("COINBASE_PASSPHRASE").ok();

    println!("✅ Credentials loaded:");
    println!("   API Key: {}...{}", &api_key[..8.min(api_key.len())],
             if api_key.len() > 8 { &api_key[api_key.len()-4..] } else { "" });
    println!("   API Secret: {}...", &api_secret[..8.min(api_secret.len())]);
    println!("   Passphrase: {}\n", if passphrase.is_some() { "Set" } else { "Not set" });

    // Import after we know env is set up
    use nzeza::infrastructure::coinbase_client::CoinbaseClient;

    // Create client
    println!("🔌 Creating Coinbase client...");
    let client = CoinbaseClient::new(
        &api_key,
        &api_secret,
        passphrase.as_deref()
    )?;
    println!("✅ Client created successfully\n");

    // Test connection by fetching accounts
    println!("📡 Testing API connection (GET /accounts)...");
    match client.get_accounts().await {
        Ok(accounts) => {
            println!("✅ Connection successful!\n");
            println!("📊 Accounts found: {}\n", accounts.len());

            for (i, account) in accounts.iter().enumerate() {
                if account.balance.parse::<f64>().unwrap_or(0.0) > 0.0 {
                    println!("Account {}:", i + 1);
                    println!("  Currency: {}", account.currency);
                    println!("  Balance: {}", account.balance);
                    println!("  Available: {}", account.available);
                    println!("  Hold: {}", account.hold);
                    println!();
                }
            }

            if accounts.is_empty() {
                println!("ℹ️  No accounts found. This might be normal for a new API key.");
            }

            println!("✅ Coinbase connection test PASSED");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Connection FAILED: {}", e);
            eprintln!("\n🔧 Troubleshooting:");
            eprintln!("   1. Check that your API credentials are correct");
            eprintln!("   2. Verify API key has 'View' permission at minimum");
            eprintln!("   3. Check if IP whitelist is configured (if enabled)");
            eprintln!("   4. Ensure you're using production credentials (not sandbox)");
            eprintln!("\n   Error details: {}", e);

            Err(e.into())
        }
    }
}
