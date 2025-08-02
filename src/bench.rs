#[cfg(test)]
mod benches {
    use super::*;
    use test::Bencher;
    
    #[bench]
    fn bench_import_1m_records(b: &mut Bencher) {
        let store = FxStore::new();
        let data = generate_test_data(1_000_000);
        
        b.iter(|| {
            for record in &data {
                store.insert(*record);
            }
        });
    }
    
    #[bench]
    fn bench_query_range(b: &mut Bencher) {
        let store = setup_test_store();
        
        b.iter(|| {
            let start = chrono::Utc::now() - chrono::Duration::days(30);
            let end = chrono::Utc::now();
            
            let count = store.query_range(
                "EURUSD",
                start.timestamp_nanos() as u64,
                end.timestamp_nanos() as u64
            ).count();
            
            assert!(count > 0);
        });
    }
}