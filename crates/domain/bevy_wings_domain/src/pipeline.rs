//! Use case pipelines: decorators that add retries, timeouts and logging to any
//! [`UseCase`].
//!
//! ```ignore
//! let load = LoadNotes { repository }
//!     .retry(RetryPolicy::times(3).with_backoff(Duration::from_millis(200)))
//!     .timeout(Duration::from_secs(5))
//!     .logged("load notes", |event| println!("{event:?}"));
//! ```
//!
//! Decorators are use cases themselves, so they nest in any order and plug into DI like the
//! use case they wrap.

use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::error::{DomainError, DomainResult};
use crate::use_case::UseCase;

/// How a [`Retrying`] use case retries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetryPolicy {
    /// Attempts in total (at least 1).
    pub attempts: u32,
    /// Wait before the first retry; doubles after every retry.
    pub backoff: Duration,
    /// Retry every error, not only [retryable](DomainError::is_retryable) ones.
    pub retry_all: bool,
}

impl RetryPolicy {
    /// `attempts` attempts in total without waiting.
    pub fn times(attempts: u32) -> Self {
        Self {
            attempts: attempts.max(1),
            backoff: Duration::ZERO,
            retry_all: false,
        }
    }

    /// Waits `backoff` before the first retry, twice as long before the next and so on.
    pub fn with_backoff(mut self, backoff: Duration) -> Self {
        self.backoff = backoff;
        self
    }

    /// Retries every error.
    pub fn retry_all(mut self) -> Self {
        self.retry_all = true;
        self
    }

    /// Wait before retry number `retry` (1 based).
    pub fn delay(&self, retry: u32) -> Duration {
        self.backoff
            .saturating_mul(1 << retry.saturating_sub(1).min(16))
    }
}

/// Retries a use case on failure. See [`UseCaseExt::retry`].
pub struct Retrying<U> {
    inner: U,
    policy: RetryPolicy,
}

impl<U> UseCase for Retrying<U>
where
    U: UseCase,
    U::Input: Clone,
{
    type Input = U::Input;
    type Output = U::Output;

    fn execute(&self, input: Self::Input) -> DomainResult<Self::Output> {
        let mut attempt = 1;
        loop {
            match self.inner.execute(input.clone()) {
                Ok(v) => return Ok(v),
                Err(e)
                    if attempt < self.policy.attempts
                        && (self.policy.retry_all || e.is_retryable()) =>
                {
                    let delay = self.policy.delay(attempt);
                    if !delay.is_zero() {
                        std::thread::sleep(delay);
                    }
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// Fails a use case that takes too long. See [`UseCaseExt::timeout`].
///
/// The wrapped use case runs on its own thread; when it times out the caller gets
/// [`DomainError::Unavailable`] (retryable) and the late result is dropped.
pub struct Timeout<U> {
    inner: Arc<U>,
    limit: Duration,
}

impl<U> UseCase for Timeout<U>
where
    U: UseCase + 'static,
    U::Input: Send + 'static,
    U::Output: Send + 'static,
{
    type Input = U::Input;
    type Output = U::Output;

    fn execute(&self, input: Self::Input) -> DomainResult<Self::Output> {
        let (tx, rx) = mpsc::channel();
        let inner = self.inner.clone();
        std::thread::spawn(move || {
            let _ = tx.send(inner.execute(input));
        });
        match rx.recv_timeout(self.limit) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => Err(DomainError::Unavailable(format!(
                "timed out after {} ms",
                self.limit.as_millis()
            ))),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err(DomainError::Unexpected("use case panicked".into()))
            }
        }
    }
}

/// What a [`Logged`] use case reports.
#[derive(Debug, Clone, PartialEq)]
pub struct UseCaseEvent {
    /// Name given to [`UseCaseExt::logged`].
    pub name: &'static str,
    /// How long it ran.
    pub elapsed: Duration,
    /// The error, if it failed.
    pub error: Option<DomainError>,
}

/// Reports every run of a use case. See [`UseCaseExt::logged`].
pub struct Logged<U, F> {
    inner: U,
    name: &'static str,
    sink: F,
}

impl<U, F> UseCase for Logged<U, F>
where
    U: UseCase,
    F: Fn(&UseCaseEvent) + Send + Sync,
{
    type Input = U::Input;
    type Output = U::Output;

    fn execute(&self, input: Self::Input) -> DomainResult<Self::Output> {
        let start = Instant::now();
        let result = self.inner.execute(input);
        (self.sink)(&UseCaseEvent {
            name: self.name,
            elapsed: start.elapsed(),
            error: result.as_ref().err().cloned(),
        });
        result
    }
}

/// Maps the output of a use case. See [`UseCaseExt::map`].
pub struct Mapped<U, F> {
    inner: U,
    f: F,
}

impl<U, F, O> UseCase for Mapped<U, F>
where
    U: UseCase,
    F: Fn(U::Output) -> O + Send + Sync,
{
    type Input = U::Input;
    type Output = O;

    fn execute(&self, input: Self::Input) -> DomainResult<O> {
        self.inner.execute(input).map(&self.f)
    }
}

/// Decorators for every use case.
pub trait UseCaseExt: UseCase + Sized {
    /// Retries failures according to `policy`.
    fn retry(self, policy: RetryPolicy) -> Retrying<Self> {
        Retrying {
            inner: self,
            policy,
        }
    }

    /// Fails with [`DomainError::Unavailable`] after `limit`.
    fn timeout(self, limit: Duration) -> Timeout<Self> {
        Timeout {
            inner: Arc::new(self),
            limit,
        }
    }

    /// Calls `sink` after every run with the name, duration and error.
    fn logged<F: Fn(&UseCaseEvent) + Send + Sync>(
        self,
        name: &'static str,
        sink: F,
    ) -> Logged<Self, F> {
        Logged {
            inner: self,
            name,
            sink,
        }
    }

    /// Transforms the output.
    fn map<O, F: Fn(Self::Output) -> O + Send + Sync>(self, f: F) -> Mapped<Self, F> {
        Mapped { inner: self, f }
    }
}

impl<U: UseCase> UseCaseExt for U {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_case::use_case;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Retryable errors are retried until success; others are not.
    #[test]
    fn retries() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = calls.clone();
        let flaky = use_case(move |_: ()| {
            if c.fetch_add(1, Ordering::SeqCst) < 2 {
                Err(DomainError::Unavailable("down".into()))
            } else {
                Ok(5)
            }
        })
        .retry(RetryPolicy::times(3));
        assert_eq!(flaky.execute(()), Ok(5));
        assert_eq!(calls.load(Ordering::SeqCst), 3);

        let calls = Arc::new(AtomicU32::new(0));
        let c = calls.clone();
        let invalid = use_case(move |_: ()| -> DomainResult<i32> {
            c.fetch_add(1, Ordering::SeqCst);
            Err(DomainError::invalid("x", "bad"))
        })
        .retry(RetryPolicy::times(5));
        assert!(invalid.execute(()).is_err());
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "validation errors are final"
        );
        assert_eq!(
            RetryPolicy::times(3)
                .with_backoff(Duration::from_millis(10))
                .delay(3),
            Duration::from_millis(40)
        );
    }

    /// Slow use cases time out with a retryable error.
    #[test]
    fn times_out() {
        let slow = use_case(|_: ()| {
            std::thread::sleep(Duration::from_millis(200));
            Ok(1)
        })
        .timeout(Duration::from_millis(20));
        let err = slow.execute(()).unwrap_err();
        assert!(err.is_retryable());
        let fast = use_case(|x: i32| Ok(x * 2)).timeout(Duration::from_secs(1));
        assert_eq!(fast.execute(4), Ok(8));
    }

    /// Logging reports names, durations and errors; decorators compose.
    #[test]
    fn logs_and_composes() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let l = log.clone();
        let uc = use_case(|x: i32| {
            if x > 0 {
                Ok(x)
            } else {
                Err(DomainError::invalid("x", "neg"))
            }
        })
        .map(|x| x + 1)
        .logged("inc", move |e| {
            l.lock().unwrap().push((e.name, e.error.is_some()))
        });
        assert_eq!(uc.execute(1), Ok(2));
        assert!(uc.execute(-1).is_err());
        assert_eq!(*log.lock().unwrap(), vec![("inc", false), ("inc", true)]);
    }
}
