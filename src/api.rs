use crate::store::FxStore;
use crate::types::OHLCV;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub type SharedStore = Arc<FxStore>;

#[derive(Serialize)]
pub struct PriceResponse {
    pub symbol: String,
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u32,
}

#[derive(Serialize)]
pub struct SymbolsResponse {
    pub symbols: Vec<String>,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub start: Option<String>,
    pub end: Option<String>,
    pub limit: Option<usize>,
}

impl From<&OHLCV> for PriceResponse {
    fn from(ohlcv: &OHLCV) -> Self {
        Self {
            symbol: "".to_string(), // Will be filled by handler
            timestamp: (ohlcv.ts / 1_000_000_000) as i64, // Convert to seconds
            open: ohlcv.open as f64 / 100000.0,
            high: ohlcv.high as f64 / 100000.0,
            low: ohlcv.low as f64 / 100000.0,
            close: ohlcv.close as f64 / 100000.0,
            volume: ohlcv.volume,
        }
    }
}

pub fn create_app(store: SharedStore) -> Router {
    Router::new()
        .route("/symbols", get(get_symbols))
        .route("/price/:symbol", get(get_current_price))
        .route("/history/:symbol", get(get_history))
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive())
        .with_state(store)
}

// GET /symbols - List all available symbols
async fn get_symbols(State(store): State<SharedStore>) -> Result<Json<SymbolsResponse>, StatusCode> {
    let symbols = store.get_symbols();
    Ok(Json(SymbolsResponse { symbols }))
}

// GET /price/{symbol} - Get current price for a symbol
async fn get_current_price(
    State(store): State<SharedStore>,
    Path(symbol): Path<String>,
) -> Result<Json<PriceResponse>, StatusCode> {
    let now = Utc::now().timestamp_nanos_opt().unwrap() as u64;
    let one_hour_ago = now - 3600_000_000_000; // 1 hour in nanoseconds

    // Get latest record from last hour
    let records: Vec<OHLCV> = store
        .query_range(&symbol, one_hour_ago, now)
        .collect();

    if let Some(latest) = records.last() {
        let mut response = PriceResponse::from(latest);
        response.symbol = symbol;
        Ok(Json(response))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// GET /history/{symbol}?start=2024-01-01&end=2024-12-31&limit=1000
async fn get_history(
    State(store): State<SharedStore>,
    Path(symbol): Path<String>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<Vec<PriceResponse>>, StatusCode> {
    let end_ts = if let Some(end_str) = &params.end {
        parse_datetime(end_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .timestamp_nanos_opt()
            .unwrap() as u64
    } else {
        Utc::now().timestamp_nanos_opt().unwrap() as u64
    };

    let start_ts = if let Some(start_str) = &params.start {
        parse_datetime(start_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .timestamp_nanos_opt()
            .unwrap() as u64
    } else {
        end_ts - 86400_000_000_000 // Default to 1 day ago
    };

    let mut records: Vec<OHLCV> = store
        .query_range(&symbol, start_ts, end_ts)
        .collect();

    // Apply limit if specified
    if let Some(limit) = params.limit {
        if records.len() > limit {
            let start_idx = records.len() - limit;
            records = records.into_iter().skip(start_idx).collect();
        }
    }

    let responses: Vec<PriceResponse> = records
        .iter()
        .map(|ohlcv| {
            let mut response = PriceResponse::from(ohlcv);
            response.symbol = symbol.clone();
            response
        })
        .collect();

    Ok(Json(responses))
}

// GET /health - Health check
async fn health_check() -> Json<HashMap<String, String>> {
    let mut response = HashMap::new();
    response.insert("status".to_string(), "ok".to_string());
    response.insert("service".to_string(), "fx-store".to_string());
    Json(response)
}

fn parse_datetime(date_str: &str) -> Result<DateTime<Utc>, anyhow::Error> {
    // Try different formats
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Ok(dt.with_timezone(&Utc));
    }
    
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt.and_utc());
    }
    
    if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }
    
    Err(anyhow::anyhow!("Unable to parse date: {}", date_str))
}

pub async fn start_server(store: SharedStore, port: u16) -> anyhow::Result<()> {
    let app = create_app(store);
    
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("🚀 FX-Store API server running on http://0.0.0.0:{}", port);
    
    axum::serve(listener, app).await?;
    Ok(())
}