//! Shared utilities for benchmark datasets and measurements.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use criterion::Criterion;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Fixed seed for reproducible datasets.
const SEED: u64 = 0xDEAD_BEEF;

/// Lorem ipsum fragment for generating realistic text content.
const LOREM: &str = "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua Ut enim ad minim veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur Excepteur sint occaecat cupidatat non proident sunt in culpa qui officia deserunt mollit anim id est laborum ";

/// Creates `count` text files of approximately `avg_size` bytes in `dir`.
///
/// Uses seeded RNG — deterministic across runs.
pub fn create_text_dataset(dir: &Path, count: usize, avg_size: usize) {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);
    fs::create_dir_all(dir).expect("failed to create dataset dir");

    for i in 0..count {
        let path = dir.join(format!("file_{i:06}.txt"));
        let target_bytes = avg_size;
        let mut content = String::with_capacity(target_bytes + 100);
        while content.len() < target_bytes {
            let chunk_len = rng.gen_range(20..80).min(target_bytes - content.len());
            content.push_str(&LOREM[..chunk_len]);
            content.push('\n');
        }
        fs::write(&path, content).expect("failed to write dataset file");
    }
}

/// Creates `count` files: 50% .txt, 20% .md, 20% .rs, 10% .json.
pub fn create_mixed_dataset(dir: &Path, count: usize) {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);
    fs::create_dir_all(dir).expect("failed to create dataset dir");

    for i in 0..count {
        let (ext, template): (&str, &str) = match i % 10 {
            0..5 => ("txt", "# Title\n\n{}\n"),
            5..7 => ("md", "# Heading\n\nParagraph: {}\n"),
            7..9 => (
                "rs",
                "fn main() {{\n    // {}\n    println!(\"hello\");\n}}\n",
            ),
            _ => ("json", "{{\"key\": \"{}\", \"value\": 42}}\n"),
        };
        let path = dir.join(format!("file_{i:06}.{ext}"));
        let text: String = (0..5)
            .map(|_| {
                let start = rng.gen_range(0..LOREM.len().saturating_sub(40));
                LOREM[start..start + 40].to_string()
            })
            .collect::<Vec<_>>()
            .join(" ");
        let content = template.replace("{}", &text);
        fs::write(&path, content).expect("failed to write dataset file");
    }
}

/// Creates `count` copies of sample.pdf with unique names.
pub fn create_pdf_dataset(dir: &Path, count: usize) {
    let src = bench_data_dir().join("sample.pdf");
    let bytes = fs::read(&src).expect("sample.pdf not found in bench_data/");
    fs::create_dir_all(dir).expect("failed to create dataset dir");
    for i in 0..count {
        let path = dir.join(format!("doc_{i:06}.pdf"));
        fs::write(&path, &bytes).expect("failed to write PDF");
    }
}

/// Creates `count` copies of sample.docx with unique names.
pub fn create_docx_dataset(dir: &Path, count: usize) {
    let src = bench_data_dir().join("sample.docx");
    let bytes = fs::read(&src).expect("sample.docx not found in bench_data/");
    fs::create_dir_all(dir).expect("failed to create dataset dir");
    for i in 0..count {
        let path = dir.join(format!("doc_{i:06}.docx"));
        fs::write(&path, &bytes).expect("failed to write DOCX");
    }
}

/// Returns paths to real-world source trees if env vars are set.
///
/// Checks `BENCHMARK_KERNEL_PATH` and `BENCHMARK_RUST_STD_PATH`.
pub fn real_world_datasets() -> Vec<(&'static str, PathBuf)> {
    let mut datasets = Vec::new();
    if let Ok(path) = std::env::var("BENCHMARK_KERNEL_PATH") {
        let p = PathBuf::from(&path);
        if p.is_dir() {
            datasets.push(("linux_kernel", p));
        }
    }
    if let Ok(path) = std::env::var("BENCHMARK_RUST_STD_PATH") {
        let p = PathBuf::from(&path);
        if p.is_dir() {
            datasets.push(("rust_stdlib", p));
        }
    }
    datasets
}

/// Peak memory in MB (high-water mark).
///
/// Uses VmHWM on Linux, mach_task_self on macOS.
/// Returns 0.0 on unsupported platforms.
pub fn peak_memory_mb() -> f64 {
    read_peak_memory().unwrap_or(0.0)
}

#[cfg(target_os = "linux")]
fn read_peak_memory() -> Option<f64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmHWM:") {
            let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
            return Some(kb as f64 / 1024.0);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn read_peak_memory() -> Option<f64> {
    // mach_task_self not available without libc dep — use proc_pidinfo
    None
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn read_peak_memory() -> Option<f64> {
    None
}

/// Human-readable byte count.
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Default Criterion configuration: warm_up=3s, measurement=15s, samples=50.
pub fn criterion_config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(15))
        .sample_size(50)
}

/// Configuration for slow benchmarks (>500ms per iteration).
/// Uses fewer samples and longer measurement time to avoid warnings.
pub fn slow_criterion_config() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(60))
        .sample_size(10)
}

/// Returns the path to the bench_data directory.
pub fn bench_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench_data")
}

// ── System metadata ──────────────────────────────────────────

/// Gathers system metadata for benchmark reports.
pub struct BenchmarkMetadata {
    pub cpu: String,
    pub ram: String,
    pub threads: usize,
    pub os: String,
    pub rust_version: String,
    pub commit: String,
}

impl BenchmarkMetadata {
    /// Gathers metadata from the current system.
    pub fn gather() -> Self {
        Self {
            cpu: gather_cpu(),
            ram: gather_ram(),
            threads: gather_threads(),
            os: gather_os(),
            rust_version: gather_rust_version(),
            commit: gather_commit(),
        }
    }

    /// Prints metadata to stdout.
    pub fn print(&self) {
        println!();
        println!("Benchmark metadata:");
        println!("  CPU:      {}", self.cpu);
        println!("  RAM:      {}", self.ram);
        println!("  Threads:  {}", self.threads);
        println!("  OS:       {}", self.os);
        println!("  Rust:     {}", self.rust_version);
        println!("  Commit:   {}", self.commit);
        println!();
    }

    /// Writes metadata as JSON to the given path.
    pub fn write_json(&self, path: &Path) {
        let json = format!(
            r#"{{
  "cpu": {},
  "ram": {},
  "threads": {},
  "os": {},
  "rust": {},
  "commit": {}
}}"#,
            quote(&self.cpu),
            quote(&self.ram),
            self.threads,
            quote(&self.os),
            quote(&self.rust_version),
            quote(&self.commit),
        );
        let _ = fs::create_dir_all(path.parent().unwrap());
        let _ = fs::write(path, json);
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Writes benchmark metadata to stdout and target/criterion/metadata.json.
/// Call once at the start of each benchmark binary.
pub fn write_metadata() {
    let meta = BenchmarkMetadata::gather();
    meta.print();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../../target/criterion/metadata.json");
    meta.write_json(&path);
}

fn run_cmd(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn gather_cpu() -> String {
    #[cfg(target_os = "linux")]
    {
        let out = run_cmd("lscpu", &[]);
        for line in out.lines() {
            if let Some(rest) = line.strip_prefix("Model name:") {
                return rest.trim().to_string();
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = run_cmd("sysctl", &["-n", "machdep.cpu.brand_string"]);
        if !out.is_empty() {
            return out;
        }
    }
    "unknown".to_string()
}

fn gather_ram() -> String {
    #[cfg(target_os = "linux")]
    {
        let out = run_cmd("free", &["-b"]);
        for line in out.lines() {
            if let Some(rest) = line.strip_prefix("Mem:") {
                if let Some(total_str) = rest.split_whitespace().next() {
                    if let Ok(bytes) = total_str.parse::<u64>() {
                        return format_bytes(bytes);
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = run_cmd("sysctl", &["-n", "hw.memsize"]);
        if let Ok(bytes) = out.parse::<u64>() {
            return format_bytes(bytes);
        }
    }
    "unknown".to_string()
}

fn gather_threads() -> usize {
    #[cfg(target_os = "linux")]
    {
        let out = run_cmd("nproc", &[]);
        if let Ok(n) = out.parse() {
            return n;
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = run_cmd("sysctl", &["-n", "hw.ncpu"]);
        if let Ok(n) = out.parse() {
            return n;
        }
    }
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn gather_os() -> String {
    #[cfg(target_os = "linux")]
    {
        let out = run_cmd("cat", &["/etc/os-release"]);
        for line in out.lines() {
            if let Some(rest) = line.strip_prefix("PRETTY_NAME=") {
                return rest.trim_matches('"').to_string();
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let name = run_cmd("sw_vers", &["-productName"]);
        let version = run_cmd("sw_vers", &["-productVersion"]);
        if !name.is_empty() {
            return if version.is_empty() {
                name
            } else {
                format!("{name} {version}")
            };
        }
    }
    std::env::consts::OS.to_string()
}

fn gather_rust_version() -> String {
    let out = run_cmd("rustc", &["--version"]);
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

fn gather_commit() -> String {
    let out = run_cmd("git", &["rev-parse", "--short", "HEAD"]);
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}
