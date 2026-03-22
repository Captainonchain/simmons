//! OKX WebSocket feed implementation

use anyhow::{anyhow, Result};
use crossbeam::channel::{bounded, Receiver, Sender};
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use simmons_core::{OrderBook, BookLevel, PriceTick, Source};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// OKX WebSocket feed
pub struct OkxFeed {
    url: String,
    subscriptions: Arc<RwLock<HashSet<String>>>,
    tick_tx: broadcast::Sender<PriceTick>,
    book_tx: broadcast::Sender<OrderBook>,
    running: Arc<RwLock<bool>>,
}

impl OkxFeed {
    pub fn new(url: &str) -> Self {
        let (tick_tx, _) = broadcast::channel(1000);
        let (book_tx, _) = broadcast::channel(100);

        Self {
            url: url.to_string(),
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
            tick_tx,
            book_tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get a receiver for price ticks
    pub fn tick_receiver(&self) -> broadcast::Receiver<PriceTick> {
        self.tick_tx.subscribe()
    }

    /// Get a receiver for order book updates
    pub fn book_receiver(&self) -> broadcast::Receiver<OrderBook> {
        self.book_tx.subscribe()
    }

    /// Connect and start receiving data
    pub async fn connect(&self, symbols: &[&str]) -> Result<()> {
        let url = &self.url;
        info!("Connecting to OKX WebSocket: {}", url);

        let (ws_stream, _) = connect_async(url).await.map_err(|e| anyhow!("WebSocket connect error: {}", e))?;
        let (mut write, mut read) = ws_stream.split();

        *self.running.write() = true;

        // Subscribe to tickers and order books
        for symbol in symbols {
            let inst_id = format!("{}", symbol.replace("-", "-")); // OKX uses BTC-USDT format

            // Subscribe to ticker
            let ticker_sub = OkxSubscription {
                op: "subscribe".to_string(),
                args: vec![OkxArg {
                    channel: "tickers".to_string(),
                    inst_id: inst_id.clone(),
                }],
            };
            let msg = serde_json::to_string(&ticker_sub)?;
            write.send(Message::Text(msg)).await?;
            debug!("Subscribed to ticker: {}", inst_id);

            // Subscribe to order book (top 5 levels)
            let book_sub = OkxSubscription {
                op: "subscribe".to_string(),
                args: vec![OkxArg {
                    channel: "books5".to_string(),
                    inst_id: inst_id.clone(),
                }],
            };
            let msg = serde_json::to_string(&book_sub)?;
            write.send(Message::Text(msg)).await?;
            debug!("Subscribed to book: {}", inst_id);

            self.subscriptions.write().insert(symbol.to_string());
        }

        let tick_tx = self.tick_tx.clone();
        let book_tx = self.book_tx.clone();
        let running = self.running.clone();

        // Spawn message handler
        tokio::spawn(async move {
            while *running.read() {
                match read.next().await {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = Self::handle_message(&text, &tick_tx, &book_tx) {
                            debug!("Message parse error: {}", e);
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        debug!("Received ping");
                    }
                    Some(Ok(Message::Close(_))) => {
                        warn!("WebSocket closed");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        warn!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }
            *running.write() = false;
        });

        info!("OKX feed connected with {} symbols", symbols.len());
        Ok(())
    }

    fn handle_message(
        text: &str,
        tick_tx: &broadcast::Sender<PriceTick>,
        book_tx: &broadcast::Sender<OrderBook>,
    ) -> Result<()> {
        // Try parsing as ticker
        if let Ok(ticker_msg) = serde_json::from_str::<OkxTickerMessage>(text) {
            if ticker_msg.arg.channel == "tickers" {
                for data in ticker_msg.data {
                    let tick = PriceTick {
                        symbol: data.inst_id,
                        price: Decimal::from_str(&data.last).unwrap_or_default(),
                        bid: Decimal::from_str(&data.bid_px).unwrap_or_default(),
                        ask: Decimal::from_str(&data.ask_px).unwrap_or_default(),
                        volume_24h: Decimal::from_str(&data.vol_24h).unwrap_or_default(),
                        timestamp: data.ts.parse().unwrap_or(0),
                        source: Source::Okx,
                    };
                    let _ = tick_tx.send(tick);
                }
            }
        }

        // Try parsing as order book
        if let Ok(book_msg) = serde_json::from_str::<OkxBookMessage>(text) {
            if book_msg.arg.channel == "books5" {
                for data in book_msg.data {
                    let bids: Vec<BookLevel> = data
                        .bids
                        .iter()
                        .filter_map(|b| {
                            Some(BookLevel {
                                price: Decimal::from_str(&b[0]).ok()?,
                                size: Decimal::from_str(&b[1]).ok()?,
                            })
                        })
                        .collect();

                    let asks: Vec<BookLevel> = data
                        .asks
                        .iter()
                        .filter_map(|a| {
                            Some(BookLevel {
                                price: Decimal::from_str(&a[0]).ok()?,
                                size: Decimal::from_str(&a[1]).ok()?,
                            })
                        })
                        .collect();

                    let book = OrderBook {
                        symbol: book_msg.arg.inst_id.clone(),
                        bids,
                        asks,
                        timestamp: data.ts.parse().unwrap_or(0),
                    };
                    let _ = book_tx.send(book);
                }
            }
        }

        Ok(())
    }

    /// Check if feed is connected
    pub fn is_connected(&self) -> bool {
        *self.running.read()
    }

    /// Stop the feed
    pub fn stop(&self) {
        *self.running.write() = false;
    }
}

// OKX message types
#[derive(Debug, Serialize)]
struct OkxSubscription {
    op: String,
    args: Vec<OkxArg>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OkxArg {
    channel: String,
    #[serde(rename = "instId")]
    inst_id: String,
}

#[derive(Debug, Deserialize)]
struct OkxTickerMessage {
    arg: OkxArg,
    data: Vec<OkxTickerData>,
}

#[derive(Debug, Deserialize)]
struct OkxTickerData {
    #[serde(rename = "instId")]
    inst_id: String,
    last: String,
    #[serde(rename = "bidPx")]
    bid_px: String,
    #[serde(rename = "askPx")]
    ask_px: String,
    #[serde(rename = "vol24h")]
    vol_24h: String,
    ts: String,
}

#[derive(Debug, Deserialize)]
struct OkxBookMessage {
    arg: OkxArg,
    data: Vec<OkxBookData>,
}

#[derive(Debug, Deserialize)]
struct OkxBookData {
    bids: Vec<Vec<String>>,
    asks: Vec<Vec<String>>,
    ts: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ticker() {
        let json = r#"{
            "arg": {"channel": "tickers", "instId": "BTC-USDT"},
            "data": [{
                "instId": "BTC-USDT",
                "last": "67234.5",
                "bidPx": "67230.0",
                "askPx": "67240.0",
                "vol24h": "12345.67",
                "ts": "1711234567890"
            }]
        }"#;

        let msg: OkxTickerMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.data[0].inst_id, "BTC-USDT");
        assert_eq!(msg.data[0].last, "67234.5");
    }
}
