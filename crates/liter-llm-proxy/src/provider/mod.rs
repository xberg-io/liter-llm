//! Provider credential management for the liter-llm proxy.
//!
//! This module provides:
//!
//! - [`credential_pool::CredentialPool`] — trait for managing a rotating pool
//!   of API credentials per provider.
//! - [`credential_pool_memory::InMemoryCredentialPool`] — default in-memory
//!   implementation backed by `DashMap`.
//! - [`auto_cycle`] — Tower layer that intercepts 429 / 5xx responses and
//!   automatically rotates to the next credential.
//! - [`metrics`] — OTel helpers for `gen_ai.credential.*` instruments.

pub mod auto_cycle;
pub mod credential_pool;
pub mod credential_pool_memory;
pub(crate) mod metrics;

pub use auto_cycle::{AutoCycleLayer, AutoCycleService};
pub use credential_pool::{CredentialError, CredentialHandle, CredentialPool, PoolSnapshot};
pub use credential_pool_memory::InMemoryCredentialPool;
