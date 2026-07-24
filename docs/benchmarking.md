# Benchmarking GlintIndex

## Prerequisites

- Rust toolchain (stable, 1.85+)
- ~500 MB free disk space for datasets

## Running Benchmarks

```bash
# All benchmarks (~10 minutes depending on dataset sizes)
cargo bench -p glintindex-core

# Individual benchmark groups
cargo bench -p glintindex-core --bench indexing_throughput
cargo bench -p glintindex-core --bench parser_throughput
cargo bench -p glintindex-core --bench search_latency
```

### Filter specific benchmarks

```bash
# Run only cold indexing benchmarks
cargo bench -p glintindex-core --bench indexing_throughput -- indexing_cold

# Run only text parser benchmarks
cargo bench -p glintindex-core --bench parser_throughput -- text_

# Run only single-word search
cargo bench -p glintindex-core --bench search_latency -- single_word
```

## Benchmark Groups

| Group | What it measures |
|-------|-----------------|
| `indexing_cold` | Full indexing from scratch (no prior index) |
| `indexing_incremental` | Re-scanning when index already exists (skips unchanged files) |
| `parsers` | Per-parser throughput (text, PDF, DOCX, XLSX, PPTX, RTF, ODT) |
| `search` | Query latency for various query types |

## Synthetic Datasets

| Dataset | Files | Avg size | Total |
|---------|-------|----------|-------|
| `1k_text_1kb` | 1,000 | 1 KB | ~1 MB |
| `10k_mixed` | 10,000 | 512 B | ~5 MB |
| `100k_tiny_50b` | 100,000 | 50 B | ~5 MB |
| `100k_small_1kb` | 100,000 | 1 KB | ~100 MB |

All synthetic datasets use seeded RNG (`ChaCha8Rng` with fixed seed) for reproducibility.

## Real-World Datasets

Set environment variables to include real source trees:

```bash
export BENCHMARK_KERNEL_PATH=/path/to/linux/kernel
export BENCHMARK_RUST_STD_PATH=/path/to/rust/library
cargo bench -p glintindex-core
```

If unset, real-world benchmarks are skipped gracefully.

## Sample Office Files

The `bench_data/` directory contains real Office documents for parser benchmarks:

```
bench_data/
    sample.docx
    sample.xlsx
    sample.pptx
    sample.pdf
    sample.odt
    sample.rtf
```

These are minimal but valid files. Replace them with real documents for more representative parser benchmarks.

## Reports

HTML reports are generated in `target/criterion/`.

```
target/criterion/
    report/index.html            # Summary of all benchmarks
    indexing_cold/
        1k_text_1kb/report/index.html
    parsers/
        text_1kb/report/index.html
    search/
        single_word/report/index.html
```

Open `target/criterion/report/index.html` in a browser.

## Reproducibility

- Synthetic datasets use seeded RNG — deterministic across runs
- Criterion tracks historical comparisons automatically
- Each benchmark runs with: warm-up=3s, measurement=15s, samples=50

## Cache Effects

- **Cold cache**: first run after reboot or `sync && sudo sh -c 'echo 3 > /proc/sys/vm/drop_caches'`
- **Warm cache**: subsequent runs. Much faster for filesystem-heavy benchmarks.

Document which mode was used when sharing results.

## Metrics

| Metric | Source |
|--------|--------|
| Elapsed time | Criterion (wall clock) |
| Throughput (MB/s) | Criterion `Throughput::Bytes` |
| Peak memory | `/proc/self/status` VmHWM (Linux) |
| Files/sec | Derived from elapsed + file count |

## What Is NOT Measured (yet)

- CPU utilization — use `perf`, `pidstat`, or `dtrace` separately
- Disk I/O — use `iostat` or `blktrace`
- Network — not applicable
