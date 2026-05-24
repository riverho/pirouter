//! pirouter — lightweight LLM routing daemon.
//!
//! Public modules are exposed for integration tests and for embedding the
//! router inside another Rust program. The CLI in `main.rs` is the primary
//! entry point.

pub mod config;
pub mod ledger;
pub mod providers;
pub mod router;
pub mod server;
pub mod types;

pub use config::Config;
pub use types::{Message, Request, Response, Role};
