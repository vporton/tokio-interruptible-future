/// Easily interrupt async code in given check points. It's useful to interrupt threads/fibers.
/// TODO: Documentation comments.

use std::{fmt, future::Future};
use async_channel::Receiver;

#[derive(Debug, PartialEq, Eq)]
pub struct InterruptError { }

impl InterruptError {
    pub fn new() -> Self {
        Self { }
    }
}

impl fmt::Display for InterruptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Async fiber interrupted.")
    }
}

/// You usually use `interruptible` or `interruptible_sendable` instead.
pub async fn interruptible_straight<T, E: From<InterruptError>>(
    rx: Receiver<()>,
    f: impl Future<Output=Result<T, E>>
) -> Result<T, E>
{
    tokio::select!{
        r = f => r,
        _ = async { // shorten lock lifetime
            let _ = rx.recv().await;
        } => Err(InterruptError::new().into()),
    }
}

pub async fn interruptible<T, E: From<InterruptError>>(
    rx: Receiver<()>,
    f: impl Future<Output=Result<T, E>> + Unpin
) -> Result<T, E>
{
    interruptible_straight(rx, f).await
}

pub async fn interruptible_sendable<T, E: From<InterruptError>>(
    rx: Receiver<()>,
    f: impl Future<Output=Result<T, E>> + Send + Unpin
) -> Result<T, E>
{
    interruptible_straight(rx, f).await
}

/// TODO: More tests.
#[cfg(test)]
mod tests {
    use std::future::Future;
    use async_channel::bounded;
    use futures::executor::block_on;

    use crate::{InterruptError, interruptible, interruptible_sendable};

    #[derive(Debug, PartialEq, Eq)]
    struct AnotherError { }
    impl AnotherError {
        pub fn new() -> Self {
            return Self { }
        }
    }
    #[derive(Debug, PartialEq, Eq)]
    enum MyError {
        Interrupted(InterruptError),
        Another(AnotherError)
    }
    impl From<InterruptError> for MyError {
        fn from(value: InterruptError) -> Self {
            Self::Interrupted(value)
        }
    }
    impl From<AnotherError> for MyError {
        fn from(value: AnotherError) -> Self {
            Self::Another(value)
        }
    }
    struct Test {
    }
    impl Test {
        pub fn new() -> Self {
            Self {
            }
        }
        pub async fn g(self) -> Result<u8, MyError> {
            let (_tx, rx) = bounded(1);

            interruptible(rx, Box::pin(async move {
                Ok(123)
            })).await
        }
        pub async fn h(self) -> Result<u8, MyError> {
            let (_tx, rx) = bounded(1);

            interruptible(rx, Box::pin(async move {
                Err(AnotherError::new().into())
            })).await
        }
    }

    #[test]
    fn interrupted() {
        let test = Test::new();
        block_on(async {
            assert_eq!(test.g().await, Ok(123));
        });
        let test = Test::new();
        block_on(async {
            assert_eq!(test.h().await, Err(AnotherError::new().into()));
        });
    }

    #[test]
    fn check_interruptible_sendable() {
        let (_tx, rx) = bounded(1);

        // Check that `interruptible_sendable(...)` is a `Send` future.
        let _: &(dyn Future<Output = Result<i32, InterruptError>> + Send) = &interruptible_sendable(rx, Box::pin(async move {
            Ok(123)
        }));
    }
}
