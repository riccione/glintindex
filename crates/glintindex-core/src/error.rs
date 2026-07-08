use std::fmt;

#[derive(Debug)]
pub enum CoreError {
    Index(String),
    Io(String),
    Config(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Index(msg) => write!(f, "index error: {msg}"),
            CoreError::Io(msg) => write!(f, "io error: {msg}"),
            CoreError::Config(msg) => write!(f, "config error: {msg}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        CoreError::Io(err.to_string())
    }
}
