//! A small profiler for the benchmark: named parts of a frame are timed
//! while it is on (and cost next to nothing while it is off).
//!
//! ```ignore
//! let _t = profile::span("simulate/physics");
//! ```

use std::cell::RefCell;
use std::time::{Duration, Instant};

thread_local! {
    static SPANS: RefCell<Option<Vec<(&'static str, Duration)>>> = const { RefCell::new(None) };
}

/// Start (and clear) or stop timing.
pub fn enable(on: bool) {
    SPANS.with(|s| *s.borrow_mut() = on.then(Vec::new));
}

fn enabled() -> bool {
    SPANS.with(|s| s.borrow().is_some())
}

/// Time from now until the returned guard is dropped, under `name`.
#[must_use]
pub fn span(name: &'static str) -> Span {
    Span { name, start: enabled().then(Instant::now) }
}

pub struct Span {
    name: &'static str,
    start: Option<Instant>,
}

impl Drop for Span {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            add(self.name, start.elapsed());
        }
    }
}

fn add(name: &'static str, d: Duration) {
    SPANS.with(|s| {
        if let Some(spans) = s.borrow_mut().as_mut() {
            match spans.iter_mut().find(|(n, _)| *n == name) {
                Some((_, total)) => *total += d,
                None => spans.push((name, d)),
            }
        }
    });
}

/// The time spent under each name since the last call (longest first),
/// and start again.
pub fn take() -> Vec<(&'static str, Duration)> {
    let mut out = SPANS.with(|s| s.borrow_mut().as_mut().map(std::mem::take).unwrap_or_default());
    out.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_add_up_only_while_on() {
        drop(span("off"));
        enable(true);
        for _ in 0..3 {
            let _t = span("a");
            std::thread::sleep(Duration::from_millis(2));
        }
        drop(span("b"));
        let spans = take();
        assert_eq!(spans[0].0, "a");
        assert!(spans[0].1 >= Duration::from_millis(6));
        assert!(spans.iter().any(|(n, _)| *n == "b"));
        assert!(!spans.iter().any(|(n, _)| *n == "off"));
        assert!(take().is_empty(), "cleared");
        enable(false);
        drop(span("late"));
        assert!(take().is_empty());
    }
}
