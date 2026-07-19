## [0.2.0] - 2026-07-19

### Bug Fixes

- *(ci)* Resolve Windows GTK4 DLL collection failure (#36)
- *(gui)* Suppress console window on Windows release builds (#38)

### Documentation

- Initialize git-cliff configuration and generate CHANGELOG for v0.1.0 (#35)

### Features

- *(gui)* Replace GTK file chooser with rfd native dialog (#41)
- *(gui)* Add first-run empty state for unconfigured applications (#46)

### Miscellaneous Tasks

- Strip release step and configure artifact upload for windows testing (#43)

### Refactor

- *(preview)* Unify preview rendering on indexed content (#45)
## [0.1.0] - 2026-07-15

### Bug Fixes

- *(core)* Resolve field_reassign_with_default clippy warnings in tests
- *(core)* Reload reader after index rebuild to ensure stale data is cleared
- *(core)* Use lowercase theme value in generated config file
- *(core)* Include all required fields in generated config file
- *(core)* Expand tilde in index_directory path after config loading
- *(core)* Use dirs::data_local_dir() for cross-platform default paths
- *(core)* Use platform-specific paths in generated config and tilde expansion
- *(gui)* Fill remaining vertical space in split view
- Resolve clippy warnings for preview integration
- Remove unused import in preview tests
- *(gui)* Render syntax-highlighted text on gray background
- *(cli)* Use tempfile::TempDir in init tests to fix CI failure
- *(core)* Share IndexService between main thread and background workers
- *(core)* Run_rebuild() now re-indexes after clearing the index
- *(core)* Pre-count files before scanning for accurate progress bar
- *(gui)* Enable text selection in preview by providing on_action
- *(gui)* Reuse PreviewService across clicks to avoid reloading syntax definitions
- *(gui)* Sanitize preview text to prevent cosmic-text crash on RTL characters
- *(gui)* Fix settings sidebar navigation not switching pages
- *(gui)* Fix Stack child name mismatch in settings sidebar
- *(gui)* Show user-friendly names in settings sidebar
- *(gui)* Add close button and toggle behavior to Settings window
- *(gui)* Make search results display in the results list
- *(gui)* Show preview in right pane when clicking search result
- *(gui)* Remove open::that() from results list that opened shell
- *(gui)* Fix Settings window close by button and Esc key
- *(gui)* Remove modal from Settings window so button toggle works
- *(core)* Prevent duplicate documents on re-index
- *(gui)* Use set_visible(false) instead of close() for Settings window
- *(gui)* Pass window reference to general::build() to prevent crash
- *(gui)* Remove self-grouping of CheckButton in Appearance settings
- *(gui)* Stop idle callbacks that caused 100% CPU after window close
- *(gui)* Fix theme system with explicit colors and single CssProvider
- *(gui)* Remove expect() from ThemeManager and fix font CSS provider lifecycle
- *(gui)* Embed CSS files in binary to fix theme switching
- *(gui)* Add text color and headerbar styling to dark theme
- *(gui)* Fix clipboard, dark theme, and header bar rendering

### Documentation

- Add comprehensive README with setup, usage, and contributing guide
- Update README with folders and clear command documentation
- *(readme)* Fix inaccuracies and add missing documentation (#32)

### Features

- *(core)* Add core library skeleton with module stubs
- *(cli)* Add cli binary skeleton with clap argument parsing
- *(gui)* Add gui binary skeleton with iced window
- *(core)* Add centralized error handling with thiserror
- *(core)* Add domain model types for documents, queries, results, and folders
- *(core)* Add configuration system with TOML loader
- *(core)* Add public traits for document indexing, scanning, and search
- *(core)* Add tracing logging initialization
- *(core)* Wire up public API exports in lib.rs
- *(core)* Extend error handling with Tantivy error conversions
- *(core)* Add Tantivy schema and domain mapping layer
- *(core)* Add IndexStatistics type for index health metrics
- *(core)* Implement IndexService with full Tantivy-backed search engine
- *(core)* Wire up index module facade and public API exports
- *(core)* Add walkdir dependency for filesystem scanning
- *(core)* Implement filesystem scanner with directory walking, ignore rules, and binary detection
- *(core)* Wire up scanner module facade and public API exports
- *(core)* Add ApplicationStatistics and IndexingResult types
- *(core)* Implement ApplicationService with indexing, search, and lifecycle methods
- *(core)* Wire up app module facade and public API exports
- *(cli)* Rename binary to glintindex and add tracing dependencies
- *(cli)* Implement CLI argument parsing with clap derive API
- *(cli)* Implement index, search, stats, rebuild, and config commands
- *(cli)* Add graceful error handling with non-zero exit code
- *(core)* Add config file auto-generation with commented examples
- *(cli)* Add init command for first-time setup
- *(cli)* Auto-generate config on first index run
- *(core)* Add dirs crate for cross-platform directory resolution
- *(cli)* Persist --folder to index.toml when indexing
- *(core)* Add folder management and clear_index to ApplicationService
- *(cli)* Add folders and clear commands
- *(core)* Add filesystem watcher for automatic incremental index updates
- *(gui)* Implement application message types
- *(gui)* Implement application state management
- *(gui)* Implement search bar widget
- *(gui)* Implement results list and preview widgets
- *(gui)* Implement status bar widget
- *(gui)* Implement main page layout
- *(gui)* Implement Iced application entry point
- *(gui)* Add minimal theme module
- *(gui)* Implement main entry point with Iced application
- *(core)* Add ignored_folders management to ApplicationService
- *(gui)* Extend message types for settings window
- *(gui)* Extend application state for settings management
- *(gui)* Implement settings window pages and navigation
- *(gui)* Wire settings into application and add settings button
- *(core)* Add AppPaths as single source of truth for filesystem locations
- *(cli)* Use AppPaths for default config path
- *(gui)* Use ApplicationService::with_default_config() instead of hardcoded paths
- Implement better search experience
- *(core)* Implement preview module with syntax highlighting and encoding detection
- *(gui)* Integrate preview service with syntax highlighting and line numbers
- *(core)* Add rich document parsers for PDF, DOCX, XLSX, PPTX, RTF, and ODT
- *(core)* Make document parsing pipeline fault-tolerant
- *(core,gui)* Implement background indexing with progress reporting
- *(core,cli,gui)* Implement indexing progress reporting
- *(gui)* Make preview pane text selectable via text_editor
- *(gui)* Migrate from Iced to GTK4
- *(gui)* Implement full Settings window in GTK4
- *(core)* Add SQLite metadata storage for indexed files
- *(core)* Metadata-based incremental indexing with detailed statistics
- *(core,cli,gui)* Show same indexing stats after rebuild in CLI and GUI
- *(gui)* Add configurable application font size
- *(gui)* Persistent split pane sizing for main window
- *(gui)* Implement theme system with CSS-based styling
- *(gui)* Add file actions (Open, Open Folder, Copy Path)
- *(core)* Implement prefix search using FuzzyTermQuery::new_prefix

### Miscellaneous Tasks

- Initialize cargo workspace with shared dependencies
- Add rustfmt and clippy configuration
- Add basic CI with fmt, lint and tests
- *(core)* Add toml, thiserror, and tracing-subscriber dependencies
- *(core)* Remove unused stub modules and update Cargo.lock
- *(core)* Add tantivy and tempfile dependencies
- *(core)* Remove unused search stub module
- Add index.toml to gitignore
- *(gui)* Add GUI MVP dependencies to workspace
- *(gui)* Add rfd dependency for native folder dialogs
- Update Cargo.lock after removing dirs from GUI
- Add missing gtk4 system dependencies to CI workflow
- *(ci)* Add packaging assets and update documentation for releases
- *(release)* Add release workflow with cross-platform builds
- Optimize release workflow and centralize version management
- Fix release workflow failures across Linux, Windows, and macOS (#31)
- *(release)* Adapt workflow to build and package both CLI and GUI crates (#34)

### Other

- Fix layout bug by keeping the preview action bar fixed at bottom

### Refactor

- *(gui)* Update module declarations
- *(gui)* Remove old theme directory module
- *(gui)* Introduce Appearance settings page
- *(gui)* Unify theme and font-size into single CSS provider

### Styling

- *(gui)* Apply rustfmt to settings window code
- Apply rustfmt formatting
- Apply cargo fmt
