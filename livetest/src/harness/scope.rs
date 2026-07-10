//! RAII-ish cleanup scope for in-run teardown.
//!
//! Async destructors don't exist in stable Rust, so cleanup is an explicit
//! `scope.cleanup().await` run in a `finally` position (after the fixtures,
//! whether they succeeded or not). Every create registers its teardown
//! immediately after success, so teardown fires exactly once per acquired
//! resource even if a later step fails -- mirroring the AGENTS.md
//! paired-API double-invocation rule.
//!
//! A `Drop` guard catches the programming error of dropping a scope without
//! calling `cleanup`: it warns loudly rather than silently leaking. It cannot
//! itself run the async teardowns; the pre-flight sweep of the next run is the
//! backstop for anything that slips through.

use std::future::Future;
use std::pin::Pin;

use voip_ms::Client;

type TeardownFuture<'a> = Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a>>;
type TeardownFn = Box<dyn for<'a> FnOnce(&'a Client) -> TeardownFuture<'a> + Send>;

/// Deferred teardowns, run in reverse (LIFO) order by [`Scope::cleanup`].
#[derive(Default)]
pub struct Scope {
    teardowns: Vec<(String, TeardownFn)>,
    cleaned: bool,
}

impl Scope {
    pub fn new() -> Self {
        Scope::default()
    }

    /// Register a teardown to run at cleanup. `label` names the resource for
    /// logging. Call this immediately after a successful create.
    pub fn defer<F>(&mut self, label: impl Into<String>, teardown: F)
    where
        F: for<'a> FnOnce(&'a Client) -> TeardownFuture<'a> + Send + 'static,
    {
        self.teardowns.push((label.into(), Box::new(teardown)));
    }

    /// Run every deferred teardown in LIFO order, best-effort. A failing
    /// teardown is logged and does not stop the others. Returns the labels of
    /// teardowns that failed so the caller can surface them.
    pub async fn cleanup(&mut self, client: &Client) -> Vec<String> {
        self.cleaned = true;
        let mut failures = Vec::new();
        while let Some((label, teardown)) = self.teardowns.pop() {
            match teardown(client).await {
                Ok(()) => println!("[cleanup] {label}"),
                Err(error) => {
                    eprintln!("[cleanup-fail] {label}: {error}");
                    failures.push(label);
                }
            }
        }

        failures
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if !self.cleaned && !self.teardowns.is_empty() {
            eprintln!(
                "[BUG] Scope dropped with {} un-run teardown(s); the next run's \
                 pre-flight sweep will reclaim them. Labels: {}",
                self.teardowns.len(),
                self.teardowns
                    .iter()
                    .map(|(l, _)| l.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}
