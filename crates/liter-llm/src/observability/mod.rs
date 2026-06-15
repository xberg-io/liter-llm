/// Canonical usage events and pluggable sinks.
pub mod usage;

pub use usage::{
    CacheState, LoggingUsageSink, MultiUsageSink, UsageEvent, UsageEventOutcome, UsageSink, UsageSinkError,
};
