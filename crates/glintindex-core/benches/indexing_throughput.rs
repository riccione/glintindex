//! Benchmark: indexing throughput (cold and incremental).

mod common;

use std::fs;
use std::path::Path;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};

use glintindex_core::{FilesystemScanner, IndexService};

use common::{
    bench_data_dir, create_docx_dataset, create_mixed_dataset, create_pdf_dataset,
    create_text_dataset, criterion_config, real_world_datasets, slow_criterion_config,
};

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

/// Sum of all file sizes in a directory tree.
fn dir_total_bytes(path: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }
    }
    total
}

fn bench_indexing_cold(_c: &mut Criterion) {
    common::write_metadata();
    // ── Slow benchmarks (>500ms per iteration) ─────────────────
    let mut slow_criterion = slow_criterion_config();
    let mut slow_group = slow_criterion.benchmark_group("indexing/cold");

    for (name, count, size) in &[
        ("10k_512b", 10_000usize, 512usize),
        ("100k_50b", 100_000, 50),
        ("100k_1kb", 100_000, 1024),
    ] {
        let data_dir = tempfile::tempdir().unwrap();
        if *count == 10_000 {
            create_mixed_dataset(data_dir.path(), *count);
        } else {
            create_text_dataset(data_dir.path(), *count, *size);
        }

        let total_bytes = (*count * *size) as u64;
        slow_group.throughput(Throughput::Bytes(total_bytes));
        slow_group.bench_function(*name, |b| {
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

    // ── PDF dataset ─────────────────────────────────────────────
    let data_dir = tempfile::tempdir().unwrap();
    create_pdf_dataset(data_dir.path(), 10_000);
    let pdf_bytes = fs::metadata(bench_data_dir().join("sample.pdf"))
        .map(|m| m.len() * 10_000)
        .unwrap_or(8_190_000);
    slow_group.throughput(Throughput::Bytes(pdf_bytes));
    slow_group.bench_function("10k_pdf", |b| {
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

    // ── DOCX dataset ────────────────────────────────────────────
    let data_dir = tempfile::tempdir().unwrap();
    create_docx_dataset(data_dir.path(), 10_000);
    let docx_bytes = fs::metadata(bench_data_dir().join("sample.docx"))
        .map(|m| m.len() * 10_000)
        .unwrap_or(11_180_000);
    slow_group.throughput(Throughput::Bytes(docx_bytes));
    slow_group.bench_function("10k_docx", |b| {
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

    for (name, path) in real_world_datasets() {
        let total = dir_total_bytes(&path);
        slow_group.throughput(Throughput::Bytes(total));
        slow_group.bench_function(name, |b| {
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

    slow_group.finish();

    // ── Fast benchmarks (<500ms per iteration) ─────────────────
    let mut criterion = criterion_config();
    let mut group = criterion.benchmark_group("indexing/cold");

    let data_dir = tempfile::tempdir().unwrap();
    create_text_dataset(data_dir.path(), 1_000, 1024);

    group.throughput(Throughput::Bytes(1_000 * 1024));
    group.bench_function("1k_1kb", |b| {
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

    group.finish();
}

fn bench_indexing_incremental(_c: &mut Criterion) {
    common::write_metadata();
    let mut criterion = criterion_config();
    let mut group = criterion.benchmark_group("indexing/incremental");

    for (name, count, size) in &[
        ("1k_1kb", 1_000usize, 1024usize),
        ("100k_1kb", 100_000, 1024),
    ] {
        // Pre-build index BEFORE timing
        let data_dir = tempfile::tempdir().unwrap();
        create_text_dataset(data_dir.path(), *count, *size);
        let idx_dir = tempfile::tempdir().unwrap();
        index_cold(data_dir.path(), &idx_dir.path().join("idx"));

        let total_bytes = (*count * *size) as u64;
        group.throughput(Throughput::Bytes(total_bytes));
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

        let total = dir_total_bytes(&path);
        group.throughput(Throughput::Bytes(total));
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
