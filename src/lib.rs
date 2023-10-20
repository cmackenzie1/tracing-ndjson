//! # tracing-ndjson
//!
//! [![Rust](https://github.com/cmackenzie1/tracing-ndjson/actions/workflows/rust.yml/badge.svg)](https://github.com/cmackenzie1/tracing-ndjson/actions/workflows/rust.yml)
//!
//! A simple library for tracing in new-line delimited JSON format. This library is meant to be used with [tracing](https://github.com/tokio-rs/tracing) as an alternative to the `tracing_subscriber::fmt::json` formatter.
//!
//! The goal of this crate is to provide a flattend JSON event, comprising of fields from the span attributes and event fields, with customizeable field names and timestamp formats.
//!
//! ## Features
//!
//! - Configurable field names for `target`, `message`, `level`, and `timestamp`.
//! - Configurable timestamp formats
//!   - RFC3339 (`2023-10-08T03:30:52Z`),
//!   - RFC339Nanos (`2023-10-08T03:30:52.123456789Z`)
//!   - Unix timestamp (`1672535452`)
//!   - UnixMills (`1672535452123`)
//! - Captures all span attributes and event fields in the root of the JSON object. Collisions will result in overwriting the existing field.
//!
//! ## Limitations
//!
//! - When flattening span attributes and event fields, the library will overwrite any existing fields with the same name, including the built-in fields such as `target`, `message`, `level`, `timestamp`, `file`, and `line`.
//! - Non-determistic ordering of fields in the JSON object. ([JSON objects are unordered](https://www.json.org/json-en.html))
//! - Currently only logs to stdout. (PRs welcome!)
//!
//! ## Usage
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! tracing = "0.1"
//! tracing-ndjson = "0.2"
//! ```
//!
//! ```rust
//! use tracing_subscriber::prelude::*;
//!
//! let subscriber = tracing_subscriber::registry().with(tracing_ndjson::layer());
//!
//! tracing::subscriber::set_global_default(subscriber).unwrap();
//!
//! tracing::info!(life = 42, "Hello, world!");
//! // {"level":"info","target":"default","life":42,"timestamp":"2023-10-20T21:17:49Z","message":"Hello, world!"}
//!
//! let span = tracing::info_span!("hello", "request.uri" = "https://example.com");
//! span.in_scope(|| {
//!     tracing::info!("Hello, world!");
//!     // {"message":"Hello, world!","request.uri":"https://example.com","level":"info","target":"default","timestamp":"2023-10-20T21:17:49Z"}
//! });
//! ```
//!
//! ### Examples
//!
//! See the [examples](./examples) directory for more examples.
//!
//! ## License
//!
//! Licensed under [MIT license](./LICENSE)

mod layer;
mod storage;

pub use layer::*;
use tracing_core::Subscriber;
use tracing_subscriber::registry::LookupSpan;

/// A timestamp format for the JSON formatter.
/// This is used to format the timestamp field in the JSON output.
/// The default is RFC3339.
#[derive(Debug, Default)]
pub enum TimestampFormat {
    /// Seconds since UNIX_EPOCH
    Unix,
    /// Milliseconds since UNIX_EPOCH
    UnixMillis,
    /// RFC3339
    #[default]
    Rfc3339,
    /// RFC3339 with nanoseconds
    Rfc3339Nanos,
    /// Custom format string. This should be a valid format string for chrono.
    Custom(String),
}

impl TimestampFormat {
    fn format_string(&self, now: &chrono::DateTime<chrono::Utc>) -> String {
        match self {
            TimestampFormat::Unix => now.timestamp().to_string(),
            TimestampFormat::UnixMillis => now.timestamp_millis().to_string(),
            TimestampFormat::Rfc3339 => now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            TimestampFormat::Rfc3339Nanos => {
                now.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
            }
            TimestampFormat::Custom(format) => now.format(format).to_string(),
        }
    }

    fn format_number(&self, now: &chrono::DateTime<chrono::Utc>) -> u64 {
        match self {
            TimestampFormat::Unix => now.timestamp() as u64,
            TimestampFormat::UnixMillis => now.timestamp_millis() as u64,
            TimestampFormat::Rfc3339 => unreachable!("rfc3339 is not a number"),
            TimestampFormat::Rfc3339Nanos => unreachable!("rfc3339_nanos is not a number"),
            TimestampFormat::Custom(_) => unreachable!("custom is not a number"),
        }
    }
}

#[derive(Debug, Default)]
pub enum Casing {
    #[default]
    Lowercase,
    Uppercase,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("fmt error: {0}")]
    Format(#[from] std::fmt::Error),
    #[error("json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

impl From<Error> for std::fmt::Error {
    fn from(_: Error) -> Self {
        Self
    }
}

/// A builder for the JSON formatter.
/// This is used to configure the JSON formatter.
/// The default configuration is:
/// * level_name: "level"
/// * level_value_casing: Casing::Lowercase
/// * message_name: "message"
/// * target_name: "target"
/// * timestamp_name: "timestamp"
/// * timestamp_format: TimestampFormat::Rfc3339
/// * line_numbers: false
/// * file_names: false
/// * flatten_fields: true
/// * flatten_spans: true
///
/// # Examples
///
/// ```rust
/// use tracing_subscriber::prelude::*;
///
/// tracing_subscriber::registry()
///     .with(
///         tracing_ndjson::Builder::default()
///             .with_level_name("severity")
///             .with_level_value_casing(tracing_ndjson::Casing::Uppercase)
///             .with_message_name("msg")
///             .with_timestamp_name("ts")
///             .with_timestamp_format(tracing_ndjson::TimestampFormat::Unix)
///             .layer(),
///     ).init();
///
/// tracing::info!(life = 42, "Hello, world!");
pub struct Builder {
    layer: crate::JsonFormattingLayer,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            layer: crate::JsonFormattingLayer::default(),
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

/// Alias for `Builder::default()`.
/// This is used to configure the JSON formatter.
pub fn builder() -> Builder {
    Builder::default()
}

impl Builder {
    /// Set the field name for the level field.
    /// The default is "level".
    pub fn with_level_name(mut self, level_name: &'static str) -> Self {
        self.layer.level_name = level_name;
        self
    }

    /// Set the casing for the level field value.
    /// The default is Casing::Lowercase.
    pub fn with_level_value_casing(mut self, casing: Casing) -> Self {
        self.layer.level_value_casing = casing;
        self
    }

    /// Set the field name for the message field.
    /// The default is "message".
    pub fn with_message_name(mut self, message_name: &'static str) -> Self {
        self.layer.message_name = message_name;
        self
    }

    /// Set the field name for the target field.
    /// The default is "target".
    pub fn with_target_name(mut self, target_name: &'static str) -> Self {
        self.layer.target_name = target_name;
        self
    }

    /// Set the field name for the timestamp field.
    /// The default is "timestamp".
    pub fn with_timestamp_name(mut self, timestamp_name: &'static str) -> Self {
        self.layer.timestamp_name = timestamp_name;
        self
    }

    /// Set the timestamp format for the timestamp field.
    /// The default is TimestampFormat::Rfc3339.
    pub fn with_timestamp_format(mut self, timestamp_format: TimestampFormat) -> Self {
        self.layer.timestamp_format = timestamp_format;
        self
    }

    /// Set whether to flatten fields.
    /// The default is true. If false, fields will be nested under a "fields" object.
    pub fn with_flatten_fields(mut self, flatten_fields: bool) -> Self {
        self.layer.flatten_fields = flatten_fields;
        self
    }

    /// Set whether to flatten spans.
    pub fn with_flatten_spans(mut self, flatten_spans: bool) -> Self {
        self.layer.flatten_spans = flatten_spans;
        self
    }

    /// Set whether to include line numbers.
    pub fn with_line_numbers(mut self, line_numbers: bool) -> Self {
        self.layer.line_numbers = line_numbers;
        self
    }

    /// Set whether to include file names.
    pub fn with_file_names(mut self, file_names: bool) -> Self {
        self.layer.file_names = file_names;
        self
    }

    pub fn layer<S>(self) -> impl tracing_subscriber::Layer<S>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        self.layer
    }
}

/// Returns a `Layer` that subscribes to all spans and events using a JSON formatter.
/// This is used to configure the JSON formatter.
pub fn layer<S>() -> impl tracing_subscriber::Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    crate::builder().layer
}

#[cfg(test)]
mod tests {

    use tracing::{debug, error, info, info_span, instrument, trace, warn};
    use tracing_subscriber::prelude::*;

    use super::*;

    #[instrument]
    fn some_function(a: u32, b: u32) {
        let span = info_span!("some_span", a = a, b = b);
        span.in_scope(|| {
            info!("some message from inside a span");
        });
    }

    #[test]
    fn test_json_event_formatter() {
        let subscriber = tracing_subscriber::registry().with(builder().layer());

        tracing::subscriber::with_default(subscriber, || {
            trace!(a = "b", "hello world from trace");
            debug!("hello world from debug");
            info!("hello world from info");
            warn!("hello world from warn");
            error!("hello world from error");
            let span = info_span!(
                "test_span",
                person.firstname = "cole",
                person.lastname = "mackenzie",
                later = tracing::field::Empty,
            );
            span.in_scope(|| {
                info!("some message from inside a info_span");
                let inner = info_span!("inner_span", a = "b", c = "d", inner_span = true);
                inner.in_scope(|| {
                    info!(
                        inner_span_field = true,
                        later = "populated from inside a span",
                        "some message from inside a info_span",
                    );
                });
            });
        });

        let subscriber = tracing_subscriber::registry().with(
            builder()
                .with_level_name("severity")
                .with_level_value_casing(Casing::Uppercase)
                .with_message_name("msg")
                .with_timestamp_name("ts")
                .with_timestamp_format(TimestampFormat::Unix)
                .with_flatten_fields(false)
                .layer(),
        );

        tracing::subscriber::with_default(subscriber, || {
            trace!(a = "b", "hello world from trace");
            debug!("hello world from debug");
            info!("hello world from info");
            warn!("hello world from warn");
            error!("hello world from error");
            let span = info_span!(
                "test_span",
                person.firstname = "cole",
                person.lastname = "mackenzie",
                later = tracing::field::Empty,
            );
            span.in_scope(|| {
                info!("some message from inside a info_span");
                let inner = info_span!("inner_span", a = "b", c = "d", inner_span = true);
                inner.in_scope(|| {
                    info!(
                        inner_span_field = true,
                        later = "populated from inside a span",
                        "some message from inside a info_span",
                    );
                });
            });
        });
    }

    #[test]
    fn test_nested_spans() {
        let subscriber = tracing_subscriber::registry().with(builder().layer());

        tracing::subscriber::with_default(subscriber, || {
            let span = info_span!(
                "test_span",
                person.firstname = "cole",
                person.lastname = "mackenzie",
                later = tracing::field::Empty,
            );
            span.in_scope(|| {
                info!("some message from inside a info_span");
                let inner = info_span!("inner_span", a = "b", c = "d", inner_span = true);
                inner.in_scope(|| {
                    info!(
                        inner_span_field = true,
                        later = "populated from inside a span",
                        "some message from inside a info_span",
                    );
                });
            });

            some_function(1, 2);
        });
    }
}
