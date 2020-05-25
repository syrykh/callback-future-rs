# callback-future-rs

A simple adaptor between callbacks and futures.

## Example

Dependencies:

```toml
[dependencies]
callback-future="0.1"
```

Code:

```rust
use callback_future::CallbackFuture;
use futures::executor::block_on;
use std::thread;
use std::time::Duration;

async fn get_result() -> String {
    CallbackFuture::new(
        |complete| {
            // simulate async callback from another thread
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(1));
                complete("Hello, world!".to_string());
            });
        }
    ).await
}

fn main() {
    assert_eq!(block_on(get_result()), "Hello, world!");
}
```

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
* MIT license ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option.
