#!/bin/bash
# Run Simmons in LIVE mode with OnchainOS
# WARNING: This uses REAL MONEY!

set -e

cd "$(dirname "$0")/.."

echo "============================================"
echo "  ⚠️  Simmons Dual Brain - LIVE MODE  ⚠️"
echo "============================================"
echo ""
echo "WARNING: This mode uses REAL MONEY!"
echo ""
echo "Config: config/live.toml"
echo "Capital: \$100"
echo "Chains: Solana, Base"
echo "Risk: Conservative (10% max position, 15% drawdown halt)"
echo ""
echo "Dashboard: http://localhost:3456"
echo ""

# Confirmation
read -p "Are you sure you want to start LIVE trading? (yes/no): " confirm
if [ "$confirm" != "yes" ]; then
    echo "Aborted."
    exit 0
fi

echo ""
echo "Starting live trading..."
echo "Press Ctrl+C to stop"
echo "============================================"
echo ""

# Check if binary exists
if [ ! -f "./target/release/simmons" ]; then
    echo "Building Simmons..."
    cargo build --release
fi

# Run with live config (--config is a global arg, before subcommand)
./target/release/simmons --config config/live.toml dual --dashboard
