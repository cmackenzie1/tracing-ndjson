# tracing-ndjson

[![Crates.io](https://img.shields.io/crates/v/tracing-ndjson)](https://crates.io/crates/tracing-ndjson)
[![Rust](https://github.com/cmackenzie1/tracing-ndjson/actions/workflows/rust.yml/badge.svg)](https://github.com/cmackenzie1/tracing-ndjson/actions/workflows/rust.yml)
[![docs.rs](https://img.shields.io/docsrs/tracing-ndjson)](https://docs.rs/tracing-ndjson/latest/tracing_ndjson)


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
    // {"level":"info","timestamp":"2023-10-10T20:35:26Z","target":"defaults","message":"Hello, world!","life":42}

    let span = tracing::info_span!("hello", "request.uri" = "https://example.com");
    span.in_scope(|| {
        tracing::info!("Hello, world!");
        // {"level":"info","timestamp":"2023-10-10T20:35:26Z","target":"defaults","message":"Hello, world!","request.uri":"https://example.com"}
    });
}
```

### Examples

See the [examples](./examples) directory for more examples.

## License

Licensed under [MIT license](./LICENSE)
