#!/bin/bash
# Test OnchainOS integration before running live trading

set -e

ONCHAINOS=~/.local/bin/onchainos

echo "============================================"
echo "  Simmons OnchainOS Integration Test"
echo "============================================"
echo ""

# 1. Check CLI exists
echo "1. Checking OnchainOS CLI..."
if [ ! -f "$ONCHAINOS" ]; then
    echo "   ❌ OnchainOS CLI not found at $ONCHAINOS"
    echo "   Install: curl -sSL https://web3.okx.com/onchainos/install | sh"
    exit 1
fi
echo "   ✅ CLI found"

# 2. Check wallet login
echo ""
echo "2. Checking wallet login..."
STATUS=$($ONCHAINOS wallet status 2>&1)
LOGGED_IN=$(echo "$STATUS" | grep -o '"loggedIn": *[^,}]*' | grep -o 'true\|false')

if [ "$LOGGED_IN" != "true" ]; then
    echo "   ❌ Not logged in"
    echo "   Run: $ONCHAINOS wallet login YOUR_EMAIL"
    exit 1
fi
echo "   ✅ Logged in"

# 3. Get wallet addresses
echo ""
echo "3. Getting wallet addresses..."
ADDRESSES=$($ONCHAINOS wallet addresses 2>&1)

SOL_ADDR=$(echo "$ADDRESSES" | grep -A2 '"solana"' | grep '"address"' | head -1 | sed 's/.*"address": *"\([^"]*\)".*/\1/')
EVM_ADDR=$(echo "$ADDRESSES" | grep -A2 '"evm"' | grep '"address"' | head -1 | sed 's/.*"address": *"\([^"]*\)".*/\1/')

echo "   Solana: $SOL_ADDR"
echo "   EVM:    $EVM_ADDR"

# 4. Check balances
echo ""
echo "4. Checking balances..."
BALANCES=$($ONCHAINOS wallet balance 2>&1)
TOTAL=$(echo "$BALANCES" | grep -o '"totalValueUsd": *"[^"]*"' | sed 's/.*"\([0-9.]*\)".*/\1/')
echo "   Total Value: \$$TOTAL"

if [ "$TOTAL" = "0.00" ]; then
    echo ""
    echo "   ⚠️  Wallet is empty! Fund before trading:"
    echo ""
    echo "   Solana (SOL + USDC):"
    echo "   $SOL_ADDR"
    echo ""
    echo "   EVM (ETH + USDC on Base/Arbitrum):"
    echo "   $EVM_ADDR"
    echo ""
fi

# 5. Test swap quote (Solana)
echo ""
echo "5. Testing swap quote (Solana SOL→USDC)..."
QUOTE=$($ONCHAINOS swap quote \
    --chain 501 \
    --from So11111111111111111111111111111111111111112 \
    --to EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
    --amount 0.01 2>&1 || echo '{"ok":false}')

if echo "$QUOTE" | grep -q '"ok": *true'; then
    TO_AMOUNT=$(echo "$QUOTE" | grep -o '"toTokenAmount": *"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)".*/\1/')
    echo "   ✅ Quote works: 0.01 SOL → $TO_AMOUNT USDC"
else
    echo "   ⚠️  Quote failed (may need funds or API issue)"
fi

# 6. Test swap quote (Base)
echo ""
echo "6. Testing swap quote (Base ETH→USDC)..."
QUOTE=$($ONCHAINOS swap quote \
    --chain 8453 \
    --from 0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee \
    --to 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
    --amount 0.001 2>&1 || echo '{"ok":false}')

if echo "$QUOTE" | grep -q '"ok": *true'; then
    TO_AMOUNT=$(echo "$QUOTE" | grep -o '"toTokenAmount": *"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)".*/\1/')
    echo "   ✅ Quote works: 0.001 ETH → $TO_AMOUNT USDC"
else
    echo "   ⚠️  Quote failed (may need funds or API issue)"
fi

echo ""
echo "============================================"
echo "  Integration Test Complete"
echo "============================================"
echo ""

if [ "$TOTAL" = "0.00" ]; then
    echo "⚠️  NEXT STEP: Fund your wallet before trading"
    echo ""
    echo "Recommended for testing:"
    echo "  • Solana: 0.1 SOL + 10 USDC"
    echo "  • Base:   0.01 ETH + 10 USDC"
else
    echo "✅ Ready for trading!"
    echo ""
    echo "Run testnet:"
    echo "  ./scripts/run-testnet.sh"
    echo ""
    echo "Run live:"
    echo "  ./scripts/run-live.sh"
fi
