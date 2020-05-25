use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::Future;
use futures::task::{Context, Poll};

/// An adaptor between callbacks and futures.
///
/// Allows wrapping asynchronous API with callbacks into futures.
/// Calls loader upon first `Future::poll` call; stores result and wakes upon getting callback.
pub struct CallbackFuture<T> {
    loader: Option<Box<dyn FnOnce(Box<dyn FnOnce(T) + Send + 'static>) + Send + 'static>>,
    result: Arc<Mutex<Option<T>>>,
}

impl<T> CallbackFuture<T> {
    /// Creates a new CallbackFuture
    ///
    /// # Examples
    /// ```
    /// use callback_future::CallbackFuture;
    /// use futures::executor::block_on;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let future = CallbackFuture::new(|complete| {
    ///     // make call with callback here, call `complete` upon callback reception, e.g.:
    ///     thread::spawn(move || {
    ///         complete("Test");
    ///     });
    /// });
    /// assert_eq!(block_on(future), "Test");
    /// ```
    pub fn new(loader: impl FnOnce(Box<dyn FnOnce(T) + Send + 'static>) + Send + 'static)
               -> CallbackFuture<T> {
        CallbackFuture {
            loader: Some(Box::new(loader)),
            result: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a ready CallbackFuture
    ///
    /// # Examples
    /// ```
    /// use callback_future::CallbackFuture;
    /// use futures::executor::block_on;
    ///
    /// assert_eq!(block_on(CallbackFuture::ready("Test")), "Test");
    /// ```
    pub fn ready(value: T) -> CallbackFuture<T> {
        CallbackFuture {
            loader: None,
            result: Arc::new(Mutex::new(Some(value))),
        }
    }
}

impl<T: Send + 'static> Future for CallbackFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let self_mut = self.get_mut();
        match self_mut.loader.take() {
            // in case loader is still present, loader was not yet invoked: invoke it
            Some(loader) => {
                let waker = cx.waker().clone();
                let result = self_mut.result.clone();
                loader(Box::new(move |value| {
                    *result.lock().unwrap() = Some(value);
                    waker.wake();
                }));
                Poll::Pending
            }
            // in case loader was moved-out: either result is already ready,
            // or we haven't yet received callback
            None => {
                match self_mut.result.lock().unwrap().take() {
                    Some(value) => Poll::Ready(value),
                    None => Poll::Pending, // we haven't received callback yet
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use futures::{executor::block_on, join};

    use crate::CallbackFuture;

    #[test]
    fn test_complete_async() {
        let fu = CallbackFuture::new(move |complete| {
            thread::spawn(move || { complete(42); });
        });

        assert_eq!(block_on(fu), 42);
    }

    #[test]
    fn test_complete_sync() {
        let fu = CallbackFuture::new(move |complete| {
            complete(42);
        });

        assert_eq!(block_on(fu), 42);
    }

    #[test]
    fn test_ready() {
        let fu = CallbackFuture::ready(42);

        assert_eq!(block_on(fu), 42);
    }

    #[test]
    fn test_join() {
        let all = async {
            let fu1 = CallbackFuture::new(move |complete| {
                complete("Hello");
            });

            let fu2 = CallbackFuture::ready(", ");

            let fu3 = CallbackFuture::new(move |complete| {
                thread::spawn(move || { complete("world!"); });
            });

            let (r1, r2, r3) = join!(fu1, fu2, fu3);
            [r1, r2, r3].concat()
        };

        assert_eq!(block_on(all), "Hello, world!");
    }

    #[test]
    fn test_await() {
        let all = async {
            let r1 = CallbackFuture::new(move |complete| {
                thread::sleep(Duration::from_millis(100));
                complete("Hello");
            }).await;

            let r2 = CallbackFuture::ready(", ").await;

            let r3 = CallbackFuture::new(move |complete| {
                thread::spawn(move || { complete("world!"); });
            }).await;

            [r1, r2, r3].concat()
        };

        assert_eq!(block_on(all), "Hello, world!");
    }

    #[test]
    fn test_async_fn() {
        async fn do_async() -> String {
            CallbackFuture::new(move |complete| {
                thread::spawn(move || { complete("Hello, world!".to_string()); });
            }).await
        }

        assert_eq!(block_on(do_async()), "Hello, world!");
    }
}
