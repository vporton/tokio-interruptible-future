# tokio-interruptible-future

Easily interrupt async code in given check points. It's useful to interrupt threads/fibers.

DISCLAIMER: Not enough tested.

TODO: Add documentation.

```rust
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
```