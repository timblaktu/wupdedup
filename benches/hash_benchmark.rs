use blake3::Hasher;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::time::Duration;

fn bench_blake3_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("blake3_hashing");
    group.measurement_time(Duration::from_secs(10));

    // Test different file sizes
    for size in [
        1024,             // 1 KB
        10 * 1024,        // 10 KB
        100 * 1024,       // 100 KB
        1024 * 1024,      // 1 MB
        10 * 1024 * 1024, // 10 MB
    ]
    .iter()
    {
        let data = vec![0u8; *size];

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{} bytes", size)),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut hasher = Hasher::new();
                    hasher.update(black_box(data));
                    hasher.finalize()
                });
            },
        );
    }

    group.finish();
}

fn bench_incremental_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_hashing");

    let data = vec![0u8; 10 * 1024 * 1024]; // 10 MB

    for chunk_size in [
        1024,        // 1 KB chunks
        16 * 1024,   // 16 KB chunks
        64 * 1024,   // 64 KB chunks (default in our implementation)
        256 * 1024,  // 256 KB chunks
        1024 * 1024, // 1 MB chunks
    ]
    .iter()
    {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{} byte chunks", chunk_size)),
            chunk_size,
            |b, &chunk_size| {
                b.iter(|| {
                    let mut hasher = Hasher::new();
                    for chunk in data.chunks(chunk_size) {
                        hasher.update(black_box(chunk));
                    }
                    hasher.finalize()
                });
            },
        );
    }

    group.finish();
}

fn bench_parallel_hashing(c: &mut Criterion) {
    use rayon::prelude::*;

    let mut group = c.benchmark_group("parallel_hashing");

    // Create 100 files of 1MB each
    let files: Vec<Vec<u8>> = (0..100).map(|i| vec![i as u8; 1024 * 1024]).collect();

    group.bench_function("sequential", |b| {
        b.iter(|| {
            files
                .iter()
                .map(|data| {
                    let mut hasher = Hasher::new();
                    hasher.update(black_box(data));
                    hasher.finalize()
                })
                .collect::<Vec<_>>()
        });
    });

    group.bench_function("parallel", |b| {
        b.iter(|| {
            files
                .par_iter()
                .map(|data| {
                    let mut hasher = Hasher::new();
                    hasher.update(black_box(data));
                    hasher.finalize()
                })
                .collect::<Vec<_>>()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_blake3_hashing,
    bench_incremental_hashing,
    bench_parallel_hashing
);
criterion_main!(benches);
