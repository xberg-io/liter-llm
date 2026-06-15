//! Global allocator selection for the `liter-llm` CLI binary.
//!
//! Exactly one of the following is selected at compile time:
//!
//! | Feature      | Allocator                          |
//! |--------------|------------------------------------|
//! | `mimalloc`   | Microsoft's mimalloc               |
//! | `jemalloc`   | jemalloc via tikv-jemallocator     |
//! | *(neither)*  | System allocator (Rust default)    |
//!
//! Enabling both `mimalloc` and `jemalloc` simultaneously is a compile-time
//! error: the two `#[global_allocator]` attributes would conflict and Rustc
//! would reject the crate.  Cargo feature resolution does not prevent this
//! automatically, so callers must not enable both.

// Only one allocator is active per build; the unused imports below are
// conditional on features that are mutually exclusive by convention.
#![cfg_attr(
    all(feature = "mimalloc", feature = "jemalloc"),
    compile_error(
        "Features `mimalloc` and `jemalloc` are mutually exclusive. \
         Enable at most one global allocator feature."
    )
)]

#[cfg(feature = "mimalloc")]
#[global_allocator]
// SAFETY: `mimalloc::MiMalloc` is a stateless ZST that implements
// `GlobalAlloc` correctly.  The crate guarantees thread-safety and the
// allocator does not impose any alignment constraints beyond those required
// by the Rust allocator contract.
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
// SAFETY: `tikv_jemallocator::Jemalloc` is a stateless ZST that implements
// `GlobalAlloc` by delegating to jemalloc.  Thread-safety and allocator
// contract compliance are guaranteed by the upstream crate.
static ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
