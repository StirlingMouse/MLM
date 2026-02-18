use dioxus::prelude::ServerFnError;

/// Helper trait for converting errors to `ServerFnError`.
pub trait IntoServerFnError<T> {
    /// Convert an error to `ServerFnError` using the error's Display representation.
    fn server_err(self) -> Result<T, ServerFnError>;

    /// Convert an error to `ServerFnError` with a context message.
    fn server_err_ctx(self, msg: &str) -> Result<T, ServerFnError>;
}

impl<T, E: std::fmt::Display> IntoServerFnError<T> for Result<T, E> {
    fn server_err(self) -> Result<T, ServerFnError> {
        self.map_err(|e| ServerFnError::new(e.to_string()))
    }

    fn server_err_ctx(self, msg: &str) -> Result<T, ServerFnError> {
        self.map_err(|e| ServerFnError::new(format!("{}: {}", msg, e)))
    }
}

/// Helper trait for converting Option to `ServerFnError`.
pub trait OptionIntoServerFnError<T> {
    /// Convert `None` to `ServerFnError` with a message.
    fn ok_or_server_err(self, msg: &str) -> Result<T, ServerFnError>;
}

impl<T> OptionIntoServerFnError<T> for Option<T> {
    fn ok_or_server_err(self, msg: &str) -> Result<T, ServerFnError> {
        self.ok_or_else(|| ServerFnError::new(msg.to_string()))
    }
}
