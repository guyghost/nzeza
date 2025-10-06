#!/bin/bash

# Script to load environment variables from 1Password
# Usage: ./load-env.sh [vault-name]
# If no vault name is provided, it will use the default vault

set -e

VAULT_NAME=${1:-""}

echo "🔐 Loading environment variables from 1Password..."

# Check if 1Password CLI is installed
if ! command -v op &> /dev/null; then
    echo "❌ 1Password CLI (op) is not installed."
    echo "   Please install it from: https://developer.1password.com/docs/cli/"
    exit 1
fi

# Check if user is signed in
if ! op whoami &> /dev/null; then
    echo "❌ You are not signed in to 1Password CLI."
    echo "   Please run: op signin"
    exit 1
fi

# Function to get secret from 1Password
get_secret() {
    local item_name=$1
    local field_name=$2

    if [ -n "$VAULT_NAME" ]; then
        op item get "$item_name" --vault "$VAULT_NAME" --field "$field_name" 2>/dev/null
    else
        op item get "$item_name" --field "$field_name" 2>/dev/null
    fi
}

echo "📋 Retrieving secrets..."

# DYDX Configuration
echo "🔑 Loading DYDX configuration..."
if DYDX_MNEMONIC=$(get_secret "DYDX Trading" "mnemonic" 2>/dev/null); then
    export DYDX_MNEMONIC
    echo "✅ DYDX_MNEMONIC loaded"
else
    echo "⚠️  DYDX_MNEMONIC not found (item: 'DYDX Trading', field: 'mnemonic')"
fi

# Coinbase Configuration
echo "🏦 Loading Coinbase configuration..."
if COINBASE_API_KEY=$(get_secret "Coinbase Pro API" "api_key" 2>/dev/null); then
    export COINBASE_API_KEY
    echo "✅ COINBASE_API_KEY loaded"
else
    echo "⚠️  COINBASE_API_KEY not found (item: 'Coinbase Pro API', field: 'api_key')"
fi

if COINBASE_API_SECRET=$(get_secret "Coinbase Pro API" "api_secret" 2>/dev/null); then
    export COINBASE_API_SECRET
    echo "✅ COINBASE_API_SECRET loaded"
else
    echo "⚠️  COINBASE_API_SECRET not found (item: 'Coinbase Pro API', field: 'api_secret')"
fi

if COINBASE_PASSPHRASE=$(get_secret "Coinbase Pro API" "passphrase" 2>/dev/null); then
    export COINBASE_PASSPHRASE
    echo "✅ COINBASE_PASSPHRASE loaded"
else
    echo "⚠️  COINBASE_PASSPHRASE not found (item: 'Coinbase Pro API', field: 'passphrase') - this is optional"
fi

echo ""
echo "🎯 Environment variables loaded successfully!"
echo ""
echo "💡 Tips:"
echo "   - Run your application: cargo run"
echo "   - Check loaded variables: env | grep -E '(DYDX|COINBASE)'"
echo "   - Clear variables: unset DYDX_MNEMONIC COINBASE_API_KEY COINBASE_API_SECRET COINBASE_PASSPHRASE"
echo ""
echo "🔒 Remember: Never commit the .env file with real values!"