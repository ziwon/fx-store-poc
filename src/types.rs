use serde::{Deserialize, Serialize};

/// 40-byte 고정폭 OHLCV (캐시라인 최적화)
#[repr(C, packed)]
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct OHLCV {
    pub ts: u64,   // epoch nanos
    pub open: u32, // 가격 * 100000 (5자리 정밀도)
    pub high: u32,
    pub low: u32,
    pub close: u32,
    pub volume: u32,
    pub symbol_id: u16,
    pub _pad: [u8; 10], // Padding to make it 40 bytes total
}

const _: () = assert!(std::mem::size_of::<OHLCV>() == 40);

impl OHLCV {
    #[inline]
    pub fn from_fx(dt: &str, o: f64, h: f64, l: f64, c: f64, v: u32, sym: u16) -> Self {
        use chrono::NaiveDateTime;

        let ts = NaiveDateTime::parse_from_str(dt, "%Y%m%d %H%M%S")
            .unwrap()
            .and_utc()
            .timestamp_nanos_opt()
            .unwrap() as u64;

        Self {
            ts,
            open: (o * 100000.0) as u32,
            high: (h * 100000.0) as u32,
            low: (l * 100000.0) as u32,
            close: (c * 100000.0) as u32,
            volume: v,
            symbol_id: sym,
            _pad: [0; 10],
        }
    }

    #[inline]
    pub fn price_f64(&self, field: PriceField) -> f64 {
        let val = match field {
            PriceField::Open => self.open,
            PriceField::High => self.high,
            PriceField::Low => self.low,
            PriceField::Close => self.close,
        };
        val as f64 / 100000.0
    }
}

#[derive(Copy, Clone)]
pub enum PriceField {
    Open,
    High,
    Low,
    Close,
}

/// 심볼 메타데이터
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Symbol {
    pub id: u16,
    pub name: String,
    pub base: String,
    pub quote: String,
}
