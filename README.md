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
| `glintindex-gui` | Desktop GUI using GTK4 |

## Supported File Types

Source code: `.rs`, `.c`, `.cpp`, `.h`, `.hpp`, `.py`, `.go`, `.java`, `.kt`, `.js`, `.jsx`, `.ts`, `.tsx`

Markup & data: `.html`, `.css`, `.scss`, `.xml`, `.json`, `.yaml`, `.yml`, `.toml`, `.csv`

Documentation: `.txt`, `.md`, `.log`

Scripts: `.sh`, `.sql`

Documents: `.pdf`, `.docx`, `.xlsx`, `.pptx`, `.rtf`, `.odt`

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
| `notify` | 8.2.0 | Filesystem watching |

### Cargo Features

The `glintindex-core` crate includes optional parser features, all enabled by default:

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `parser-pdf` | PDF parsing | `pdf-extract` |
| `parser-docx` | DOCX parsing | `docx-lite`, `zip` |
| `parser-xlsx` | XLSX parsing | `calamine` |
| `parser-pptx` | PPTX parsing | `zip`, `quick-xml` |
| `parser-rtf` | RTF parsing | `rtf-parser` |
| `parser-odt` | ODT parsing | `zip`, `quick-xml` |

To build without document parsers, disable default features:

```bash
cargo build --no-default-features -p glintindex-core
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Controls log level when using `--verbose`. Default: `info`. Example: `RUST_LOG=debug` |

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

This builds two binaries:

- `target/release/glintindex` — GUI application
- `target/release/glintindex-cli` — CLI tool

### Using cargo install

```bash
cargo install --path crates/glintindex-cli
```

This installs the `glintindex-cli` binary.

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
3. Run `glintindex.exe` (GUI) or `glintindex-cli.exe` (CLI)

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
glintindex-cli init
```

### Index files

```bash
# Index all configured folders
glintindex-cli index

# Index a specific folder (also adds it to configuration)
glintindex-cli index --folder ~/Documents
```

### Manage folders

```bash
# List all configured folders
glintindex-cli folders list

# Add a folder to configuration
glintindex-cli folders add ~/Documents

# Remove a folder from configuration
glintindex-cli folders remove ~/Documents

# Enable a disabled folder
glintindex-cli folders enable ~/Documents

# Disable a folder (keeps it in config)
glintindex-cli folders disable ~/Documents
```

### Search

```bash
glintindex-cli search "function definition"
glintindex-cli search "TODO fix"
```

### View statistics

```bash
glintindex-cli stats
```

### Clear index

```bash
# Clear with confirmation prompt
glintindex-cli clear

# Skip confirmation
glintindex-cli clear -y
glintindex-cli clear --yes
```

### Rebuild index

Discards all existing index data, recreates the index structure, and re-indexes all configured folders.

```bash
glintindex-cli rebuild
```

### View configuration

Displays current configuration: indexed folders, ignored folders, index directory, and preview settings.

```bash
glintindex-cli config
```

### Options

```
-v, --verbose          Enable verbose logging (respects RUST_LOG)
-c, --config <path>    Config file path (default: platform-specific)
-h, --help             Print help
-V, --version          Print version
```

## GUI

The GUI application provides a graphical interface for searching and managing your indexed files.

```bash
glintindex-gui
```

### Features

- **Live search**: Results update as you type
- **File preview**: Syntax-highlighted preview with line numbers
- **File actions**: Open files, reveal in file manager, copy file path
- **Settings**: Configure theme (light/dark/system), font size, indexed folders, and ignored folders
- **Index management**: Index all folders, rebuild index, or clear index from the GUI

### Settings

Access settings via the hamburger menu or keyboard shortcut:

- **General**: Configure indexed and ignored folders
- **Appearance**: Theme selection, font size (8-32pt)
- **Index**: View index statistics, trigger rebuild or clear

## Configuration

GlintIndex uses a TOML configuration file. Default locations:

| Platform | Config Path |
|----------|-------------|
| Linux | `~/.config/glintindex/config.toml` |
| macOS | `~/Library/Application Support/GlintIndex/config.toml` |
| Windows | `%APPDATA%/GlintIndex/config.toml` |

```toml
indexed_folders = [
  { path = "/home/user/projects", enabled = true },
  { path = "/home/user/docs", enabled = true },
]

ignored_folders = [".git", ".svn", ".hg", "node_modules", "__pycache__", ".DS_Store"]

index_directory = "/home/user/.local/share/glintindex/index"

max_preview_size = 200

theme = "system"  # Options: "light", "dark", "system"
font_size = 12    # Range: 8-32
```

If no config file exists, defaults are used with the index stored at the platform-specific location (e.g., `~/.local/share/glintindex/index` on Linux).

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
