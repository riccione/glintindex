//! Benchmark: indexing throughput (cold and incremental).

mod common;

use std::path::Path;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

use glintindex_core::{FilesystemScanner, IndexService};

use common::{create_mixed_dataset, create_text_dataset, criterion_config, real_world_datasets};

/// Index a directory into a fresh index. Returns elapsed time.
fn index_cold(data_dir: &Path, index_dir: &Path) {
    let service = IndexService::open_or_create(index_dir).unwrap();
    let scanner = FilesystemScanner::new(&service);
    let _stats = scanner.scan_directory(data_dir).unwrap();
    service.commit().unwrap();
}

/// Re-scan a directory where the index already exists (incremental).
fn index_incremental(data_dir: &Path, index_dir: &Path) {
    let service = IndexService::open_or_create(index_dir).unwrap();
    let scanner = FilesystemScanner::new(&service);
    let _stats = scanner.scan_directory(data_dir).unwrap();
    service.commit().unwrap();
}

fn bench_indexing_cold(_c: &mut Criterion) {
    let mut criterion = criterion_config();
    let mut group = criterion.benchmark_group("indexing_cold");

    // ── Synthetic text datasets ────────────────────────────────
    for (name, count, size) in &[
        ("1k_text_1kb", 1_000usize, 1024usize),
        ("10k_mixed", 10_000, 512),
        ("100k_tiny_50b", 100_000, 50),
        ("100k_small_1kb", 100_000, 1024),
    ] {
        // Dataset created OUTSIDE the timed loop
        let data_dir = tempfile::tempdir().unwrap();
        if *count == 10_000 {
            create_mixed_dataset(data_dir.path(), *count);
        } else {
            create_text_dataset(data_dir.path(), *count, *size);
        }

        group.bench_function(*name, |b| {
            b.iter_batched(
                || {
                    let idx = tempfile::tempdir().unwrap();
                    (data_dir.path().to_path_buf(), idx)
                },
                |(data_path, idx_dir)| {
                    index_cold(&data_path, &idx_dir.path().join("idx"));
                },
                BatchSize::SmallInput,
            );
        });
    }

    // ── Real-world source trees ────────────────────────────────
    for (name, path) in real_world_datasets() {
        group.bench_function(name, |b| {
            b.iter_batched(
                || {
                    let idx = tempfile::tempdir().unwrap();
                    (path.clone(), idx)
                },
                |(data_path, idx_dir)| {
                    index_cold(&data_path, &idx_dir.path().join("idx"));
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_indexing_incremental(_c: &mut Criterion) {
    let mut criterion = criterion_config();
    let mut group = criterion.benchmark_group("indexing_incremental");

    for (name, count, size) in &[
        ("1k_text_1kb", 1_000usize, 1024usize),
        ("100k_small_1kb", 100_000, 1024),
    ] {
        // Pre-build index BEFORE timing
        let data_dir = tempfile::tempdir().unwrap();
        create_text_dataset(data_dir.path(), *count, *size);
        let idx_dir = tempfile::tempdir().unwrap();
        index_cold(data_dir.path(), &idx_dir.path().join("idx"));

        group.bench_function(*name, |b| {
            b.iter_batched(
                || {
                    // Fresh IndexService instance (simulates real usage)
                    let idx = idx_dir.path().join("idx");
                    (data_dir.path().to_path_buf(), idx)
                },
                |(data_path, idx_path)| {
                    index_incremental(&data_path, &idx_path);
                },
                BatchSize::SmallInput,
            );
        });
    }

    // ── Real-world incremental ─────────────────────────────────
    for (name, path) in real_world_datasets() {
        // Pre-build index
        let idx_dir = tempfile::tempdir().unwrap();
        index_cold(&path, &idx_dir.path().join("idx"));

        group.bench_function(name, |b| {
            b.iter_batched(
                || {
                    let idx = idx_dir.path().join("idx");
                    (path.clone(), idx)
                },
                |(data_path, idx_path)| {
                    index_incremental(&data_path, &idx_path);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_indexing_cold, bench_indexing_incremental);
criterion_main!(benches);
