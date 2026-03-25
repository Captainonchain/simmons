#!/bin/bash
# Run Simmons in testnet mode with OnchainOS

set -e

cd "$(dirname "$0")/.."

echo "============================================"
echo "  Simmons Dual Brain - TESTNET MODE"
echo "============================================"
echo ""
echo "Config: config/testnet.toml"
echo "Capital: \$50"
echo "Chains: Solana, Base"
echo "Risk: Conservative (20% max position)"
echo ""
echo "Dashboard: http://localhost:3456"
echo ""
echo "Press Ctrl+C to stop"
echo "============================================"
echo ""

# Check if binary exists
if [ ! -f "./target/release/simmons" ]; then
    echo "Building Simmons..."
    cargo build --release
fi

# Run with testnet config (--config is a global arg, before subcommand)
./target/release/simmons --config config/testnet.toml dual --dashboard
