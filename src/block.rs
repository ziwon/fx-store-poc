use crate::types::OHLCV;
use parking_lot::RwLock;
use std::sync::Arc;
use zstd::bulk::{compress, decompress};

const BLOCK_SIZE: usize = 1440; // 1일 = 1440분

/// 압축된 일일 블록
#[derive(Clone)]
pub struct CompressedBlock {
    pub date: u32, // YYYYMMDD
    pub symbol_id: u16,
    pub data: Arc<Vec<u8>>,
    cached: Arc<RwLock<Option<Box<[OHLCV; BLOCK_SIZE]>>>>,
}

impl CompressedBlock {
    pub fn new(date: u32, symbol_id: u16, records: &[OHLCV]) -> Self {
        let mut block = Box::new([OHLCV::default(); BLOCK_SIZE]);

        // 1분 간격으로 정렬
        for rec in records {
            let minute_of_day = ((rec.ts / 1_000_000_000) % 86400) / 60;
            block[minute_of_day as usize] = *rec;
        }

        // 압축 (레벨 3이 속도/압축률 균형 최적)
        let serialized = bincode::serialize(&block.to_vec()).unwrap();
        let compressed = compress(&serialized, 3).unwrap();

        Self {
            date,
            symbol_id,
            data: Arc::new(compressed),
            cached: Arc::new(RwLock::new(None)),
        }
    }

    pub fn decompress(&self) -> Box<[OHLCV; BLOCK_SIZE]> {
        // 캐시 확인
        if let Some(cached) = self.cached.read().as_ref() {
            return cached.clone();
        }

        // 압축 해제
        let decompressed = decompress(&self.data, BLOCK_SIZE * 40).unwrap();
        let records: Vec<OHLCV> = bincode::deserialize(&decompressed).unwrap();
        let mut block = Box::new([OHLCV::default(); BLOCK_SIZE]);
        for (i, record) in records.into_iter().enumerate() {
            if i < BLOCK_SIZE {
                block[i] = record;
            }
        }

        // 캐시 저장
        *self.cached.write() = Some(block.clone());
        block
    }
}
