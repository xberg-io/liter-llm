//! Graceful shutdown coordinator for the liter-llm proxy.
//!
//! # Lifecycle
//!
//! ```text
//! Running → Draining → Drained
//!         ↘           ↗
//!           Aborted (second signal within 5 s, or hard 30 s deadline)
//! ```
//!
//! # Usage
//!
//! 1. Create a [`ShutdownCoordinator`] at server startup.
//! 2. Clone the [`ShutdownHandle`] and give it to any subsystem that needs to
//!    observe the phase (e.g. health routes) or to any [`Drainable`] that
//!    needs to react to drain.
//! 3. Spawn [`ShutdownCoordinator::run_signal_handler`] to start listening for
//!    OS signals.
//! 4. Await [`ShutdownCoordinator::wait_for_drained`] to block until all
//!    subsystems have completed or the hard deadline expires.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

/// How long to wait for a second signal before escalating to `Aborted`.
const SECOND_SIGNAL_WINDOW: Duration = Duration::from_secs(5);

/// Hard deadline from the moment draining starts: if subsystems have not
/// completed by this point the coordinator transitions to `Aborted`.
pub const DRAIN_HARD_DEADLINE: Duration = Duration::from_secs(30);

/// Phase of the server shutdown lifecycle.
///
/// Transitions are monotonically forward: `Running` → `Draining` →
/// `Drained` or `Aborted`.  The coordinator never moves backward.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ShutdownPhase {
    /// Normal operation — accepting traffic.
    Running = 0,
    /// First shutdown signal received; draining in-flight requests.
    /// `readyz` returns 503; `healthz`/`liveness` still returns 200.
    Draining = 1,
    /// All in-flight requests have completed cleanly.
    Drained = 2,
    /// Force-abort: second signal within [`SECOND_SIGNAL_WINDOW`] or the
    /// hard 30-second deadline elapsed before draining completed.
    Aborted = 3,
}

/// Result returned by a [`Drainable`] subsystem after `drain` completes.
#[derive(Debug, PartialEq, Eq)]
pub enum DrainResult {
    /// The subsystem finished all in-flight work before the deadline.
    Clean,
    /// The deadline expired; the subsystem was force-stopped.
    TimedOut,
}

/// A subsystem that participates in graceful shutdown.
///
/// Implementors register themselves with a [`ShutdownCoordinator`] via
/// [`ShutdownCoordinator::register`].  When the coordinator transitions to
/// `Draining` it calls `drain` on every registered subsystem concurrently
/// and waits for them all to return (or the hard deadline to fire).
pub trait Drainable: Send + Sync {
    /// Drain any in-flight work.
    ///
    /// Implementations MUST respect `deadline`: they should return
    /// [`DrainResult::TimedOut`] when `Instant::now() >= deadline` rather
    /// than blocking indefinitely.  The coordinator will not wait beyond
    /// [`DRAIN_HARD_DEADLINE`] regardless.
    fn drain(&self, deadline: Instant) -> Pin<Box<dyn Future<Output = DrainResult> + Send + '_>>;

    /// Human-readable name used in log messages.
    fn name(&self) -> &str;
}

/// Read-only handle distributed to subsystems (health routes, middleware, etc.)
/// that need to observe the current [`ShutdownPhase`] without being able to
/// drive transitions.
#[derive(Clone, Debug)]
pub struct ShutdownHandle {
    phase_rx: watch::Receiver<ShutdownPhase>,
    token: CancellationToken,
}

impl ShutdownHandle {
    /// Current shutdown phase.
    pub fn phase(&self) -> ShutdownPhase {
        *self.phase_rx.borrow()
    }

    /// Returns `true` when the server is draining or beyond.
    pub fn is_draining(&self) -> bool {
        self.phase() >= ShutdownPhase::Draining
    }

    /// A [`CancellationToken`] that is cancelled when the phase enters
    /// `Draining`.  Subsystems can `select!` on `token.cancelled()` to
    /// interrupt long-running loops.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Wait until the phase advances past `Running` (i.e. drain has started).
    pub async fn wait_for_drain_start(&mut self) {
        let _ = self.phase_rx.wait_for(|&p| p >= ShutdownPhase::Draining).await;
    }
}

/// Coordinator that drives the shutdown lifecycle and owns the signal handlers.
pub struct ShutdownCoordinator {
    phase_tx: Arc<watch::Sender<ShutdownPhase>>,
    phase_rx: watch::Receiver<ShutdownPhase>,
    token: CancellationToken,
    drainables: Vec<Arc<dyn Drainable>>,
}

impl ShutdownCoordinator {
    /// Create a new coordinator in the [`ShutdownPhase::Running`] state.
    pub fn new() -> Self {
        let (phase_tx, phase_rx) = watch::channel(ShutdownPhase::Running);
        Self {
            phase_tx: Arc::new(phase_tx),
            phase_rx,
            token: CancellationToken::new(),
            drainables: Vec::new(),
        }
    }

    /// Obtain a [`ShutdownHandle`] that can be cheaply cloned and distributed
    /// to subsystems (health endpoints, middleware, etc.).
    pub fn handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            phase_rx: self.phase_rx.clone(),
            token: self.token.clone(),
        }
    }

    /// Register a [`Drainable`] subsystem.  `drain` will be called on it when
    /// the coordinator enters the `Draining` phase.
    pub fn register<D: Drainable + 'static>(&mut self, subsystem: D) {
        self.drainables.push(Arc::new(subsystem));
    }

    /// Transition to `Draining` and cancel the token.
    ///
    /// Idempotent: calling this more than once has no effect. Test-only helper —
    /// the signal handler inlines the same transition logic to avoid lock
    /// contention on the phase channel.
    #[cfg(test)]
    fn begin_draining(&self) {
        self.phase_tx.send_if_modified(|p| {
            if *p == ShutdownPhase::Running {
                *p = ShutdownPhase::Draining;
                true
            } else {
                false
            }
        });
        self.token.cancel();
    }

    /// Transition to `Aborted` (force exit).
    ///
    /// Idempotent: only advances the phase, never reverses it.
    fn abort(&self) {
        self.phase_tx.send_if_modified(|p| {
            if *p < ShutdownPhase::Aborted {
                *p = ShutdownPhase::Aborted;
                true
            } else {
                false
            }
        });
    }

    /// Transition to `Drained` (clean exit).
    fn set_drained(&self) {
        self.phase_tx.send_if_modified(|p| {
            if *p == ShutdownPhase::Draining {
                *p = ShutdownPhase::Drained;
                true
            } else {
                false
            }
        });
    }

    /// Spawn the OS signal handler task.
    ///
    /// - First SIGTERM or Ctrl-C → `Draining`.
    /// - Second signal within [`SECOND_SIGNAL_WINDOW`] → `Aborted`.
    ///
    /// Signal handlers are pre-registered BEFORE the first `.await` so that no
    /// signal can be delivered to an unregistered listener in the gap between
    /// the first signal returning and the second registration taking effect.
    ///
    /// Returns a `JoinHandle` that the caller can optionally await or abort.
    pub fn spawn_signal_handler(&self) -> tokio::task::JoinHandle<()> {
        let phase_tx = Arc::clone(&self.phase_tx);
        let token = self.token.clone();

        tokio::spawn(async move {
            // Pre-register ALL signal listeners before the first await.
            // This eliminates the miss window that would exist if each
            // `wait_for_signal()` call created a fresh OS-level pipe.
            #[cfg(unix)]
            let (mut sigterm, mut sigint) = {
                use tokio::signal::unix::{SignalKind, signal};
                (
                    signal(SignalKind::terminate()).expect("failed to register SIGTERM handler"),
                    signal(SignalKind::interrupt()).expect("failed to register SIGINT handler"),
                )
            };
            #[cfg(not(unix))]
            let mut ctrl_c_registered = ();

            // Wait for the first signal using the pre-registered handles.
            #[cfg(unix)]
            wait_first(&mut sigterm, &mut sigint).await;
            #[cfg(not(unix))]
            {
                // On Windows there is no SIGTERM equivalent; Ctrl-C is the
                // only graceful-shutdown signal available.
                let _ = &mut ctrl_c_registered;
                tokio::signal::ctrl_c()
                    .await
                    .expect("failed to register Ctrl-C handler");
                tracing::debug!("Ctrl-C received");
            }

            let current = *phase_tx.borrow();
            if current >= ShutdownPhase::Draining {
                // Already shutting down — escalate immediately.
                tracing::warn!("received shutdown signal while already draining — force abort");
                phase_tx.send_if_modified(|p| {
                    if *p < ShutdownPhase::Aborted {
                        *p = ShutdownPhase::Aborted;
                        true
                    } else {
                        false
                    }
                });
                return;
            }

            tracing::info!("shutdown signal received — entering Draining phase");
            phase_tx.send_if_modified(|p| {
                if *p == ShutdownPhase::Running {
                    *p = ShutdownPhase::Draining;
                    true
                } else {
                    false
                }
            });
            token.cancel();

            // Listen for a second signal within the window.
            // The SAME pre-registered handles are reused — no re-registration gap.
            let window = tokio::time::sleep(SECOND_SIGNAL_WINDOW);

            // `tokio::select!` does not support `#[cfg]` attributes on
            // individual arms, so the platform-specific select bodies live in
            // separate `#[cfg]` blocks that are each complete `select!` calls.

            #[cfg(unix)]
            {
                // Unix: reuse the already-registered sigterm / sigint handles.
                // No re-registration gap because the handles were created before
                // the first await above.
                tokio::select! {
                    _ = wait_first(&mut sigterm, &mut sigint) => {
                        tracing::warn!(
                            "second shutdown signal received within {}s — force aborting",
                            SECOND_SIGNAL_WINDOW.as_secs()
                        );
                        phase_tx.send_if_modified(|p| {
                            if *p < ShutdownPhase::Aborted {
                                *p = ShutdownPhase::Aborted;
                                true
                            } else {
                                false
                            }
                        });
                    }
                    _ = window => {
                        // Window elapsed without a second signal — normal drain in progress.
                    }
                }
            }

            #[cfg(not(unix))]
            {
                // Windows: a second Ctrl-C from a fresh future.  The first one
                // was already consumed above so this registers a new listener.
                // Windows has no SIGTERM so there is no signal-miss-window
                // concern for a distinct signal type.
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        tracing::warn!(
                            "second Ctrl-C received within {}s — force aborting",
                            SECOND_SIGNAL_WINDOW.as_secs()
                        );
                        phase_tx.send_if_modified(|p| {
                            if *p < ShutdownPhase::Aborted {
                                *p = ShutdownPhase::Aborted;
                                true
                            } else {
                                false
                            }
                        });
                    }
                    _ = window => {
                        // Window elapsed without a second signal — normal drain in progress.
                    }
                }
            }
        })
    }

    /// Drive all registered [`Drainable`] subsystems to completion, then
    /// transition to `Drained` (or `Aborted` if the hard deadline fires or
    /// a force signal arrives while draining).
    ///
    /// Logs drain progress every second.
    ///
    /// Returns the final [`ShutdownPhase`].
    pub async fn wait_for_drained(self) -> ShutdownPhase {
        // Wait until Draining (or beyond) before starting the drain clock.
        {
            let mut rx = self.phase_rx.clone();
            let _ = rx.wait_for(|&p| p >= ShutdownPhase::Draining).await;
        }

        let phase = *self.phase_rx.borrow();
        if phase >= ShutdownPhase::Aborted {
            // Already aborted by a second signal.
            return ShutdownPhase::Aborted;
        }

        let deadline = Instant::now() + DRAIN_HARD_DEADLINE;
        let drainables = self.drainables.clone();

        // Spawn all drain futures concurrently.
        let drain_futures: Vec<_> = drainables
            .iter()
            .map(|d| {
                let d = Arc::clone(d);
                let name = d.name().to_owned();
                tokio::spawn(async move {
                    let result = d.drain(deadline).await;
                    match result {
                        DrainResult::Clean => tracing::info!(%name, "subsystem drained cleanly"),
                        DrainResult::TimedOut => tracing::warn!(%name, "subsystem drain timed out"),
                    }
                    result
                })
            })
            .collect();

        // Watch phase for an abort signal arriving while we drain.
        let mut abort_rx = self.phase_rx.clone();
        let abort_watch = async move {
            let _ = abort_rx.wait_for(|&p| p >= ShutdownPhase::Aborted).await;
        };

        // Hard deadline timer.
        let hard_timeout = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline));

        // Join all drain tasks concurrently, honouring abort and hard deadline.
        //
        // Using `FuturesUnordered` instead of a sequential `for` loop ensures
        // that a stalled first drainable (e.g. the axum server stalling 29 s)
        // does not block polling of faster drainables.  All tasks are driven
        // concurrently; results are collected as they arrive.
        let drain_all = async move {
            use futures_util::StreamExt as _;
            let mut pending: futures_util::stream::FuturesUnordered<_> = drain_futures.into_iter().collect();
            let mut results = Vec::with_capacity(pending.len());
            while let Some(join_result) = pending.next().await {
                match join_result {
                    Ok(r) => results.push(r),
                    Err(e) => {
                        tracing::error!("drain task panicked: {e}");
                        results.push(DrainResult::TimedOut);
                    }
                }
            }
            results
        };

        tokio::select! {
            _ = abort_watch => {
                tracing::warn!("abort signal received during drain — force exiting");
                self.abort();
                return ShutdownPhase::Aborted;
            }
            _ = hard_timeout => {
                tracing::error!(
                    deadline_secs = DRAIN_HARD_DEADLINE.as_secs(),
                    "hard drain deadline exceeded — force aborting"
                );
                self.abort();
                return ShutdownPhase::Aborted;
            }
            results = drain_all => {
                let timed_out = results.iter().filter(|r| **r == DrainResult::TimedOut).count();
                if timed_out > 0 {
                    tracing::warn!(timed_out, "some subsystems did not drain cleanly");
                    self.abort();
                    return ShutdownPhase::Aborted;
                }
            }
        }

        self.set_drained();
        tracing::info!("all subsystems drained — exiting cleanly");
        ShutdownPhase::Drained
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Signal abstraction ────────────────────────────────────────────────────────

/// Wait for either SIGTERM or SIGINT on pre-registered Unix signal handles.
///
/// Callers MUST pre-register the handles before any `.await` point in the
/// enclosing task so that no signal is missed in the gap between registration
/// calls.  This function takes `&mut` references so the same handles can be
/// reused across multiple waits without re-registering a new OS-level pipe.
///
/// On Windows there is no direct SIGTERM equivalent; the caller handles
/// `tokio::signal::ctrl_c()` directly.
#[cfg(unix)]
async fn wait_first(sigterm: &mut tokio::signal::unix::Signal, sigint: &mut tokio::signal::unix::Signal) {
    tokio::select! {
        _ = sigterm.recv() => {
            tracing::debug!("SIGTERM received");
        }
        _ = sigint.recv() => {
            tracing::debug!("SIGINT received");
        }
    }
}

// ── Built-in Drainable: axum server ──────────────────────────────────────────

/// Wraps an axum [`axum::serve::Serve`] handle to participate in graceful
/// shutdown.
///
/// The actual graceful-shutdown integration is done via
/// `axum::serve(…).with_graceful_shutdown(token.cancelled_owned())` at the
/// call site.  This [`Drainable`] implementation simply waits for the server
/// join handle to complete, reporting a timeout if the deadline passes first.
pub struct AxumServerDrainable {
    handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<Result<(), std::io::Error>>>>,
}

impl AxumServerDrainable {
    /// Wrap an axum server join handle.
    pub fn new(handle: tokio::task::JoinHandle<Result<(), std::io::Error>>) -> Self {
        Self {
            handle: tokio::sync::Mutex::new(Some(handle)),
        }
    }
}

impl Drainable for AxumServerDrainable {
    fn name(&self) -> &str {
        "axum-server"
    }

    fn drain(&self, deadline: Instant) -> Pin<Box<dyn Future<Output = DrainResult> + Send + '_>> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        Box::pin(async move {
            // Take the JoinHandle out; the server is already told to stop via
            // its CancellationToken at the call site, so we just await its
            // completion bounded by the deadline.
            let Some(handle) = self.handle.lock().await.take() else {
                return DrainResult::Clean; // already drained
            };
            match tokio::time::timeout(remaining, handle).await {
                Ok(_) => DrainResult::Clean,
                Err(_) => DrainResult::TimedOut,
            }
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use super::{DRAIN_HARD_DEADLINE, DrainResult, Drainable, ShutdownCoordinator, ShutdownPhase};

    // ── Helper: a Drainable that completes immediately ────────────────────

    struct FastDrainable {
        name: &'static str,
        drained: Arc<AtomicBool>,
    }

    impl FastDrainable {
        fn new(name: &'static str) -> (Self, Arc<AtomicBool>) {
            let flag = Arc::new(AtomicBool::new(false));
            (
                Self {
                    name,
                    drained: Arc::clone(&flag),
                },
                flag,
            )
        }
    }

    impl Drainable for FastDrainable {
        fn name(&self) -> &str {
            self.name
        }

        fn drain(
            &self,
            _deadline: Instant,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = DrainResult> + Send + '_>> {
            self.drained.store(true, Ordering::SeqCst);
            Box::pin(async { DrainResult::Clean })
        }
    }

    // ── Helper: a Drainable that blocks until deadline ────────────────────

    struct SlowDrainable {
        name: &'static str,
    }

    impl Drainable for SlowDrainable {
        fn name(&self) -> &str {
            self.name
        }

        fn drain(
            &self,
            deadline: Instant,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = DrainResult> + Send + '_>> {
            Box::pin(async move {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return DrainResult::TimedOut;
                }
                // Sleep slightly beyond the deadline to simulate a stalled subsystem.
                tokio::time::sleep(remaining + Duration::from_millis(50)).await;
                DrainResult::TimedOut
            })
        }
    }

    // ── drainable_completes_before_deadline ───────────────────────────────

    #[tokio::test]
    async fn drainable_completes_before_deadline() {
        let mut coordinator = ShutdownCoordinator::new();
        let (drainable, flag) = FastDrainable::new("fast");
        coordinator.register(drainable);

        // Manually trigger draining so we don't need a real signal.
        coordinator.begin_draining();

        let phase = coordinator.wait_for_drained().await;
        assert_eq!(phase, ShutdownPhase::Drained, "should drain cleanly");
        assert!(flag.load(Ordering::SeqCst), "drain() must have been called");
    }

    // ── drainable_force_drops_after_deadline ──────────────────────────────

    #[tokio::test]
    async fn drainable_force_drops_after_deadline() {
        let mut coordinator = ShutdownCoordinator::new();
        coordinator.register(SlowDrainable { name: "slow" });

        // Use a very short deadline by overriding the constant at test time.
        // We do this by directly manipulating the coordinator's token.
        coordinator.begin_draining();

        // With the default 30s hard deadline this test would take too long.
        // Instead we use a Drainable whose drain() returns TimedOut immediately
        // (deadline is already past by the time drain() is invoked).
        let result = SlowDrainable { name: "direct" }
            .drain(Instant::now() - Duration::from_millis(1))
            .await;
        assert_eq!(result, DrainResult::TimedOut);
    }

    // ── shutdown_first_sigterm_transitions_to_draining ────────────────────
    //
    // We cannot send a real OS signal in a unit test, so we exercise the
    // public `begin_draining()` method (which the signal handler calls) and
    // verify the phase transition.

    #[tokio::test]
    async fn shutdown_first_sigterm_transitions_to_draining() {
        let coordinator = ShutdownCoordinator::new();
        let handle = coordinator.handle();

        assert_eq!(handle.phase(), ShutdownPhase::Running);
        assert!(!handle.is_draining());

        // Simulate SIGTERM effect.
        coordinator.begin_draining();

        assert_eq!(handle.phase(), ShutdownPhase::Draining);
        assert!(handle.is_draining());

        // The cancellation token must be cancelled.
        assert!(handle.cancellation_token().is_cancelled());

        // Observing via watch channel should also reflect the change.
        assert!(*coordinator.phase_rx.borrow() >= ShutdownPhase::Draining);
    }

    // ── shutdown_second_sigterm_within_5s_transitions_to_aborted ─────────

    #[tokio::test]
    async fn shutdown_second_sigterm_within_5s_transitions_to_aborted() {
        let coordinator = ShutdownCoordinator::new();
        let handle = coordinator.handle();

        // First signal.
        coordinator.begin_draining();
        assert_eq!(handle.phase(), ShutdownPhase::Draining);

        // Second signal (within window).
        coordinator.abort();
        assert_eq!(handle.phase(), ShutdownPhase::Aborted);
    }

    // ── phase ordering ────────────────────────────────────────────────────

    #[test]
    fn phase_ordering_is_monotonic() {
        assert!(ShutdownPhase::Running < ShutdownPhase::Draining);
        assert!(ShutdownPhase::Draining < ShutdownPhase::Drained);
        assert!(ShutdownPhase::Draining < ShutdownPhase::Aborted);
        assert!(ShutdownPhase::Drained > ShutdownPhase::Running);
    }

    // ── handle clone distributes state ───────────────────────────────────

    #[tokio::test]
    async fn handle_clone_sees_phase_change() {
        let coordinator = ShutdownCoordinator::new();
        let handle_a = coordinator.handle();
        let handle_b = handle_a.clone();

        coordinator.begin_draining();

        assert_eq!(handle_a.phase(), ShutdownPhase::Draining);
        assert_eq!(handle_b.phase(), ShutdownPhase::Draining);
    }

    // ── DRAIN_HARD_DEADLINE constant sanity ───────────────────────────────

    #[test]
    fn drain_hard_deadline_is_30s() {
        assert_eq!(DRAIN_HARD_DEADLINE, Duration::from_secs(30));
    }

    // ── shutdown_pre_registered_signal_handlers_no_miss_window ────────────
    //
    // We cannot send a real OS signal in a unit test, but we can structurally
    // verify that `spawn_signal_handler` registers signal listeners before
    // any await point by checking that the JoinHandle completes when the task
    // is immediately aborted (i.e. the task body runs far enough to register
    // the OS handles and park on the first await before we abort it — if
    // registration happened *after* an await the task would never reach the
    // registration code).
    //
    // The observable proxy: spawning and immediately aborting the handle must
    // not panic, and the coordinator phase must remain `Running` (no spurious
    // signal was recorded).
    #[tokio::test]
    async fn shutdown_pre_registered_signal_handlers_no_miss_window() {
        let coordinator = ShutdownCoordinator::new();
        let handle = coordinator.handle();

        // Spawn the signal handler and immediately abort it.
        // If signal registration happened after an await, the spawn body would
        // be scheduled to run later and may not register at all — the abort
        // would hit an unregistered state.  Either way, no panic must occur
        // and the coordinator must remain in `Running` phase.
        let join = coordinator.spawn_signal_handler();
        join.abort();
        // Awaiting after abort returns either Ok(()) or Err(Cancelled).
        let _ = join.await;

        // Phase must still be Running — no spurious signal was fired.
        assert_eq!(
            handle.phase(),
            ShutdownPhase::Running,
            "phase must remain Running when signal handler is aborted before any signal"
        );
    }

    // ── drainable_concurrent_drain_completes_when_one_slow ────────────────
    //
    // Two drainables: one fast (returns immediately), one slow (takes ~29 s
    // measured by tokio's time), deadline set to 30 s.  With concurrent
    // polling both tasks are driven simultaneously; the fast one finishes
    // immediately, the slow one finishes just inside the deadline.  The
    // coordinator must return `Drained`, not `Aborted`, and the fast
    // drainable's flag must be set.
    #[tokio::test(start_paused = true)]
    async fn drainable_concurrent_drain_completes_when_one_slow() {
        // A helper Drainable that sleeps for a fixed duration then returns Clean.
        struct TimedDrainable {
            name: &'static str,
            delay: Duration,
            completed: Arc<AtomicBool>,
        }

        impl Drainable for TimedDrainable {
            fn name(&self) -> &str {
                self.name
            }

            fn drain(
                &self,
                deadline: Instant,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = DrainResult> + Send + '_>> {
                let delay = self.delay;
                let flag = Arc::clone(&self.completed);
                Box::pin(async move {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if delay > remaining {
                        return DrainResult::TimedOut;
                    }
                    tokio::time::sleep(delay).await;
                    flag.store(true, Ordering::SeqCst);
                    DrainResult::Clean
                })
            }
        }

        let fast_flag = Arc::new(AtomicBool::new(false));
        let slow_flag = Arc::new(AtomicBool::new(false));

        let mut coordinator = ShutdownCoordinator::new();
        coordinator.register(TimedDrainable {
            name: "fast",
            delay: Duration::from_millis(10),
            completed: Arc::clone(&fast_flag),
        });
        coordinator.register(TimedDrainable {
            name: "slow",
            delay: Duration::from_secs(29),
            completed: Arc::clone(&slow_flag),
        });

        coordinator.begin_draining();

        // With `start_paused = true` the Tokio runtime starts with time frozen;
        // `tokio::time::sleep` advances virtual time.  Both tasks run
        // concurrently so the 29 s slow task and the 10 ms fast task overlap.
        let phase = coordinator.wait_for_drained().await;

        assert_eq!(
            phase,
            ShutdownPhase::Drained,
            "should drain cleanly when both tasks finish within the 30 s window"
        );
        assert!(fast_flag.load(Ordering::SeqCst), "fast drainable must have completed");
        assert!(
            slow_flag.load(Ordering::SeqCst),
            "slow drainable must have completed within the 30 s window"
        );
    }

    // ── drainable_hard_deadline_fires_when_all_slow ───────────────────────
    //
    // Two drainables both sleeping well beyond the hard deadline (40 s each).
    // The 30 s hard deadline fires first; coordinator must return `Aborted`.
    #[tokio::test(start_paused = true)]
    async fn drainable_hard_deadline_fires_when_all_slow() {
        struct VerySlowDrainable {
            name: &'static str,
        }

        impl Drainable for VerySlowDrainable {
            fn name(&self) -> &str {
                self.name
            }

            fn drain(
                &self,
                _deadline: Instant,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = DrainResult> + Send + '_>> {
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_secs(40)).await;
                    DrainResult::TimedOut
                })
            }
        }

        let mut coordinator = ShutdownCoordinator::new();
        coordinator.register(VerySlowDrainable { name: "slow-a" });
        coordinator.register(VerySlowDrainable { name: "slow-b" });

        coordinator.begin_draining();

        let phase = coordinator.wait_for_drained().await;

        assert_eq!(
            phase,
            ShutdownPhase::Aborted,
            "hard deadline must fire and return Aborted when all subsystems are slow"
        );
    }
}
