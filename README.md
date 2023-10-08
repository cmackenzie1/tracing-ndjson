# tracing-ndjson

A simple library for tracing in new-line delimited JSON format. This library is meant to be used with [tracing](https://github.com/tokio-rs/tracing) as an alternative to the `tracing_subscriber::fmt::json` formatter.

## Features

- Configurable field names for `target`, `message`, `level`, and `timestamp`.
- Configurable timestamp formats such as RFC3339, UNIX timestamp, or any custom chrono format.
- Captures all span attributes and event fields in the root of the JSON object.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
tracing = "0.1"
tracing-ndjson = "0.1"
```

```rust
use tracing_subscriber::prelude::*;

fn main() {
    tracing_subscriber::registry()
        .with(tracing_ndjson::builder().layer())
        .init();

    tracing::info!(life = 42, "Hello, world!");
    // {"level":"info","timestamp":"2023-10-08T03:30:52Z","target":"default","message":"Hello, world!"}

    let span = tracing::info_span!("hello", "request.uri" = "https://example.com");
    span.in_scope(|| {
        tracing::info!("Hello, world!");
        // {"level":"info","timestamp":"2023-10-08T03:34:33Z","target":"defaults","message":"Hello, world!","request.uri":"https://example.com"}
    });
}
```

### Examples

See the [examples](./examples) directory for more examples.

## License

Licensed under MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)
