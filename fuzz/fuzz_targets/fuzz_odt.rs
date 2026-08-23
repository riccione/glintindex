#![no_main]

use glintindex_core::ParserRegistry;
use libfuzzer_sys::fuzz_target;
use std::path::Path;
use std::sync::OnceLock;

static REGISTRY: OnceLock<ParserRegistry> = OnceLock::new();

fuzz_target!(|data: &[u8]| {
    let registry = REGISTRY.get_or_init(ParserRegistry::new);
    let path = Path::new("test.odt");
    let parser = registry.parser_for(path);
    let _ = parser.parse(data, path);
});
