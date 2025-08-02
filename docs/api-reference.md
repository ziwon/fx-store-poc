# API Reference

## Overview

FX-Store provides both HTTP REST API and native Rust client library for querying and managing financial time-series data.

## HTTP REST API

### Base URL
```
http://localhost:9000/api/v1
```

### Authentication
Currently uses API key authentication:
```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:9000/api/v1/query
```

### Endpoints

#### Health Check
```http
GET /health
```

**Response:**
```json
{
  "status": "ok",
  "uptime": 3600,
  "version": "1.0.0"
}
```

#### Query Data
```http
GET /query?symbol={symbol}&start={start}&end={end}
```

**Parameters:**
- `symbol` (required): Trading symbol (e.g., "EURUSD")
- `start` (required): Start timestamp (ISO 8601)
- `end` (required): End timestamp (ISO 8601)
- `format` (optional): Response format ("json", "csv", "parquet")
- `limit` (optional): Maximum records to return

**Example:**
```bash
curl "http://localhost:9000/api/v1/query?symbol=EURUSD&start=2023-01-01T00:00:00Z&end=2023-01-02T00:00:00Z"
```

**Response:**
```json
{
  "symbol": "EURUSD",
  "records": [
    {
      "timestamp": "2023-01-01T00:00:00Z",
      "open": 1.05432,
      "high": 1.05478,
      "low": 1.05401,
      "close": 1.05456,
      "volume": 12500
    }
  ],
  "count": 1440,
  "query_ms": 15
}
```

#### Import Data
```http
POST /import
Content-Type: multipart/form-data
```

**Parameters:**
- `file`: CSV file to import
- `symbol`: Trading symbol
- `compression_level`: Compression level (1-22)

#### Metrics
```http
GET /metrics
```

Returns Prometheus-format metrics for monitoring.

## Rust Client Library

### Installation
```toml
[dependencies]
fx-store = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Quick Start
```rust
use fx_store::{Client, TimeRange};
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect("localhost:9000").await?;
    
    let records = client
        .query("EURUSD")
        .range(Utc::now() - Duration::days(1), Utc::now())
        .execute()
        .await?;
    
    println!("Found {} records", records.len());
    Ok(())
}
```

### Client Methods

#### Connection
```rust
let client = Client::connect("localhost:9000").await?;
let client = Client::with_config(config).await?;
```

#### Query Builder
```rust
let query = client
    .query("EURUSD")
    .range(start, end)
    .limit(1000)
    .format(OutputFormat::Json);

let records = query.execute().await?;
```

#### Streaming
```rust
let mut stream = client.subscribe("EURUSD").await?;

while let Some(ohlcv) = stream.next().await {
    println!("Real-time: {:?}", ohlcv);
}
```

#### Batch Operations
```rust
let symbols = vec!["EURUSD", "GBPUSD", "USDJPY"];
let results = client.query_batch(symbols, range).await?;
```

## Data Types

### OHLCV Record
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct OHLCV {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u32,
}
```

### Query Options
```rust
pub struct QueryOptions {
    pub symbol: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub limit: Option<usize>,
    pub format: OutputFormat,
}
```

### Output Formats
```rust
pub enum OutputFormat {
    Json,
    Csv,
    Parquet,
    Binary,
}
```

## Error Handling

### HTTP Status Codes
- `200`: Success
- `400`: Bad Request (invalid parameters)
- `404`: Symbol not found
- `429`: Rate limit exceeded
- `500`: Internal server error

### Rust Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum FxStoreError {
    #[error("Connection failed: {0}")]
    Connection(String),
    
    #[error("Query failed: {0}")]
    Query(String),
    
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
    
    #[error("Invalid time range")]
    InvalidTimeRange,
}
```

## Rate Limits

- Query API: 100 requests/minute per API key
- Import API: 10 requests/minute per API key
- WebSocket: 1000 messages/minute per connection

## Examples

### Python Client
```python
import requests
from datetime import datetime, timedelta

# Query data
response = requests.get(
    "http://localhost:9000/api/v1/query",
    params={
        "symbol": "EURUSD",
        "start": (datetime.now() - timedelta(days=1)).isoformat(),
        "end": datetime.now().isoformat(),
        "format": "json"
    }
)

data = response.json()
print(f"Found {data['count']} records")
```

### WebSocket Streaming
```javascript
const ws = new WebSocket('ws://localhost:9000/ws');

ws.on('open', () => {
    ws.send(JSON.stringify({
        action: 'subscribe',
        symbol: 'EURUSD'
    }));
});

ws.on('message', (data) => {
    const ohlcv = JSON.parse(data);
    console.log('Real-time update:', ohlcv);
});
```

## Configuration

### Server Configuration
See [deployment.md](./deployment.md) for server configuration options.

### Client Configuration
```rust
let config = ClientConfig {
    endpoint: "http://localhost:9000".to_string(),
    api_key: Some("your-api-key".to_string()),
    timeout: Duration::from_secs(30),
    retry_attempts: 3,
};

let client = Client::with_config(config).await?;
```