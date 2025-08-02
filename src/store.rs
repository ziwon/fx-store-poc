use dashmap::DashMap;
use memmap2::{Mmap, MmapOptions};
use crossbeam::channel::{bounded, Sender, Receiver};
use std::sync::atomic::{AtomicU64, Ordering};
use ahash::RandomState;

pub struct FxStore {
    /// symbol_id -> date -> block
    blocks: DashMap<u16, DashMap<u32, CompressedBlock, RandomState>, RandomState>,
    
    /// 심볼 테이블
    symbols: DashMap<String, Symbol>,
    
    /// 통계
    stats: StoreStats,
    
    /// 백그라운드 압축 채널
    compress_tx: Sender<(u32, u16, Vec<OHLCV>)>,
    compress_handle: Option<std::thread::JoinHandle<()>>,
}

#[derive(Default)]
struct StoreStats {
    total_records: AtomicU64,
    compressed_bytes: AtomicU64,
    cache_hits: AtomicU64,
}

impl FxStore {
    pub fn new() -> Self {
        let (tx, rx) = bounded(1000);
        
        // 백그라운드 압축 스레드
        let handle = std::thread::spawn(move || {
            compress_worker(rx);
        });
        
        Self {
            blocks: DashMap::with_hasher(RandomState::new()),
            symbols: DashMap::new(),
            stats: StoreStats::default(),
            compress_tx: tx,
            compress_handle: Some(handle),
        }
    }
    
    /// CSV 임포트 (rayon 병렬)
    pub fn import_csv(&self, path: &str, symbol: &str) -> anyhow::Result<()> {
        use rayon::prelude::*;
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        
        let sym_id = self.get_or_create_symbol(symbol);
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        // 라인을 일별로 그룹화
        let mut daily_groups: DashMap<u32, Vec<String>> = DashMap::new();
        
        for line in reader.lines().skip(1) {
            let line = line?;
            let date = line[0..8].parse::<u32>()?;
            daily_groups.entry(date).or_default().push(line);
        }
        
        // 병렬 파싱 및 압축
        daily_groups.into_par_iter().for_each(|(date, lines)| {
            let records: Vec<OHLCV> = lines.par_iter()
                .filter_map(|line| parse_line(line, sym_id).ok())
                .collect();
                
            self.compress_tx.send((date, sym_id, records)).ok();
        });
        
        Ok(())
    }
    
    /// 시간 범위 쿼리 (zero-copy 이터레이터)
    pub fn query_range(&self, symbol: &str, start_ts: u64, end_ts: u64) 
        -> impl Iterator<Item = OHLCV> + '_ 
    {
        let sym_id = match self.symbols.get(symbol) {
            Some(s) => s.id,
            None => return Box::new(std::iter::empty()) as Box<dyn Iterator<Item = OHLCV>>,
        };
        
        let start_date = ts_to_date(start_ts);
        let end_date = ts_to_date(end_ts);
        
        let symbol_blocks = match self.blocks.get(&sym_id) {
            Some(b) => b,
            None => return Box::new(std::iter::empty()) as Box<dyn Iterator<Item = OHLCV>>,
        };
        
        // 날짜 범위의 블록들을 순회
        let blocks: Vec<_> = symbol_blocks.iter()
            .filter(|entry| *entry.key() >= start_date && *entry.key() <= end_date)
            .map(|entry| entry.value().clone())
            .collect();
            
        Box::new(blocks.into_iter().flat_map(move |block| {
            let data = block.decompress();
            data.into_vec().into_iter()
                .filter(move |rec| rec.ts >= start_ts && rec.ts <= end_ts)
        }))
    }
    
    /// 리얼타임 스트리밍 (tick-to-1min 집계)
    pub fn stream_realtime(&self, symbol: &str) -> Receiver<OHLCV> {
        let (tx, rx) = bounded(10000);
        let sym_id = self.get_or_create_symbol(symbol);
        
        // 실시간 집계 스레드
        std::thread::spawn(move || {
            aggregate_ticks_to_minutes(sym_id, tx);
        });
        
        rx
    }
}

/// 백그라운드 압축 워커
fn compress_worker(rx: Receiver<(u32, u16, Vec<OHLCV>)>) {
    while let Ok((date, symbol_id, records)) = rx.recv() {
        let block = CompressedBlock::new(date, symbol_id, &records);
        // 저장 로직...
    }
}

/// 타임스탬프 → YYYYMMDD 변환
#[inline]
fn ts_to_date(ts: u64) -> u32 {
    use chrono::{DateTime, Utc};
    let dt = DateTime::from_timestamp_nanos(ts as i64);
    dt.format("%Y%m%d").to_string().parse().unwrap()
}