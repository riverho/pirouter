//! Router — rule evaluation + cascade execution.
//!
//! The router is split into two halves on purpose:
//!
//! * `rules` turns a request into a `Decision` (primary model + cascade).
//!   This is *pure* and deterministic — given a config and a request, the
//!   chosen decision is fully predictable. Easy to test, easy to debug.
//!
//! * `cascade` takes a decision and actually executes it against
//!   providers, escalating on configured signals. This is where the I/O
//!   and retry logic lives.
//!
//! Keeping them separate is the heart of pirouter's context-engineering
//! pitch: the *decision* is a first-class artifact you can inspect,
//! log, and reason about independently of the call itself.

pub mod cascade;
pub mod policy;
pub mod rules;

pub use cascade::{CascadeAttempt, CascadeOutcome};
pub use rules::ResolvedDecision;
