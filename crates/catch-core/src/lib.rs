//! Catch's rules. No filesystem, WebView, process, or operating-system access.
//! Callers supply time and identity, then persist the returned candidate before
//! exposing it as authoritative state.
pub mod growth;
pub mod recurrence;
pub mod resources;
pub mod todos;
pub mod tracking;

pub type Result<T> = std::result::Result<T, String>;
