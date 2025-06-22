use std::fmt::{Debug, Display};

#[derive(Debug)]
pub struct QbitError(pub qbit::Error);

impl std::error::Error for QbitError {}
impl Display for QbitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}
impl From<qbit::Error> for QbitError {
    fn from(value: qbit::Error) -> Self {
        QbitError(value)
    }
}
