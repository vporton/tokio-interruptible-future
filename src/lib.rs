/// Easily interrupt async code in given check points. It's useful to interrupt threads/fibers.
/// TODO: Documentation comments.

use std::{fmt, future::Future};
use std::sync::Arc;
use tokio::sync::broadcast::error::{RecvError, SendError, TryRecvError};
use tokio::sync::Mutex;

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

/// tokio::sync::broadcast::Sender has `Receiver` not cloneable, creating the temptation to
/// clone an `Arc` with `Receiver` inside, what would lead to loss of messages.
///
/// So, this instead (make it a separate library?)
pub fn broadcast<'a, T: Clone>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = tokio::sync::broadcast::channel(capacity);
    let tx = Arc::new(Mutex::new(tx));
    return (
        Sender {
            tx: tx.clone(),
        },
        Receiver {
            tx: tx.clone(),
            rx
        }
    )
}

pub struct Sender<T> {
    tx: Arc<Mutex<tokio::sync::broadcast::Sender<T>>>,
}

pub struct Receiver<T> {
    tx: Arc<Mutex<tokio::sync::broadcast::Sender<T>>>,
    rx: tokio::sync::broadcast::Receiver<T>,
}

impl<T> Sender<T> {
    pub async fn receiver_count(&self) -> usize {
        self.tx.lock().await.receiver_count()
    }
    pub async fn send(&self, value: T) -> Result<usize, SendError<T>> {
        self.tx.lock().await.send(value)
    }
}

impl<T: Clone> Receiver<T> {
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        self.rx.recv().await
    }
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        self.rx.try_recv()
    }
    pub async fn receiver_count(&self) -> usize {
        self.tx.lock().await.receiver_count()
    }
    pub async fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: self.tx.lock().await.subscribe(),
        }
    }
}

/// You usually use `interruptible` or `interruptible_sendable` instead.
pub async fn interruptible_straight<T, E: From<InterruptError>>(
    rx: Receiver<()>,
    f: impl Future<Output=Result<T, E>>
) -> Result<T, E>
{
    let mut rx = rx;
    tokio::select!{
        r = f => r,
        _ = async { // shorten lock lifetime
            let _ = rx.recv().await;
        } => Err(InterruptError::new().into()),
    }
}

pub async fn interruptible<T, E: From<InterruptError>>(
    rx: Receiver<()>,
    f: Arc<Mutex<dyn Future<Output=Result<T, E>> + Unpin>>
) -> Result<T, E>
{
    let f = f.clone();
    let mut f = f.lock().await;
    let f = Box::pin(&mut *f);
    interruptible_straight(rx, f).await
}

pub async fn interruptible_sendable<T, E: From<InterruptError>>(
    rx: Receiver<()>,
    f: Arc<Mutex<dyn Future<Output=Result<T, E>> + Send + Unpin>>
) -> Result<T, E>
{
    let f = f.clone();
    let mut f = f.lock().await;
    let f = Box::pin(&mut *f);
    interruptible_straight(rx, f).await
}

/// TODO: More tests.
#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::sync::Arc;
    use futures::executor::block_on;
    use tokio::sync::Mutex;

    use crate::{InterruptError, interruptible, interruptible_sendable, broadcast};

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
            let (_tx, rx) = broadcast::<()>(1);

            interruptible(rx, Arc::new(Mutex::new(Box::pin(async move {
                Ok(123)
            })))).await
        }
        pub async fn h(self) -> Result<u8, MyError> {
            let (_tx, rx) = broadcast::<()>(1);

            interruptible(rx, Arc::new(Mutex::new(Box::pin(async move {
                Err(AnotherError::new().into())
            })))).await
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
        let (_tx, rx) = broadcast::<()>(1);

        // Check that `interruptible_sendable(...)` is a `Send` future.
        let _: &(dyn Future<Output = Result<i32, InterruptError>> + Send) = &interruptible_sendable(rx, Arc::new(Mutex::new(Box::pin(async move {
            Ok(123)
        }))));
    }
}
