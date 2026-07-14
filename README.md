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
│   └── glintindex-gui     # GUI frontend (GTK4)
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
| `gtk4` | 0.11.4 | GUI framework |
| `notify` | 8.2.0 | Filesystem watching (planned) |

## Requirements

- Rust 1.85 or later (edition 2024)
- Linux, macOS, or Windows

## Installation

### From source

```bash
git clone https://github.com/riccione/glintindex.git
cd glintindex
cargo build --release
```

The binary will be at `target/release/glintindex`.

### Using cargo install

```bash
cargo install --path crates/glintindex-cli
```

### Pre-built releases

Download pre-built binaries from [GitHub Releases](https://github.com/riccione/glintindex/releases).

#### Linux (.deb)

```bash
# Download the .deb package for your version
sudo dpkg -i glintindex_<version>_amd64.deb

# Fix any missing dependencies
sudo apt-get install -f
```

**Requirements:** GTK4 must be installed on your system.

```bash
# Debian/Ubuntu
sudo apt-get install libgtk-4-0

# Fedora
sudo dnf install gtk4

# Arch
sudo pacman -S gtk4
```

#### Windows

1. Download `glintindex-windows-x64-<version>.zip`
2. Extract the ZIP to a directory of your choice
3. Run `glintindex-gui.exe`

**Note:** GTK4 runtime libraries are bundled in the ZIP.

#### macOS

1. Download the appropriate ZIP for your architecture:
   - `glintindex-macos-<version>.zip` (Apple Silicon / arm64)
   - `glintindex-macos-x86-64-<version>.zip` (Intel / x86_64)
2. Extract and move `GlintIndex.app` to your Applications folder
3. Right-click and select "Open" the first time (unsigned app warning)

**Note:** The `.app` bundle is self-contained with GTK4 libraries included.

## Usage

### Initialize configuration

```bash
glintindex init
```

### Index files

```bash
# Index all configured folders
glintindex index

# Index a specific folder (also adds it to configuration)
glintindex index --folder ~/Documents
```

### Manage folders

```bash
# List all configured folders
glintindex folders list

# Add a folder to configuration
glintindex folders add ~/Documents

# Remove a folder from configuration
glintindex folders remove ~/Documents

# Enable a disabled folder
glintindex folders enable ~/Documents

# Disable a folder (keeps it in config)
glintindex folders disable ~/Documents
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

### Clear index

```bash
# Clear with confirmation prompt
glintindex clear

# Skip confirmation
glintindex clear --yes
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

## Known Limitations (MVP)

- **macOS:** The `.app` bundle is self-contained but not code-signed or notarized. Users will see a Gatekeeper warning on first launch.
- **macOS:** No DMG disk image distribution. ZIP is provided for the MVP.
- **macOS:** No custom application icon yet (uses default app icon).
- **macOS x86_64:** Built via cross-compilation. Some edge cases with GTK4 libraries may exist.
- **Windows:** No MSI or NSIS installer. ZIP distribution only.
- **Windows:** GTK4 runtime DLLs are bundled, but some rare transitive dependencies may be missing.
- **Linux:** Requires GTK4 system libraries to be installed (`libgtk-4-0` or equivalent).

## Release

Releases are automated via GitHub Actions. When a tag matching `v*` is pushed, the workflow:

1. Runs format, lint, and test checks
2. Builds release binaries for Linux, Windows, and macOS
3. Packages artifacts (.deb, .zip, source tarball)
4. Creates a GitHub Release with generated release notes

See [`.github/workflows/release.yml`](.github/workflows/release.yml) for details.

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.
