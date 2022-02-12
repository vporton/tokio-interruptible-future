// #![feature(async_closure)]
// #![feature(explicit_generic_args_with_impl_trait)]

/// Easily interrupt async code in given check points. It's useful to interrupt threads/fibers.
/// TODO: Documentation comments.

use std::{fmt, future::Future, sync::Arc};

use tokio::sync::Notify;

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

pub async fn interruptible<'a, T, E: From<InterruptError>>(
    notifier: Arc<Notify>,
    f: impl Future<Output = Result<T, E>> + 'a
) -> Result<T, E>
{
    tokio::select!{
        r = f => r,
        _ = notifier.notified() => Err(InterruptError::new().into()),
    }
}

pub async fn check_for_interrupt<E: From<InterruptError>>(notifier: Arc<Notify>) -> Result<(), E> {
    // interruptible(notifier, ready(Ok::<(), E>(()))).await // `E` cannot be sent between threads safely, if no `Send`
    interruptible(notifier, async move { Ok(()) }).await // works without Send but requires compiler directives
}

/// TODO: More tests.
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tokio::sync::Notify;
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
            let interrupt_notifier = Arc::new(Notify::new()); // Arc::new(Notify::new());
            interrupt_notifier.notify_one(); // In real code called from another fiber or another thread.

            interruptible(interrupt_notifier.clone(), async move {
                loop {
                    check_for_interrupt::<MyError>(interrupt_notifier.clone()).await?;
                }
            }).await
        }
        pub async fn f2(self) -> Result<(), MyError> {
            let interrupt_notifier = Arc::new(Notify::new()); // Arc::new(Notify::new());

            interruptible(interrupt_notifier.clone(), async move {
                loop {
                    interrupt_notifier.clone().notify_one(); // In real code called from another fiber or another thread.
                    check_for_interrupt::<MyError>(interrupt_notifier.clone()).await?;
                }
            }).await
        }
        pub async fn g(self) -> Result<u8, MyError> {
            let interrupt_notifier = Arc::new(Notify::new());

            interruptible(interrupt_notifier.clone(), async move {
                Ok(123)
            }).await
        }
        pub async fn h(self) -> Result<u8, MyError> {
            let interrupt_notifier = Arc::new(Notify::new());

            interruptible(interrupt_notifier.clone(), async move {
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

    #[test]
    fn not_interrupted() {
    }
}
