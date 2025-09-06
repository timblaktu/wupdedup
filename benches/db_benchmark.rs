use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tempfile::tempdir;
use wupdedup_rs::db::DB;

fn bench_db_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("database_operations");
    
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let db = DB::init(db_path.to_str().unwrap()).unwrap();
    let bucket = db.bucket("bench").unwrap();
    
    // Benchmark single write
    group.bench_function("single_write", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = format!("value_{}", counter);
            bucket.put(black_box(&key), black_box(value.as_bytes())).unwrap();
            counter += 1;
        });
    });
    
    // Setup data for read benchmarks
    for i in 0..1000 {
        let key = format!("read_key_{}", i);
        let value = format!("read_value_{}", i);
        bucket.put(&key, value.as_bytes()).unwrap();
    }
    
    // Benchmark single read
    group.bench_function("single_read", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("read_key_{}", counter % 1000);
            let _ = bucket.get(black_box(&key)).unwrap();
            counter += 1;
        });
    });
    
    // Benchmark batch writes
    for batch_size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_write", batch_size),
            batch_size,
            |b, &size| {
                let mut base = 0;
                b.iter(|| {
                    for i in 0..size {
                        let key = format!("batch_key_{}_{}", base, i);
                        let value = format!("batch_value_{}_{}", base, i);
                        bucket.put(&key, value.as_bytes()).unwrap();
                    }
                    base += size;
                });
            },
        );
    }
    
    group.finish();
}

fn bench_db_scan(c: &mut Criterion) {
    use wupdedup_rs::db::Scanner;
    
    let mut group = c.benchmark_group("database_scanning");
    
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("bench_scan.db");
    let db = DB::init(db_path.to_str().unwrap()).unwrap();
    let bucket = db.bucket("bench_scan").unwrap();
    
    // Setup test data
    for count in [100, 1000, 10000].iter() {
        // Clear and repopulate
        for i in 0..*count {
            let key = format!("scan_key_{:06}", i);
            let value = vec![0u8; 100]; // 100 byte values
            bucket.put(&key, &value).unwrap();
        }
        
        group.bench_with_input(
            BenchmarkId::new("full_scan", count),
            count,
            |b, _| {
                b.iter(|| {
                    let mut count = 0;
                    bucket.scan(|_key, _value| {
                        count += 1;
                        Ok(())
                    }).unwrap();
                    black_box(count);
                });
            },
        );
    }
    
    group.finish();
}

fn bench_db_concurrent(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;
    
    let mut group = c.benchmark_group("concurrent_operations");
    
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("bench_concurrent.db");
    let db = Arc::new(DB::init(db_path.to_str().unwrap()).unwrap());
    
    for thread_count in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_writes", thread_count),
            thread_count,
            |b, &threads| {
                b.iter(|| {
                    let mut handles = vec![];
                    
                    for thread_id in 0..threads {
                        let db_clone = Arc::clone(&db);
                        let handle = thread::spawn(move || {
                            let bucket = db_clone.bucket(&format!("thread_{}", thread_id)).unwrap();
                            for i in 0..100 {
                                let key = format!("key_{}", i);
                                let value = format!("value_{}_{}", thread_id, i);
                                bucket.put(&key, value.as_bytes()).unwrap();
                            }
                        });
                        handles.push(handle);
                    }
                    
                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, bench_db_operations, bench_db_scan, bench_db_concurrent);
criterion_main!(benches);