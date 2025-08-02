mod types;
mod block;
mod store;
mod query;
mod mmap_format;

use store::FxStore;
use query::TechnicalIndicators;

fn main() -> anyhow::Result<()> {
    // 1. 스토어 생성
    let store = FxStore::new();
    
    // 2. HISTDATA CSV 임포트
    store.import_csv("data/EURUSD_2023.csv", "EURUSD")?;
    store.import_csv("data/GBPUSD_2023.csv", "GBPUSD")?;
    
    // 3. 쿼리 예제
    let start = chrono::Utc::now() - chrono::Duration::days(7);
    let end = chrono::Utc::now();
    
    let records: Vec<_> = store.query_range(
        "EURUSD",
        start.timestamp_nanos() as u64,
        end.timestamp_nanos() as u64
    ).collect();
    
    // 4. 기술적 지표 계산
    let sma_20 = TechnicalIndicators::sma(&records, 20);
    let rsi_14 = TechnicalIndicators::rsi(&records, 14);
    
    println!("Found {} records", records.len());
    println!("SMA(20): {:?}", &sma_20[..5]);
    
    // 5. 실시간 스트리밍
    let rx = store.stream_realtime("EURUSD");
    std::thread::spawn(move || {
        while let Ok(ohlcv) = rx.recv() {
            println!("Real-time: {:?}", ohlcv);
        }
    });
    
    Ok(())
}