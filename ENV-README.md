# Environment Variables Setup

This project uses environment variables for sensitive configuration like API keys and trading credentials. We provide multiple ways to manage these variables securely.

## Quick Start

1. **Copy the example file:**
   ```bash
   cp .env.example .env
   ```

2. **Fill in your values** in `.env` (or use 1Password CLI below)

3. **Run the application:**
   ```bash
   cargo run
   ```

## Using 1Password CLI (Recommended)

For secure credential management, we recommend using 1Password CLI to automatically load your secrets.

### Prerequisites

1. **Install 1Password CLI:**
   ```bash
   # macOS with Homebrew
   brew install 1password-cli

   # Or download from: https://developer.1password.com/docs/cli/get-started/
   ```

2. **Sign in to 1Password:**
   ```bash
   op signin
   ```

### Setting up Your Secrets in 1Password

Create the following items in your 1Password vault:

#### 1. DYDX Trading Item
- **Item Name:** `DYDX Trading`
- **Fields:**
  - `mnemonic` - Your 12-word seed phrase

#### 2. Coinbase Pro API Item
- **Item Name:** `Coinbase Pro API`
- **Fields:**
  - `api_key` - Your Coinbase API Key
  - `api_secret` - Your Coinbase API Secret
  - `passphrase` - Your Coinbase Passphrase

### Loading Environment Variables

```bash
# Load from default vault
./load-env.sh

# Or specify a vault name
./load-env.sh "My Trading Vault"
```

The script will:
- ✅ Load your secrets from 1Password
- ✅ Set environment variables automatically
- ⚠️  Warn you if any secrets are missing
- 🔒 Keep your credentials secure

### Verifying Setup

```bash
# Check that variables are loaded
env | grep -E "(DYDX|COINBASE)"

# Should show something like:
# DYDX_MNEMONIC=abandon abandon abandon...
# COINBASE_API_KEY=your_key_here
# COINBASE_API_SECRET=your_secret_here
# COINBASE_PASSPHRASE=your_passphrase_here
```

## Manual Configuration

If you prefer not to use 1Password CLI, you can manually set environment variables:

```bash
# DYDX
export DYDX_MNEMONIC="your twelve word mnemonic phrase"

# Coinbase
export COINBASE_API_KEY="your_api_key"
export COINBASE_API_SECRET="your_api_secret"
export COINBASE_PASSPHRASE="your_passphrase"

# Trading configuration (optional)
export MIN_CONFIDENCE_THRESHOLD=0.7
export MAX_POSITIONS_PER_SYMBOL=1
export ENABLE_AUTOMATED_TRADING=true
```

## Security Best Practices

- 🔒 **Never commit** the `.env` file with real values
- 🔑 **Use 1Password CLI** for secure credential management
- 🛡️ **Restrict API permissions** to minimum required (View + Trade only)
- 🌐 **Use IP restrictions** on your API keys when possible
- 🔄 **Rotate credentials** regularly
- 📁 **Keep .env in .gitignore** (it should be already)

## Troubleshooting

### 1Password CLI Issues

```bash
# Check if signed in
op whoami

# List available vaults
op vault list

# List items in a vault
op item list --vault "Your Vault Name"

# Get a specific field
op item get "Item Name" --field "field_name"
```

### Environment Variables Not Loading

```bash
# Check if .env file exists
ls -la .env

# Manually source the file
source .env

# Check variable values
echo $DYDX_MNEMONIC
echo $COINBASE_API_KEY
```

### Permission Issues

```bash
# Make script executable
chmod +x load-env.sh

# Check script permissions
ls -la load-env.sh
```

## Advanced Configuration

### Custom Vault Names

If your 1Password items are in a specific vault:

```bash
# Edit load-env.sh and change VAULT_NAME
VAULT_NAME="My Custom Vault"

# Or pass it as argument
./load-env.sh "My Custom Vault"
```

### Custom Item Names

If you used different item names in 1Password, update the `get_secret` calls in `load-env.sh`:

```bash
# Change from:
get_secret "DYDX Trading" "mnemonic"

# To:
get_secret "My DYDX Item" "seed_phrase"
```

## Support

For issues with 1Password CLI, see the [official documentation](https://developer.1password.com/docs/cli/).

For project-specific issues, check the main README.md or create an issue on GitHub.