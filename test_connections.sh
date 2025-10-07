#!/bin/bash
# Test Connections Script
# This script runs connection tests with .env loaded by the Rust programs

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  CONNECTION TESTS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ ! -f .env ]; then
    echo "❌ .env file not found!"
    echo "   Copy .env.example to .env and configure your credentials"
    exit 1
fi

echo "✅ .env file found"
echo "   (Rust programs will load it automatically via dotenvy)"
echo ""

# Test Coinbase
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  TESTING COINBASE CONNECTION"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Running: cargo run --example test_coinbase_connection"
echo ""

if cargo run --example test_coinbase_connection 2>&1; then
    echo ""
    echo "✅ Coinbase test completed"
else
    echo ""
    echo "⚠️  Coinbase test failed (check if COINBASE_API_KEY is set in .env)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  TESTING DYDX CONNECTION"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Running: cargo run --example test_dydx_connection"
echo ""

if cargo run --example test_dydx_connection 2>&1; then
    echo ""
    echo "✅ dYdX test completed"
else
    echo ""
    echo "⚠️  dYdX test failed (check if DYDX_MNEMONIC is set in .env)"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ALL TESTS COMPLETED"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
