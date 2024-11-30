use crate::BinanceTrade;
use crate::Message;
use crate::{BinanceCandle, CandleType, Candlestick};
use async_stream::stream;
use futures_util::Stream; // Add this at the top with other imports
use iced::futures::{channel::mpsc, StreamExt};
use iced::time::{self, Duration, Instant};
use reqwest::Url;
use std::collections::{BTreeMap, HashMap};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as ME};
pub fn binance_connection() -> impl Stream<Item = Message> {
    stream! {
        let (tx, mut rx) = mpsc::channel(100);
        let mut current_coin = "btcusdt".to_string();
        let mut last_prices: HashMap<String, f64> = HashMap::new();

        yield Message::WebSocketInit(tx.clone());

        loop {
            let url = Url::parse(&format!(
                "wss://fstream.binance.com/ws/{}@aggTrade",  // @trade를 @aggTrade로 변경
                current_coin.to_lowercase()
            )).unwrap();

            match connect_async(url).await {
                Ok((mut ws_stream, _)) => {
                    println!("Connected to futures stream for {}", current_coin);

                    loop {
                        tokio::select! {
                            Some(new_coin) = rx.next() => {
                                println!("Switching to futures coin: {}", new_coin);
                                current_coin = format!("{}usdt", new_coin.to_lowercase());
                                break;
                            }
                            Some(msg) = ws_stream.next() => {
                                match msg {
                                    Ok(ME::Text(text)) => {
                                        // println!("Received message: {}", text);  // 디버그용
                                        if let Ok(trade) = serde_json::from_str::<BinanceTrade>(&text) {
                                            let symbol = trade.symbol.replace("USDT", "");

                                            if let Ok(price) = trade.price.parse::<f64>() {
                                                let prev_price = *last_prices.get(&symbol).unwrap_or(&price);
                                                let change_percent = if prev_price != 0.0 {
                                                    ((price - prev_price) / prev_price) * 100.0
                                                } else {
                                                    0.0
                                                };

                                                last_prices.insert(symbol.clone(), price);
                                                // println!("Price update: {} -> {}", symbol, price);  // 디버그용

                                                yield Message::UpdatePrice(
                                                    symbol.clone(),
                                                    price,
                                                    change_percent
                                                );
                                                yield Message::AddCandlestick((
                                                    trade.transaction_time as u64,
                                                    trade.clone()
                                                ));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("Futures WebSocket error: {}", e);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    let _ = ws_stream.close(None).await;
                }
                Err(e) => {
                    println!("Futures connection error: {}", e);
                    yield Message::Error;
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}
pub async fn fetch_candles_async(
    market: &str,
    candle_type: &CandleType,
    to_date: Option<String>,
) -> Result<BTreeMap<u64, Candlestick>, Box<dyn std::error::Error>> {
    let count = match candle_type {
        CandleType::Day => 1000,
        CandleType::Minute1 => 1000,
        CandleType::Minute3 => 1000,
    };

    let binance_symbol = match market.split('-').last() {
        Some(symbol) => format!("{}USDT", symbol),
        None => "BTCUSDT".to_string(),
    };

    let interval = match candle_type {
        CandleType::Minute1 => "1m",
        CandleType::Minute3 => "3m",
        CandleType::Day => "1d",
    };

    let url = format!(
        "https://fapi.binance.com/fapi/v1/klines?symbol={}&interval={}&limit={}",
        binance_symbol, interval, count
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        let error_msg = format!("API error: {}", response.status());
        println!("{}", error_msg);
        return Err(error_msg.into());
    }

    let text = response.text().await?;
    let candles: Vec<BinanceCandle> = serde_json::from_str(&text)?;

    let result: BTreeMap<u64, Candlestick> = candles
        .into_iter()
        .filter(|candle| {
            candle.open.parse::<f32>().unwrap_or(0.0) > 0.0
                && candle.high.parse::<f32>().unwrap_or(0.0) > 0.0
                && candle.low.parse::<f32>().unwrap_or(0.0) > 0.0
                && candle.close.parse::<f32>().unwrap_or(0.0) > 0.0
        })
        .map(|candle| {
            (
                candle.open_time,
                Candlestick {
                    open: candle.open.parse().unwrap_or(0.0),
                    high: candle.high.parse().unwrap_or(0.0),
                    low: candle.low.parse().unwrap_or(0.0),
                    close: candle.close.parse().unwrap_or(0.0),
                    volume: candle.volume.parse().unwrap_or(0.0),
                },
            )
        })
        .collect();

    if result.is_empty() {
        Err("No valid candles returned".into())
    } else {
        Ok(result)
    }
}
pub fn fetch_candles(
    market: &str,
    candle_type: &CandleType,
    to_date: Option<String>, // 추가
) -> Result<BTreeMap<u64, Candlestick>, Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(fetch_candles_async(market, candle_type, to_date))
}