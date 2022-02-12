/// Easily interrupt async code in given check points. It's useful to interrupt threads/fibers.
/// TODO: Documentation comments.

use std::{fmt, future::Future};
use async_channel::Receiver;

#[derive(Debug, PartialEq, Eq)]
pub struct InterruptError { }

impl InterruptError {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { }
    }
}

impl fmt::Display for InterruptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Async fiber interrupted.")
    }
}

pub async fn interruptible<T, E: From<InterruptError>>(
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

pub async fn check_for_interrupt<E: From<InterruptError>>(
    rx: Receiver<()>,
) -> Result<(), E> {
    interruptible(rx, async move { Ok(()) }).await
}

/// TODO: More tests.
#[cfg(test)]
mod tests {
    use async_channel::bounded;
    use futures::executor::block_on;

    use crate::{InterruptError, check_for_interrupt, interruptible};

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
        pub async fn f(self) -> Result<(), MyError> {
            let (tx, rx) = bounded(1);
            tx.send(()).await.unwrap(); // In real code called from another fiber or another thread.

            interruptible(rx.clone(), async move {
                loop {
                    check_for_interrupt::<MyError>(rx.clone()).await?;
                }
            }).await
        }
        pub async fn f2(self) -> Result<(), MyError> {
            let (tx, rx) = bounded(1);

            interruptible(rx.clone(), async move {
                loop {
                    tx.send(()).await.unwrap(); // In real code called from another fiber or another thread.
                    check_for_interrupt::<MyError>(rx.clone()).await?;
                }
            }).await
        }
        pub async fn g(self) -> Result<u8, MyError> {
            let (_tx, rx) = bounded::<()>(1);

            interruptible(rx, async move {
                Ok(123)
            }).await
        }
        pub async fn h(self) -> Result<u8, MyError> {
            let (_tx, rx) = bounded::<()>(1);

            interruptible(rx, async move {
                Err(AnotherError::new().into())
            }).await
        }
    }

    #[test]
    fn interrupted() {
        let test = Test::new();
        block_on(async {
            match test.f().await {
                Err(MyError::Interrupted(_)) => {},
                _ => assert!(false),
            }
            });
        let test = Test::new();
        block_on(async {
            match test.f2().await {
                Err(MyError::Interrupted(_)) => {},
                _ => assert!(false),
            }
        });
        let test = Test::new();
        block_on(async {
            assert_eq!(test.g().await, Ok(123));
        });
        let test = Test::new();
        block_on(async {
            assert_eq!(test.h().await, Err(AnotherError::new().into()));
        });
    }
}
