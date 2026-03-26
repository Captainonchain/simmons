//! DEX/AMM Pool integration for X Layer
//!
//! Supports Uniswap V2/V3 style AMMs deployed on X Layer.

use anyhow::{anyhow, Result};
use ethers::prelude::*;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Known DEX router addresses on X Layer
/// Supports override via environment variables
pub mod routers {
    use ethers::types::Address;
    use std::str::FromStr;
    use std::sync::OnceLock;

    static OKX_DEX: OnceLock<Address> = OnceLock::new();
    static UNISWAP_V2: OnceLock<Address> = OnceLock::new();

    fn parse_address(addr: &str, name: &str) -> Address {
        Address::from_str(addr)
            .unwrap_or_else(|_| panic!("Invalid {} address - build error", name))
    }

    /// OKX DEX aggregator router
    pub fn okx_dex() -> Address {
        *OKX_DEX.get_or_init(|| {
            std::env::var("DEX_OKX_ROUTER")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x1111111254fb6c44bAC0beD2854e76F90643097d", "OKX DEX"))
        })
    }

    /// Uniswap V2 style router (if deployed)
    pub fn uniswap_v2() -> Address {
        *UNISWAP_V2.get_or_init(|| {
            std::env::var("DEX_UNISWAP_V2")
                .ok()
                .and_then(|a| Address::from_str(&a).ok())
                .unwrap_or_else(|| parse_address("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D", "Uniswap V2"))
        })
    }
}

/// Pool type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolType {
    UniswapV2,
    UniswapV3,
    Curve,
    Balancer,
}

/// DEX pool representation
#[derive(Debug, Clone)]
pub struct DexPool {
    pub address: Address,
    pub token0: Address,
    pub token1: Address,
    pub pool_type: PoolType,
    pub fee_bps: u32, // Fee in basis points
}

/// DEX client for interacting with AMMs
pub struct DexClient {
    provider: Arc<Provider<Http>>,
}

impl DexClient {
    pub fn new(provider: Arc<Provider<Http>>) -> Self {
        Self { provider }
    }

    /// Create from RPC URL
    pub fn from_rpc(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    /// Get reserves from a Uniswap V2 style pool
    pub async fn get_v2_reserves(&self, pool: Address) -> Result<PoolReserves> {
        // getReserves() selector: 0x0902f1ac
        let call_data = vec![0x09, 0x02, 0xf1, 0xac];

        let tx = TransactionRequest::new().to(pool).data(call_data);
        let result = self.provider.call(&tx.into(), None).await?;

        if result.len() < 64 {
            return Err(anyhow!("Invalid reserves response"));
        }

        let reserve0 = U256::from_big_endian(&result[0..32]);
        let reserve1 = U256::from_big_endian(&result[32..64]);

        Ok(PoolReserves {
            reserve0: decimal_from_u256(reserve0, 18),
            reserve1: decimal_from_u256(reserve1, 18),
            timestamp: chrono::Utc::now().timestamp() as u64,
        })
    }

    /// Get token0 from pool
    pub async fn get_token0(&self, pool: Address) -> Result<Address> {
        // token0() selector: 0x0dfe1681
        let call_data = vec![0x0d, 0xfe, 0x16, 0x81];

        let tx = TransactionRequest::new().to(pool).data(call_data);
        let result = self.provider.call(&tx.into(), None).await?;

        if result.len() < 32 {
            return Err(anyhow!("Invalid token0 response"));
        }

        Ok(Address::from_slice(&result[12..32]))
    }

    /// Get token1 from pool
    pub async fn get_token1(&self, pool: Address) -> Result<Address> {
        // token1() selector: 0xd21220a7
        let call_data = vec![0xd2, 0x12, 0x20, 0xa7];

        let tx = TransactionRequest::new().to(pool).data(call_data);
        let result = self.provider.call(&tx.into(), None).await?;

        if result.len() < 32 {
            return Err(anyhow!("Invalid token1 response"));
        }

        Ok(Address::from_slice(&result[12..32]))
    }

    /// Calculate output amount for a swap (constant product formula)
    pub fn calculate_output_amount(
        &self,
        amount_in: Decimal,
        reserve_in: Decimal,
        reserve_out: Decimal,
        fee_bps: u32,
    ) -> SwapCalculation {
        if reserve_in.is_zero() || reserve_out.is_zero() {
            return SwapCalculation {
                amount_out: Decimal::ZERO,
                price_impact_bps: Decimal::ZERO,
                effective_price: Decimal::ZERO,
                fee_amount: Decimal::ZERO,
            };
        }

        // Apply fee
        let fee_multiplier = Decimal::from(10000 - fee_bps) / Decimal::from(10000);
        let amount_in_with_fee = amount_in * fee_multiplier;

        // Constant product: x * y = k
        // amount_out = (amount_in_with_fee * reserve_out) / (reserve_in + amount_in_with_fee)
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in + amount_in_with_fee;
        let amount_out = numerator / denominator;

        // Calculate price impact
        let spot_price = reserve_out / reserve_in;
        let effective_price = if amount_in.is_zero() {
            spot_price
        } else {
            amount_out / amount_in
        };

        let price_impact_bps = if spot_price.is_zero() {
            Decimal::ZERO
        } else {
            ((spot_price - effective_price).abs() / spot_price) * Decimal::from(10000)
        };

        let fee_amount = amount_in - amount_in_with_fee;

        SwapCalculation {
            amount_out,
            price_impact_bps,
            effective_price,
            fee_amount,
        }
    }

    /// Get quote for a swap
    pub async fn get_quote(
        &self,
        pool: &DexPool,
        amount_in: Decimal,
        is_token0_in: bool,
    ) -> Result<SwapQuote> {
        let reserves = self.get_v2_reserves(pool.address).await?;

        let (reserve_in, reserve_out) = if is_token0_in {
            (reserves.reserve0, reserves.reserve1)
        } else {
            (reserves.reserve1, reserves.reserve0)
        };

        let calc = self.calculate_output_amount(amount_in, reserve_in, reserve_out, pool.fee_bps);

        Ok(SwapQuote {
            pool_address: pool.address,
            token_in: if is_token0_in { pool.token0 } else { pool.token1 },
            token_out: if is_token0_in { pool.token1 } else { pool.token0 },
            amount_in,
            amount_out: calc.amount_out,
            price_impact_bps: calc.price_impact_bps,
            fee_amount: calc.fee_amount,
            effective_price: calc.effective_price,
        })
    }

    /// Create swap transaction data (Uniswap V2 style)
    pub fn create_swap_tx(
        &self,
        router: Address,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        recipient: Address,
        deadline: U256,
    ) -> Result<TransactionRequest> {
        // swapExactTokensForTokens(uint256,uint256,address[],address,uint256)
        let selector = ethers::utils::id("swapExactTokensForTokens(uint256,uint256,address[],address,uint256)");

        let path_tokens: Vec<ethers::abi::Token> = path
            .iter()
            .map(|a| ethers::abi::Token::Address(*a))
            .collect();

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Uint(amount_in),
            ethers::abi::Token::Uint(amount_out_min),
            ethers::abi::Token::Array(path_tokens),
            ethers::abi::Token::Address(recipient),
            ethers::abi::Token::Uint(deadline),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        Ok(TransactionRequest::new().to(router).data(tx_data))
    }

    /// Create swap transaction for ETH input
    pub fn create_swap_eth_tx(
        &self,
        router: Address,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        recipient: Address,
        deadline: U256,
    ) -> Result<TransactionRequest> {
        // swapExactETHForTokens(uint256,address[],address,uint256)
        let selector = ethers::utils::id("swapExactETHForTokens(uint256,address[],address,uint256)");

        let path_tokens: Vec<ethers::abi::Token> = path
            .iter()
            .map(|a| ethers::abi::Token::Address(*a))
            .collect();

        let data = ethers::abi::encode(&[
            ethers::abi::Token::Uint(amount_out_min),
            ethers::abi::Token::Array(path_tokens),
            ethers::abi::Token::Address(recipient),
            ethers::abi::Token::Uint(deadline),
        ]);

        let mut tx_data = selector[..4].to_vec();
        tx_data.extend_from_slice(&data);

        Ok(TransactionRequest::new()
            .to(router)
            .data(tx_data)
            .value(amount_in))
    }

    /// Find best route across multiple pools
    pub async fn find_best_route(
        &self,
        pools: &[DexPool],
        token_in: Address,
        token_out: Address,
        amount_in: Decimal,
    ) -> Result<Option<SwapRoute>> {
        let mut best_route: Option<SwapRoute> = None;

        // Direct routes
        for pool in pools {
            if (pool.token0 == token_in && pool.token1 == token_out)
                || (pool.token1 == token_in && pool.token0 == token_out)
            {
                let is_token0_in = pool.token0 == token_in;
                if let Ok(quote) = self.get_quote(pool, amount_in, is_token0_in).await {
                    let route = SwapRoute {
                        hops: vec![quote.clone()],
                        total_amount_out: quote.amount_out,
                        total_price_impact_bps: quote.price_impact_bps,
                    };

                    if best_route
                        .as_ref()
                        .map_or(true, |r| route.total_amount_out > r.total_amount_out)
                    {
                        best_route = Some(route);
                    }
                }
            }
        }

        // Multi-hop routes (token_in -> intermediate -> token_out)
        // Common intermediates: WETH, USDC, USDT, WBTC
        let intermediates: Vec<Address> = pools
            .iter()
            .flat_map(|p| vec![p.token0, p.token1])
            .filter(|&t| t != token_in && t != token_out)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for intermediate in intermediates {
            // Find pool for first hop: token_in -> intermediate
            let first_hop_pool = pools.iter().find(|p| {
                (p.token0 == token_in && p.token1 == intermediate)
                    || (p.token1 == token_in && p.token0 == intermediate)
            });

            // Find pool for second hop: intermediate -> token_out
            let second_hop_pool = pools.iter().find(|p| {
                (p.token0 == intermediate && p.token1 == token_out)
                    || (p.token1 == intermediate && p.token0 == token_out)
            });

            if let (Some(pool1), Some(pool2)) = (first_hop_pool, second_hop_pool) {
                // Calculate first hop
                let is_token0_in_hop1 = pool1.token0 == token_in;
                if let Ok(quote1) = self.get_quote(pool1, amount_in, is_token0_in_hop1).await {
                    // Use output of first hop as input for second hop
                    let is_token0_in_hop2 = pool2.token0 == intermediate;
                    if let Ok(quote2) = self.get_quote(pool2, quote1.amount_out, is_token0_in_hop2).await {
                        // Combined price impact (additive for simplicity)
                        let combined_impact = quote1.price_impact_bps + quote2.price_impact_bps;
                        let total_out = quote2.amount_out;

                        let route = SwapRoute {
                            hops: vec![quote1, quote2],
                            total_amount_out: total_out,
                            total_price_impact_bps: combined_impact,
                        };

                        // Compare with current best (considering price impact)
                        if best_route.as_ref().map_or(true, |r| {
                            route.total_amount_out > r.total_amount_out
                        }) {
                            best_route = Some(route);
                        }
                    }
                }
            }
        }

        Ok(best_route)
    }
}

/// Pool reserves snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolReserves {
    pub reserve0: Decimal,
    pub reserve1: Decimal,
    pub timestamp: u64,
}

impl PoolReserves {
    /// Get price of token0 in terms of token1
    pub fn price_0_in_1(&self) -> Decimal {
        if self.reserve0.is_zero() {
            return Decimal::ZERO;
        }
        self.reserve1 / self.reserve0
    }

    /// Get price of token1 in terms of token0
    pub fn price_1_in_0(&self) -> Decimal {
        if self.reserve1.is_zero() {
            return Decimal::ZERO;
        }
        self.reserve0 / self.reserve1
    }

    /// Calculate liquidity (sqrt(reserve0 * reserve1))
    pub fn liquidity(&self) -> Decimal {
        let product = self.reserve0 * self.reserve1;
        // Approximate square root using Newton's method
        if product.is_zero() {
            return Decimal::ZERO;
        }
        let mut x = product;
        for _ in 0..20 {
            x = (x + product / x) / Decimal::from(2);
        }
        x
    }
}

/// Swap calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapCalculation {
    pub amount_out: Decimal,
    pub price_impact_bps: Decimal,
    pub effective_price: Decimal,
    pub fee_amount: Decimal,
}

/// Swap quote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub pool_address: Address,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
    pub price_impact_bps: Decimal,
    pub fee_amount: Decimal,
    pub effective_price: Decimal,
}

/// Multi-hop swap route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRoute {
    pub hops: Vec<SwapQuote>,
    pub total_amount_out: Decimal,
    pub total_price_impact_bps: Decimal,
}

impl SwapRoute {
    /// Get all token addresses in the path
    pub fn path(&self) -> Vec<Address> {
        if self.hops.is_empty() {
            return vec![];
        }

        let mut path = vec![self.hops[0].token_in];
        for hop in &self.hops {
            path.push(hop.token_out);
        }
        path
    }
}

/// Helper to convert U256 to Decimal with decimals
fn decimal_from_u256(value: U256, decimals: u32) -> Decimal {
    let divisor = Decimal::from(10u64.pow(decimals));
    let value_str = value.to_string();
    Decimal::from_str(&value_str)
        .map(|d| d / divisor)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_swap_calculation() {
        let client = DexClient {
            provider: Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap()),
        };

        // Pool with 100 ETH and 300,000 USDT (ETH = $3000)
        let calc = client.calculate_output_amount(
            dec!(1),         // 1 ETH in
            dec!(100),       // 100 ETH reserve
            dec!(300000),    // 300k USDT reserve
            30,              // 0.3% fee
        );

        // Should get approximately $2970 out (slight slippage + fee)
        assert!(calc.amount_out > dec!(2900));
        assert!(calc.amount_out < dec!(3000));
        assert!(calc.price_impact_bps > Decimal::ZERO);
    }

    #[test]
    fn test_pool_reserves() {
        let reserves = PoolReserves {
            reserve0: dec!(100),   // 100 ETH
            reserve1: dec!(300000), // 300k USDT
            timestamp: 0,
        };

        assert_eq!(reserves.price_0_in_1(), dec!(3000)); // 1 ETH = 3000 USDT
    }

    #[test]
    fn test_router_addresses() {
        let okx = routers::okx_dex();
        assert_ne!(okx, Address::zero());
    }
}
