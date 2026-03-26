//! OKX Exchange API Client
//!
//! Handles order placement and management on OKX CEX.

use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, info, warn};

/// OKX API configuration
#[derive(Debug, Clone)]
pub struct OkxApiConfig {
    /// API Key
    pub api_key: String,
    /// API Secret
    pub api_secret: String,
    /// Passphrase
    pub passphrase: String,
    /// Use demo/simulated trading
    pub simulated: bool,
    /// Base URL
    pub base_url: String,
}

impl OkxApiConfig {
    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OKX_API_KEY")
            .map_err(|_| anyhow!("OKX_API_KEY not set"))?;
        let api_secret = std::env::var("OKX_API_SECRET")
            .map_err(|_| anyhow!("OKX_API_SECRET not set"))?;
        let passphrase = std::env::var("OKX_PASSPHRASE")
            .map_err(|_| anyhow!("OKX_PASSPHRASE not set"))?;
        let simulated = std::env::var("OKX_SIMULATED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Ok(Self {
            api_key,
            api_secret,
            passphrase,
            simulated,
            base_url: "https://www.okx.com".to_string(),
        })
    }

    /// Create demo/simulated trading config
    pub fn demo() -> Result<Self> {
        let mut config = Self::from_env()?;
        config.simulated = true;
        Ok(config)
    }
}

/// OKX API Client
pub struct OkxApiClient {
    config: OkxApiConfig,
    client: reqwest::Client,
}

impl OkxApiClient {
    pub fn new(config: OkxApiConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        info!(
            "OKX API client initialized (simulated={})",
            config.simulated
        );

        Self { config, client }
    }

    /// Generate signature for OKX API
    fn sign(&self, timestamp: &str, method: &str, path: &str, body: &str) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        use base64::{Engine as _, engine::general_purpose::STANDARD};

        let prehash = format!("{}{}{}{}", timestamp, method, path, body);
        let mut mac = Hmac::<Sha256>::new_from_slice(self.config.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(prehash.as_bytes());
        let result = mac.finalize();
        STANDARD.encode(result.into_bytes())
    }

    /// Build authenticated headers
    fn build_headers(&self, method: &str, path: &str, body: &str) -> Result<HeaderMap> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let signature = self.sign(&timestamp, method, path, body);

        let mut headers = HeaderMap::new();
        headers.insert("OK-ACCESS-KEY", HeaderValue::from_str(&self.config.api_key)?);
        headers.insert("OK-ACCESS-SIGN", HeaderValue::from_str(&signature)?);
        headers.insert("OK-ACCESS-TIMESTAMP", HeaderValue::from_str(&timestamp)?);
        headers.insert("OK-ACCESS-PASSPHRASE", HeaderValue::from_str(&self.config.passphrase)?);
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if self.config.simulated {
            headers.insert("x-simulated-trading", HeaderValue::from_static("1"));
        }

        Ok(headers)
    }

    /// Place a market order
    pub async fn place_market_order(
        &self,
        symbol: &str,
        side: OrderSide,
        size: Decimal,
    ) -> Result<OrderResponse> {
        let order_req = PlaceOrderRequest {
            inst_id: symbol.to_string(),
            td_mode: "cash".to_string(), // Spot trading
            side: side.to_string(),
            ord_type: "market".to_string(),
            sz: size.to_string(),
            px: None,
            cl_ord_id: None,
        };

        self.place_order(order_req).await
    }

    /// Place a limit order
    pub async fn place_limit_order(
        &self,
        symbol: &str,
        side: OrderSide,
        size: Decimal,
        price: Decimal,
    ) -> Result<OrderResponse> {
        let order_req = PlaceOrderRequest {
            inst_id: symbol.to_string(),
            td_mode: "cash".to_string(),
            side: side.to_string(),
            ord_type: "limit".to_string(),
            sz: size.to_string(),
            px: Some(price.to_string()),
            cl_ord_id: None,
        };

        self.place_order(order_req).await
    }

    /// Place order (internal)
    async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderResponse> {
        let path = "/api/v5/trade/order";
        let body = serde_json::to_string(&request)?;

        debug!("Placing order: {}", body);

        let headers = self.build_headers("POST", path, &body)?;
        let url = format!("{}{}", self.config.base_url, path);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        debug!("OKX response: {}", text);

        if !status.is_success() {
            return Err(anyhow!("OKX API error ({}): {}", status, text));
        }

        let api_response: ApiResponse<Vec<OrderResponse>> = serde_json::from_str(&text)?;

        if api_response.code != "0" {
            return Err(anyhow!("OKX error: {} - {}", api_response.code, api_response.msg));
        }

        api_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No order data in response"))
    }

    /// Cancel an order
    pub async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<CancelResponse> {
        let path = "/api/v5/trade/cancel-order";
        let request = CancelOrderRequest {
            inst_id: symbol.to_string(),
            ord_id: Some(order_id.to_string()),
            cl_ord_id: None,
        };
        let body = serde_json::to_string(&request)?;

        let headers = self.build_headers("POST", path, &body)?;
        let url = format!("{}{}", self.config.base_url, path);

        let response = self
            .client
            .post(&url)
            .headers(headers)
            .body(body)
            .send()
            .await?;

        let text = response.text().await?;
        let api_response: ApiResponse<Vec<CancelResponse>> = serde_json::from_str(&text)?;

        if api_response.code != "0" {
            return Err(anyhow!("Cancel error: {} - {}", api_response.code, api_response.msg));
        }

        api_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No cancel data in response"))
    }

    /// Get order details
    pub async fn get_order(&self, symbol: &str, order_id: &str) -> Result<OrderDetail> {
        let path = format!("/api/v5/trade/order?instId={}&ordId={}", symbol, order_id);
        let headers = self.build_headers("GET", &path, "")?;
        let url = format!("{}{}", self.config.base_url, path);

        let response = self.client.get(&url).headers(headers).send().await?;
        let text = response.text().await?;
        let api_response: ApiResponse<Vec<OrderDetail>> = serde_json::from_str(&text)?;

        if api_response.code != "0" {
            return Err(anyhow!("Get order error: {}", api_response.msg));
        }

        api_response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Order not found"))
    }

    /// Get account balance
    pub async fn get_balance(&self, currency: Option<&str>) -> Result<Vec<BalanceData>> {
        let path = match currency {
            Some(ccy) => format!("/api/v5/account/balance?ccy={}", ccy),
            None => "/api/v5/account/balance".to_string(),
        };

        let headers = self.build_headers("GET", &path, "")?;
        let url = format!("{}{}", self.config.base_url, path);

        let response = self.client.get(&url).headers(headers).send().await?;
        let text = response.text().await?;

        debug!("Balance response: {}", text);

        let api_response: ApiResponse<Vec<AccountBalance>> = serde_json::from_str(&text)?;

        if api_response.code != "0" {
            return Err(anyhow!("Balance error: {}", api_response.msg));
        }

        Ok(api_response
            .data
            .into_iter()
            .flat_map(|ab| ab.details)
            .collect())
    }

    /// Wait for order to fill
    pub async fn wait_for_fill(
        &self,
        symbol: &str,
        order_id: &str,
        timeout_secs: u64,
    ) -> Result<OrderDetail> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            let order = self.get_order(symbol, order_id).await?;

            match order.state.as_str() {
                "filled" => {
                    info!("Order {} filled at avg price {}", order_id, order.avg_px);
                    return Ok(order);
                }
                "canceled" | "cancelled" => {
                    return Err(anyhow!("Order was cancelled"));
                }
                "live" | "partially_filled" => {
                    if start.elapsed() > timeout {
                        return Err(anyhow!("Order fill timeout"));
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                state => {
                    return Err(anyhow!("Unexpected order state: {}", state));
                }
            }
        }
    }
}

/// Order side
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl ToString for OrderSide {
    fn to_string(&self) -> String {
        match self {
            OrderSide::Buy => "buy".to_string(),
            OrderSide::Sell => "sell".to_string(),
        }
    }
}

impl From<simmons_core::Side> for OrderSide {
    fn from(side: simmons_core::Side) -> Self {
        match side {
            simmons_core::Side::Long => OrderSide::Buy,
            simmons_core::Side::Short => OrderSide::Sell,
        }
    }
}

// Request/Response types

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaceOrderRequest {
    inst_id: String,
    td_mode: String,
    side: String,
    ord_type: String,
    sz: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    px: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cl_ord_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CancelOrderRequest {
    inst_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ord_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cl_ord_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    code: String,
    msg: String,
    data: T,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    pub ord_id: String,
    pub cl_ord_id: String,
    pub tag: String,
    pub s_code: String,
    pub s_msg: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelResponse {
    pub ord_id: String,
    pub cl_ord_id: String,
    pub s_code: String,
    pub s_msg: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetail {
    pub inst_id: String,
    pub ord_id: String,
    pub cl_ord_id: String,
    pub side: String,
    pub ord_type: String,
    pub sz: String,
    pub px: String,
    pub avg_px: String,
    pub acc_fill_sz: String,
    pub state: String,
    pub fee: String,
    pub fee_ccy: String,
    pub u_time: String,
    pub c_time: String,
}

impl OrderDetail {
    pub fn fill_price(&self) -> Decimal {
        Decimal::from_str(&self.avg_px).unwrap_or_default()
    }

    pub fn fill_size(&self) -> Decimal {
        Decimal::from_str(&self.acc_fill_sz).unwrap_or_default()
    }

    pub fn fee(&self) -> Decimal {
        Decimal::from_str(&self.fee).unwrap_or_default()
    }

    pub fn is_filled(&self) -> bool {
        self.state == "filled"
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountBalance {
    details: Vec<BalanceData>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceData {
    pub ccy: String,
    pub bal: String,
    pub avail_bal: String,
    pub frozen_bal: String,
}

impl BalanceData {
    pub fn available(&self) -> Decimal {
        Decimal::from_str(&self.avail_bal).unwrap_or_default()
    }

    pub fn total(&self) -> Decimal {
        Decimal::from_str(&self.bal).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_side_conversion() {
        assert_eq!(OrderSide::Buy.to_string(), "buy");
        assert_eq!(OrderSide::Sell.to_string(), "sell");
    }

    #[test]
    fn test_order_side_from_core() {
        let buy: OrderSide = simmons_core::Side::Long.into();
        let sell: OrderSide = simmons_core::Side::Short.into();
        assert_eq!(buy, OrderSide::Buy);
        assert_eq!(sell, OrderSide::Sell);
    }
}
