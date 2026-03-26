#!/bin/bash
# Simmons Dual Brain - X Layer Mainnet Startup Script
# Capital: $50 | Chain: X Layer | Mode: LIVE

set -e

echo "============================================"
echo "  SIMMONS DUAL BRAIN - MAINNET MODE"
echo "  Chain: X Layer | Capital: \$50"
echo "============================================"
echo ""

# Check for required environment variables
if [ -z "$OKX_API_KEY" ]; then
    echo "ERROR: OKX_API_KEY not set"
    echo ""
    echo "Get your API keys from: https://web3.okx.com/build/dev-portal"
    echo ""
    echo "Then run:"
    echo "  export OKX_API_KEY='your-api-key'"
    echo "  export OKX_SECRET_KEY='your-secret-key'"
    echo "  export OKX_PASSPHRASE='your-passphrase'"
    exit 1
fi

if [ -z "$OKX_SECRET_KEY" ]; then
    echo "ERROR: OKX_SECRET_KEY not set"
    exit 1
fi

if [ -z "$OKX_PASSPHRASE" ]; then
    echo "ERROR: OKX_PASSPHRASE not set"
    exit 1
fi

echo "API Keys: OK"
echo ""

# Show risk settings
echo "Risk Settings:"
echo "  - Capital: \$50"
echo "  - Max Position: 20% (\$10)"
echo "  - Max Drawdown: 20% (\$10)"
echo "  - Max Consecutive Losses: 5"
echo "  - Daily Loss Limit: \$20"
echo ""

# Confirm before starting
read -p "Start LIVE trading on X Layer mainnet? (yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    echo "Aborted."
    exit 0
fi

echo ""
echo "Starting Simmons Dual Brain in LIVE mode..."
echo ""

cd /Users/sandeep/simmons

# Start the trading engine with mainnet settings
./target/release/simmons --capital 50 --mode live dual --dashboard

echo "Simmons stopped."
