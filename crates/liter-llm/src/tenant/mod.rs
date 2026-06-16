//! Generic multi-tenant primitives for propagating tenant identity through the
//! Tower request stack.
//!
//! # Overview
//!
//! - [`TenantId`] / [`TenantContext`] — carry tenant identity on each request.
//! - [`KeyResolver`] / [`ResolvedKey`] / [`KeyResolverError`] — trait and types
//!   for resolving raw API tokens to tenant metadata.
//! - [`InMemoryKeyResolver`] — built-in in-memory implementation for tests and
//!   simple deployments.
//!
//! # Wiring
//!
//! Attach a tenant to any [`LlmRequest`][crate::tower::types::LlmRequest] via
//! [`LlmRequest::with_tenant_id`].  Tower layers (budget, cache, hooks) read
//! [`LlmRequest::tenant_id`] automatically.

pub mod context;
#[cfg(feature = "etcd-key-resolver")]
pub mod etcd;
pub mod in_memory;
pub mod resolver;

pub use context::{TenantContext, TenantId};
#[cfg(feature = "etcd-key-resolver")]
pub use etcd::{EtcdKeyResolver, EtcdKeyResolverConfig};
pub use in_memory::InMemoryKeyResolver;
pub use resolver::{KeyResolver, KeyResolverError, ResolvedKey};
