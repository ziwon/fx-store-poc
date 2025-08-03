use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;

/// 영속성을 위한 mmap 파일 구조
#[repr(C, packed)]
struct MmapHeader {
    magic: [u8; 8], // "FXSTORE1"
    version: u32,
    symbol_count: u32,
    block_count: u64,
    index_offset: u64,
    data_offset: u64,
}

pub struct PersistentStore {
    mmap: MmapMut,
    header: *mut MmapHeader,
}

impl PersistentStore {
    pub unsafe fn create(path: &str, size: usize) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        file.set_len(size as u64)?;

        let mut mmap = unsafe { MmapOptions::new().len(size).map_mut(&file)? };

        // 헤더 초기화
        let header = unsafe { &mut *(mmap.as_mut_ptr() as *mut MmapHeader) };
        header.magic = *b"FXSTORE1";
        header.version = 1;

        Ok(Self {
            header: mmap.as_mut_ptr() as *mut MmapHeader,
            mmap,
        })
    }
}
