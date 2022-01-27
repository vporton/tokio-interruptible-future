#![feature(async_closure)]
#![feature(explicit_generic_args_with_impl_trait)]

/// Easily interrupt async code in given check points. It's useful to interrupt threads/fibers.
/// TODO: Documentation comments.

use std::{fmt, future::Future};

use async_trait::async_trait;
use tokio::sync::Notify;

#[derive(Debug, PartialEq, Eq)]
struct InterruptError { }

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

#[async_trait]
trait Interruptible {
    fn interrupt_notifier(&self) -> &Notify;

    fn interrupt(&self) {
        self.interrupt_notifier().notify_one();
    }

    async fn check_for_interrupt<E: From<InterruptError>>(&self) -> Result<(), E> {
        self.interrupt_notifier().notified().await;
        Err(InterruptError::new().into())
    }

    async fn interruptible<'a, T, E: From<InterruptError>>(&self, f: impl Future<Output = Result<T, E>> + Send + 'a)
        -> Result<T, E>
    {
        tokio::select!{
            r = f => r,
            Err(e) = self.check_for_interrupt() => Err(E::from(e)),
        }
    }
}

/// TODO: More tests.
#[cfg(test)]
mod tests {
    use tokio::sync::Notify;
    use futures::executor::block_on;

    use crate::{Interruptible, InterruptError};

    #[derive(Debug, PartialEq, Eq)]
    enum MyError {
        Interrupted(InterruptError),
    }
    impl From<InterruptError> for MyError {
        fn from(value: InterruptError) -> Self {
            Self::Interrupted(value)
        }
    }
    struct Test {
        interrupt_notifier: Notify,
    }
    impl Interruptible for Test {
        fn interrupt_notifier(&self) -> &Notify {
            &self.interrupt_notifier
        }
    }
    impl Test {
        pub fn new() -> Self {
            Self {
                interrupt_notifier: Notify::new()
            }
        }
        pub async fn f(&self) -> Result<(), MyError> {
            self.interruptible/*::<(), MyError>*/(async {
                loop {
                    self.interrupt(); // In real code called from another fiber or another thread.
                    self.check_for_interrupt::<MyError>().await?;
                }
            }).await
        }
        pub async fn g(&self) -> Result<u8, MyError> {
            self.interruptible::<u8, MyError>(async {
                Ok(123)
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
    }

    #[test]
    fn not_interrupted() {
        let test = Test::new();
        block_on(async {
            assert_eq!(test.g().await, Ok(123));
        });
    }
}
