//! Benchmark: per-parser throughput.

mod common;

use std::path::PathBuf;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};

use glintindex_core::ParserRegistry;

use common::{bench_data_dir, criterion_config};

fn bench_parsers(_c: &mut Criterion) {
    common::write_metadata();
    let mut criterion = criterion_config();
    let mut group = criterion.benchmark_group("parsers");
    let registry = ParserRegistry::new();

    // ── Text parser at various sizes ───────────────────────────
    for (size_name, repeat_count) in &[("1kb", 23usize), ("100kb", 2_300), ("1mb", 23_000)] {
        let content = "The quick brown fox jumps over the lazy dog. ".repeat(*repeat_count);
        let bytes = content.as_bytes();
        let path = PathBuf::from(format!("test_{size_name}.txt"));
        let parser = registry.parser_for(&path);

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_function(format!("text_{size_name}"), |b| {
            b.iter(|| {
                let result = parser.parse(bytes, &path);
                std::hint::black_box(result)
            });
        });
    }

    // ── Office format parsers (real files from bench_data/) ────
    let bench_data = bench_data_dir();
    let mut executed = 0u32;
    let mut skipped = 0u32;

    for (ext, name) in &[
        ("docx", "docx"),
        ("xlsx", "xlsx"),
        ("pptx", "pptx"),
        ("pdf", "pdf"),
        ("odt", "odt"),
        ("rtf", "rtf"),
    ] {
        let file_path = bench_data.join(format!("sample.{ext}"));
        if !file_path.exists() {
            eprintln!(
                "Skipping parser benchmark '{name}':\n    sample.{ext} not found in bench_data/"
            );
            skipped += 1;
            continue;
        }
        let bytes = match std::fs::read(&file_path) {
            Ok(b) => b,
            Err(err) => {
                eprintln!(
                    "Skipping parser benchmark '{name}':\n    sample.{ext} failed to read: {err}"
                );
                skipped += 1;
                continue;
            }
        };
        let parser = registry.parser_for(&file_path);
        if let Err(err) = parser.parse(&bytes, &file_path) {
            eprintln!(
                "Skipping parser benchmark '{name}':\n    sample.{ext} failed to parse: {err}"
            );
            skipped += 1;
            continue;
        }
        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_function(*name, |b| {
            b.iter(|| {
                let result = parser.parse(&bytes, &file_path);
                std::hint::black_box(result)
            });
        });
        executed += 1;
    }

    eprintln!("Parser benchmarks: executed: {executed}, skipped: {skipped}");

    group.finish();
}

criterion_group!(benches, bench_parsers);
criterion_main!(benches);
