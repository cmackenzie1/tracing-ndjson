# tracing-ndjson

[![Crates.io](https://img.shields.io/crates/v/tracing-ndjson)](https://crates.io/crates/tracing-ndjson)
[![Rust](https://github.com/cmackenzie1/tracing-ndjson/actions/workflows/rust.yml/badge.svg)](https://github.com/cmackenzie1/tracing-ndjson/actions/workflows/rust.yml)
[![docs.rs](https://img.shields.io/docsrs/tracing-ndjson)](https://docs.rs/tracing-ndjson/latest/tracing_ndjson)

A simple library for tracing in new-line delimited JSON format. This library is meant to be used with [tracing](https://github.com/tokio-rs/tracing) as an alternative to the `tracing_subscriber::fmt::json` formatter.

The goal of this crate is to provide a flattend JSON event, comprising of fields from the span attributes and event fields, with customizeable field names and timestamp formats.

## Features

- Configurable field names for `target`, `message`, `level`, and `timestamp`.
- Configurable timestamp formats
  - RFC3339 (`2023-10-08T03:30:52Z`),
  - RFC339Nanos (`2023-10-08T03:30:52.123456789Z`)
  - Unix timestamp (`1672535452`)
  - UnixMills (`1672535452123`)
- Captures all span attributes and event fields in the root of the JSON object. Collisions will result in overwriting the existing field.

## Limitations

- When flattening span attributes and event fields, the library will overwrite any existing fields with the same name, including the built-in fields such as `target`, `message`, `level`, `timestamp`, `file`, and `line`.
- Non-determistic ordering of fields in the JSON object. ([JSON objects are unordered](https://www.json.org/json-en.html))
- Currently only logs to stdout. (PRs welcome!)

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
tracing = "0.1"
tracing-ndjson = "0.2"
```

```rust
use tracing_subscriber::prelude::*;

fn main() {
    let subscriber = tracing_subscriber::registry().with(tracing_ndjson::layer());

    tracing::subscriber::set_global_default(subscriber).unwrap();

    tracing::info!(life = 42, "Hello, world!");
    // {"level":"info","target":"default","life":42,"timestamp":"2023-10-20T21:17:49Z","message":"Hello, world!"}

    let span = tracing::info_span!("hello", "request.uri" = "https://example.com");
    span.in_scope(|| {
        tracing::info!("Hello, world!");
        // {"message":"Hello, world!","request.uri":"https://example.com","level":"info","target":"default","timestamp":"2023-10-20T21:17:49Z"}
    });
}
```

### Examples

See the [examples](./examples) directory for more examples.

## License

Licensed under [MIT license](./LICENSE)
