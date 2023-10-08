mod formatter;

use tracing_core::Subscriber;
use tracing_subscriber::fmt::{Layer, SubscriberBuilder};
use tracing_subscriber::registry::LookupSpan;

/// A timestamp format for the JSON formatter.
/// This is used to format the timestamp field in the JSON output.
/// The default is RFC3339.
#[derive(Debug)]
pub enum TimestampFormat {
    /// Seconds since UNIX_EPOCH
    Unix,
    /// Milliseconds since UNIX_EPOCH
    UnixMillis,
    /// RFC3339
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

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("fmt error: {0}")]
    Format(#[from] std::fmt::Error),
    #[error("json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("unknown error")]
    Unknown,
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
/// * message_name: "message"
/// * target_name: "target"
/// * timestamp_name: "timestamp"
/// * timestamp_format: TimestampFormat::Rfc3339
/// * flatten_fields: true
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
///             .with_message_name("msg")
///             .with_timestamp_name("ts")
///             .with_timestamp_format(tracing_ndjson::TimestampFormat::Unix)
///             .layer(),
///     ).init();
///
/// tracing::info!(life = 42, "Hello, world!");
pub struct Builder {
    events: formatter::JsonEventFormatter,
    fields: formatter::FieldsFormatter,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            events: formatter::JsonEventFormatter::new(),
            fields: formatter::FieldsFormatter::new(),
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
        self.events = self.events.with_level_name(level_name);
        self
    }

    /// Set the field name for the message field.
    /// The default is "message".
    pub fn with_message_name(mut self, message_name: &'static str) -> Self {
        self.events = self.events.with_message_name(message_name);
        self
    }

    /// Set the field name for the target field.
    /// The default is "target".
    pub fn with_target_name(mut self, target_name: &'static str) -> Self {
        self.events = self.events.with_target_name(target_name);
        self
    }

    /// Set the field name for the timestamp field.
    /// The default is "timestamp".
    pub fn with_timestamp_name(mut self, timestamp_name: &'static str) -> Self {
        self.events = self.events.with_timestamp_name(timestamp_name);
        self
    }

    /// Set the timestamp format for the timestamp field.
    /// The default is TimestampFormat::Rfc3339.
    pub fn with_timestamp_format(mut self, timestamp_format: TimestampFormat) -> Self {
        self.events = self.events.with_timestamp_format(timestamp_format);
        self
    }

    /// Set whether to flatten fields.
    /// The default is true. If false, fields will be nested under a "fields" object.
    pub fn with_flatten_fields(mut self, flatten_fields: bool) -> Self {
        self.events = self.events.with_flatten_fields(flatten_fields);
        self
    }

    /// Return a `Layer` that subscribes to all spans and events using the defined formatter.
    pub fn layer<S>(self) -> Layer<S, formatter::FieldsFormatter, formatter::JsonEventFormatter>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        tracing_subscriber::fmt::layer()
            .event_format(self.events)
            .fmt_fields(self.fields)
    }

    pub fn subscriber_builder(
        self,
    ) -> SubscriberBuilder<formatter::FieldsFormatter, formatter::JsonEventFormatter> {
        tracing_subscriber::fmt::Subscriber::builder()
            .event_format(self.events)
            .fmt_fields(self.fields)
    }
}

/// Returns a `Layer` that subscribes to all spans and events using a JSON formatter.
/// This is used to configure the JSON formatter.
pub fn layer<S>() -> impl tracing_subscriber::Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    crate::builder().layer()
}

#[cfg(test)]
mod tests {

    use super::*;
    use tracing::{debug, error, info, info_span, trace, warn};
    use tracing_core::Level;

    #[test]
    fn test_json_event_formatter() {
        let formatter = formatter::JsonEventFormatter::new();
        let subscriber = tracing_subscriber::fmt()
            .fmt_fields(formatter::FieldsFormatter::new())
            .event_format(formatter)
            .with_max_level(Level::TRACE)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            trace!(a = "b", "hello world from trace");
            debug!("hello world from debug");
            info!("hello world from info");
            warn!("hello world from warn");
            error!("hello world from error");

            let span = info_span!("test_span", b = "b", d = "d", later = tracing::field::Empty,);
            span.in_scope(|| {
                info!("some message from inside a info_span");
            });
        });

        let formatter = formatter::JsonEventFormatter::new()
            .with_level_name("severity")
            .with_message_name("msg")
            .with_timestamp_name("ts")
            .with_timestamp_format(TimestampFormat::Unix);

        let subscriber = tracing_subscriber::fmt()
            .fmt_fields(formatter::FieldsFormatter::new())
            .event_format(formatter)
            .with_max_level(Level::TRACE)
            .finish();
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
                let inner = info_span!("inner_span", a = "b", c = "d");
                inner.in_scope(|| {
                    info!("some message from inside a info_span");
                });
            });
        });
    }
}
