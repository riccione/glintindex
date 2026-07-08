/// Initializes the tracing subscriber for the application.
///
/// This function sets up a [`tracing_subscriber`] that logs to stderr
/// with the given maximum log level. It should be called once at
/// application startup, typically from the binary entry point.
///
/// The library itself never calls this function. Initialization
/// is the responsibility of the consuming binary (CLI or GUI).
///
/// # Arguments
///
/// * `level` - The maximum log level to accept (e.g., `"info"`, `"debug"`).
///
/// # Panics
///
/// Panics if the subscriber cannot be set (e.g., if called more than once
/// in the same process).
pub fn init(level: &str) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[cfg(test)]
mod tests {
    #[test]
    fn env_filter_creation() {
        use tracing_subscriber::EnvFilter;

        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        assert!(filter.to_string().contains("info"));
    }
}
