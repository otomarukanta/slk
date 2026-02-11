#[derive(Debug)]
pub struct SlkError {
    pub message: String,
}

impl std::fmt::Display for SlkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<String> for SlkError {
    fn from(s: String) -> Self {
        SlkError { message: s }
    }
}

impl From<&str> for SlkError {
    fn from(s: &str) -> Self {
        SlkError {
            message: s.to_string(),
        }
    }
}
