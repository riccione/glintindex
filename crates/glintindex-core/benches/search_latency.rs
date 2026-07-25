//! Benchmark: search query latency.

mod common;

use std::path::PathBuf;
use std::time::SystemTime;

use criterion::{Criterion, criterion_group, criterion_main};

use glintindex_core::{Document, DocumentIndexer, IndexService, SearchEngine, SearchQuery};

use common::criterion_config;

/// Build a test index with `num_docs` documents. Not timed.
fn build_search_index(num_docs: usize) -> (IndexService, tempfile::TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let index_path = tmp.path().join("index");
    let service = IndexService::open_or_create(&index_path).unwrap();

    for i in 0..num_docs {
        let doc = Document::new(
            PathBuf::from(format!("/docs/file_{i:06}.txt")),
            1000,
            SystemTime::now(),
            format!(
                "Document {i} covers topics like rust programming, \
                 memory management, and systems design. \
                 The quick brown fox jumps over the lazy dog. \
                 Performance optimization requires careful measurement."
            ),
        );
        service.add_document(&doc).unwrap();
    }
    service.commit().unwrap();
    (service, tmp)
}

fn bench_search(_c: &mut Criterion) {
    common::write_metadata();
    // Build index ONCE — not inside any b.iter()
    let (service, _tmp) = build_search_index(100_000);

    let mut criterion = criterion_config();
    let mut group = criterion.benchmark_group("search");

    let queries: &[(&str, &str)] = &[
        ("single_word", "rust"),
        ("multi_word", "memory management"),
        ("prefix", "prog"),
        ("phrase", "\"systems design\""),
        ("common_term", "the"),
        ("rare_term", "xyzzy"),
        ("typo", "progamming"),
    ];

    for (name, query_str) in queries {
        group.bench_function(*name, |b| {
            let query = SearchQuery::new(*query_str);
            b.iter(|| service.search(&query).unwrap());
        });
    }

    group.finish();
}

criterion_group!(benches, bench_search);
criterion_main!(benches);
