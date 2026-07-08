#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Message {
    Tick,
    SearchQueryChanged(String),
}
