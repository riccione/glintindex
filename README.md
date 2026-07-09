# GlintIndex

A fast, local desktop search engine built in Rust. GlintIndex indexes your files and provides instant full-text search without sending data to the cloud.

## Overview

GlintIndex crawls configured directories, parses text content from 27+ file types, and builds a Tantivy-powered search index. It supports binary detection, configurable ignore rules, and provides both a CLI and a GUI frontend.

**Key features:**

- Full-text search across local files
- Supports source code, markup, config, and documentation files
- Binary file detection (skips non-text content automatically)
- Configurable folder indexing with ignore rules
- CLI and GUI interfaces
- All processing happens locally — no network required

## Architecture

```
glintindex/
├── crates/
│   ├── glintindex-core    # Core library: indexing, scanning, search, config
│   ├── glintindex-cli     # Command-line interface
│   └── glintindex-gui     # GUI frontend (Iced)
```

| Crate | Description |
|-------|-------------|
| `glintindex-core` | Application service, filesystem scanner, Tantivy index, configuration |
| `glintindex-cli` | Thin CLI frontend using clap |
| `glintindex-gui` | Desktop GUI using Iced |

## Supported File Types

Source code: `.rs`, `.c`, `.cpp`, `.h`, `.hpp`, `.py`, `.go`, `.java`, `.kt`, `.js`, `.jsx`, `.ts`, `.tsx`

Markup & data: `.html`, `.css`, `.scss`, `.xml`, `.json`, `.yaml`, `.yml`, `.toml`, `.csv`

Documentation: `.txt`, `.md`, `.log`

Scripts: `.sh`, `.sql`

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tantivy` | 0.26.1 | Full-text search engine |
| `clap` | 4.6.1 | CLI argument parsing |
| `walkdir` | 2.5.0 | Recursive directory traversal |
| `serde` | 1.0.228 | Serialization |
| `toml` | 0.8 | Configuration file format |
| `thiserror` | 2.0.18 | Error handling |
| `anyhow` | 1.0.103 | Error context (CLI) |
| `tracing` | 0.1.44 | Structured logging |
| `iced` | 0.14.0 | GUI framework |
| `notify` | 8.2.0 | Filesystem watching (planned) |

## Requirements

- Rust 1.85 or later (edition 2024)
- Linux, macOS, or Windows

## Installation

### From source

```bash
git clone https://github.com/zer0/dev/rust/glintindex.git
cd glintindex
cargo build --release
```

The binary will be at `target/release/glintindex`.

### Using cargo install

```bash
cargo install --path crates/glintindex-cli
```

## Usage

### Index files

```bash
# Index all configured folders
glintindex index

# Index a specific folder
glintindex index --folder /path/to/docs
```

### Search

```bash
glintindex search "function definition"
glintindex search "TODO fix"
```

### View statistics

```bash
glintindex stats
```

### Rebuild index

```bash
glintindex rebuild
```

### View configuration

```bash
glintindex config
```

### Options

```
-v, --verbose          Enable verbose logging
-c, --config <path>    Config file path (default: index.toml)
-h, --help             Print help
-V, --version          Print version
```

## Configuration

GlintIndex uses a TOML configuration file. Default location: `index.toml`

```toml
indexed_folders = [
  { path = "/home/user/projects", enabled = true },
  { path = "/home/user/docs", enabled = true },
]

ignored_folders = [".git", "node_modules", "target", "__pycache__"]

index_directory = "/home/user/.local/share/glintindex/index"

max_preview_size = 200
```

If no config file exists, defaults are used with the index stored at `~/.local/share/glintindex/index`.

## Development

### Build

```bash
cargo build
```

### Run tests

```bash
cargo test --workspace
```

### Lint

```bash
cargo clippy --all-targets -- -D warnings
```

### Format

```bash
cargo fmt --all
```

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Make your changes
4. Run tests and lints (`cargo test --workspace && cargo clippy --all-targets -- -D warnings`)
5. Format code (`cargo fmt --all`)
6. Commit with conventional style (`feat(core): add new feature`)
7. Push and open a pull request

### Commit convention

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat(scope):` new feature
- `fix(scope):` bug fix
- `chore(scope):` maintenance
- `docs(scope):` documentation
- `test(scope):` tests
- `refactor(scope):` code restructuring

Scope is typically `core`, `cli`, or `gui`.

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.
