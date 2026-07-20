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

// ~keep Enabling both allocator features at once is rejected at compile time with a
// ~keep clear message instead of the raw duplicate-`#[global_allocator]` errors.
// ~keep `compile_error!` is a macro, so it must be a standalone item gated by `cfg`
// ~keep — it cannot be used as a `cfg_attr` attribute.
#[cfg(all(feature = "mimalloc", feature = "jemalloc"))]
compile_error!(
    "Features `mimalloc` and `jemalloc` are mutually exclusive. \
     Enable at most one global allocator feature."
);

#[cfg(feature = "mimalloc")]
#[global_allocator]
// ~keep SAFETY: `mimalloc::MiMalloc` is a stateless ZST implementing GlobalAlloc.
// ~keep The crate guarantees thread-safety and allocator-contract compliance.
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
// ~keep SAFETY: `tikv_jemallocator::Jemalloc` delegates GlobalAlloc to jemalloc.
// ~keep Thread-safety and allocator-contract compliance are guaranteed upstream.
static ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
