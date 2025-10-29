//! Test Coinbase Advanced Trade API Connection
//!
//! This example tests the Coinbase Advanced Trade API using JWT authentication.
//! Run with: cargo run --example test_coinbase_advanced

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

    println!("🔍 Testing Coinbase Advanced Trade API Connection...\n");

    // Import the Coinbase Advanced client
    use nzeza::infrastructure::coinbase_advanced_client::CoinbaseAdvancedClient;

    // Load credentials from environment
    let api_key = env::var("COINBASE_CLOUD_API_KEY").or_else(|_| env::var("COINBASE_API_KEY"))
        .expect("COINBASE_CLOUD_API_KEY or COINBASE_API_KEY not set. Set it with:\n   export COINBASE_CLOUD_API_KEY='organizations/{org_id}/apiKeys/{key_id}'");

    let api_secret = env::var("COINBASE_CLOUD_API_SECRET").or_else(|_| env::var("COINBASE_API_SECRET"))
        .expect("COINBASE_CLOUD_API_SECRET or COINBASE_API_SECRET not set. Set it with:\n   export COINBASE_CLOUD_API_SECRET='-----BEGIN EC PRIVATE KEY-----\\n...\\n-----END EC PRIVATE KEY-----'");

    println!("✅ Credentials loaded:");
    println!(
        "   API Key: {}",
        if api_key.len() > 20 {
            &api_key[..20]
        } else {
            &api_key
        }
    );
    println!("   API Secret: {} characters\n", api_secret.len());

    // Validate API key format
    if !api_key.starts_with("organizations/") {
        eprintln!("❌ Invalid API key format!");
        eprintln!("   Expected: organizations/{{org_id}}/apiKeys/{{key_id}}");
        eprintln!("   Got: {}", api_key);
        return Err("Invalid API key format".into());
    }

    // Validate API secret format
    if !api_secret.contains("BEGIN") || !api_secret.contains("PRIVATE KEY") {
        eprintln!("❌ Invalid API secret format!");
        eprintln!("   Expected: PEM-encoded EC private key");
        eprintln!("   Got: {} characters", api_secret.len());
        return Err("Invalid API secret format".into());
    }

    println!("🔌 Creating Coinbase Advanced Trade API client...");
    let client = match CoinbaseAdvancedClient::new(&api_key, &api_secret) {
        Ok(client) => {
            println!("✅ Client created successfully\n");
            client
        }
        Err(e) => {
            eprintln!("❌ Failed to create client: {}", e);
            return Err(e.into());
        }
    };

    // Test getting accounts
    println!("📊 Fetching accounts...");
    match client.get_accounts().await {
        Ok(accounts) => {
            println!("✅ Accounts retrieved: {} accounts\n", accounts.len());

            for (i, account) in accounts.iter().take(5).enumerate() {
                println!("Account {}:", i + 1);
                println!("  UUID: {}", account.uuid);
                println!("  Name: {}", account.name);
                println!("  Currency: {}", account.currency);
                println!(
                    "  Available Balance: {} {}",
                    account.available_balance.value, account.available_balance.currency
                );
                println!();
            }
        }
        Err(e) => {
            eprintln!("⚠️  Failed to fetch accounts: {}", e);
            eprintln!("   This might indicate:");
            eprintln!("   - Invalid API credentials");
            eprintln!("   - API key does not have required permissions");
            eprintln!("   - Network connectivity issues");
            return Err(e.into());
        }
    }

    println!("✅ Coinbase Advanced Trade API connection test PASSED");
    println!("\n🎉 You can now use Coinbase Advanced Trade API for trading!");
    println!("\n⚠️  Important notes:");
    println!("   - This uses the NEW Coinbase Cloud API (Advanced Trade)");
    println!("   - Different from the old Coinbase Pro API");
    println!("   - Uses JWT authentication with ES256 signing");
    println!("   - API keys must be in format: organizations/{{org_id}}/apiKeys/{{key_id}}");
    println!("   - API secrets must be PEM-encoded EC private keys");

    Ok(())
}
