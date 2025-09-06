mod api;
mod block;
mod mmap_format;
mod query;
mod store;
mod types;

use api::start_server;
use store::FxStore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 스토어 생성
    let store = Arc::new(FxStore::new());

    // 2. 데이터 임포트 (비동기 실행)
    let import_store = Arc::clone(&store);
    tokio::task::spawn_blocking(move || {
        // XAUUSD 데이터 임포트 (ignore missing files)
        if let Err(e) = import_store.import_csv("data/xauusd/DAT_ASCII_XAUUSD_M1_2023.csv", "XAUUSD") {
            eprintln!("Failed to import 2023 XAUUSD data: {}", e);
        }
        if let Err(e) = import_store.import_csv("data/xauusd/DAT_ASCII_XAUUSD_M1_2024.csv", "XAUUSD") {
            eprintln!("Failed to import 2024 XAUUSD data: {}", e);
        }
        
        // Optional: Import other symbols if available
        if let Err(e) = import_store.import_csv("data/BTCUSD_2024.csv", "BTCUSD") {
            eprintln!("BTCUSD data not found: {}", e);
        }
        if let Err(e) = import_store.import_csv("data/EURUSD_2024.csv", "EURUSD") {
            eprintln!("EURUSD data not found: {}", e);
        }
        
        println!("✅ Data import completed");
    });

    // 3. 쿼리 예제 - XAUUSD 데이터 (delayed to allow import)
    let query_store = Arc::clone(&store);
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        let start = chrono::Utc::now() - chrono::Duration::days(30);
        let end = chrono::Utc::now();

        let xauusd_records: Vec<_> = query_store
            .query_range(
                "XAUUSD",
                start.timestamp_nanos_opt().unwrap() as u64,
                end.timestamp_nanos_opt().unwrap() as u64,
            )
            .collect();

        if !xauusd_records.is_empty() {
            println!("📊 XAUUSD Found {} records", xauusd_records.len());
            
            // 최근 5개 XAUUSD 레코드 출력
            for (i, record) in xauusd_records.iter().rev().take(5).enumerate() {
                println!(
                    "XAUUSD #{}: O:{:.5} H:{:.5} L:{:.5} C:{:.5}",
                    i + 1,
                    record.open as f64 / 100000.0,
                    record.high as f64 / 100000.0,
                    record.low as f64 / 100000.0,
                    record.close as f64 / 100000.0
                );
            }
        } else {
            println!("⚠️  No XAUUSD data found in the last 30 days");
        }
    });

    // 4. HTTP API 서버 시작
    println!("🔧 Starting FX-Store with HTTP API...");
    start_server(store, 8080).await?;

    Ok(())
}
