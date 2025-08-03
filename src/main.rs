mod block;
mod mmap_format;
mod query;
mod store;
mod types;

use query::TechnicalIndicators;
use store::FxStore;

fn main() -> anyhow::Result<()> {
    // 1. 스토어 생성
    let store = FxStore::new();

    // 2. HISTDATA CSV 임포트 (skip missing files)
    // store.import_csv("data/EURUSD_2023.csv", "EURUSD")?;
    // store.import_csv("data/GBPUSD_2023.csv", "GBPUSD")?;

    // 3. XAUUSD 데이터 임포트 (2023-2024)
    store.import_csv("data/xauusd/DAT_ASCII_XAUUSD_M1_2023.csv", "XAUUSD")?;
    store.import_csv("data/xauusd/DAT_ASCII_XAUUSD_M1_2024.csv", "XAUUSD")?;

    // 4. 쿼리 예제 - XAUUSD 데이터
    let start = chrono::Utc::now() - chrono::Duration::days(30);
    let end = chrono::Utc::now();

    let xauusd_records: Vec<_> = store
        .query_range(
            "XAUUSD",
            start.timestamp_nanos() as u64,
            end.timestamp_nanos() as u64,
        )
        .collect();

    // 5. 기술적 지표 계산 - XAUUSD
    if !xauusd_records.is_empty() {
        let sma_20 = TechnicalIndicators::sma(&xauusd_records, 20);
        let rsi_14 = TechnicalIndicators::rsi(&xauusd_records, 14);

        println!("XAUUSD Found {} records", xauusd_records.len());
        if !sma_20.is_empty() {
            println!("XAUUSD SMA(20): {:?}", &sma_20[..sma_20.len().min(5)]);
        }

        // 최근 5개 XAUUSD 레코드 출력
        for (i, record) in xauusd_records.iter().rev().take(5).enumerate() {
            println!(
                "XAUUSD #{}: O:{:.3} H:{:.3} L:{:.3} C:{:.3}",
                i + 1,
                record.open as f64 / 100000.0,
                record.high as f64 / 100000.0,
                record.low as f64 / 100000.0,
                record.close as f64 / 100000.0
            );
        }
    }

    // 6. 실시간 스트리밍
    let rx = store.stream_realtime("XAUUSD");
    std::thread::spawn(move || {
        while let Ok(ohlcv) = rx.recv() {
            println!(
                "Real-time XAUUSD: O:{:.3} H:{:.3} L:{:.3} C:{:.3}",
                ohlcv.open as f64 / 100000.0,
                ohlcv.high as f64 / 100000.0,
                ohlcv.low as f64 / 100000.0,
                ohlcv.close as f64 / 100000.0
            );
        }
    });

    Ok(())
}
