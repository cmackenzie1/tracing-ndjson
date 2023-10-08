use std::fmt::{self};

use serde::ser::SerializeMap;
use serde::Serializer;

use tracing_core::{Event, Subscriber};
use tracing_serde::fields::AsMap;
use tracing_subscriber::fmt::format::{self, FormatEvent, FormatFields};
use tracing_subscriber::fmt::{FmtContext, FormattedFields, Layer, SubscriberBuilder};
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

/// A JSON formatter for tracing events.
/// This is used to format the event field in the JSON output.
pub struct JsonEventFormatter {
    level_name: &'static str,
    message_name: &'static str,
    target_name: &'static str,
    timestamp_name: &'static str,
    timestamp_format: TimestampFormat,
    flatten_fields: bool,
}

impl Default for JsonEventFormatter {
    fn default() -> Self {
        Self {
            level_name: "level",
            message_name: "message",
            target_name: "target",
            timestamp_name: "timestamp",
            timestamp_format: TimestampFormat::Rfc3339,
            flatten_fields: true,
        }
    }
}

impl JsonEventFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_level_name(mut self, level_name: &'static str) -> Self {
        self.level_name = level_name;
        self
    }

    pub fn with_message_name(mut self, message_name: &'static str) -> Self {
        self.message_name = message_name;
        self
    }

    pub fn with_target_name(mut self, target_name: &'static str) -> Self {
        self.target_name = target_name;
        self
    }

    pub fn with_timestamp_name(mut self, timestamp_name: &'static str) -> Self {
        self.timestamp_name = timestamp_name;
        self
    }

    pub fn with_timestamp_format(mut self, timestamp_format: TimestampFormat) -> Self {
        self.timestamp_format = timestamp_format;
        self
    }

    pub fn with_flatten_fields(mut self, flatten_fields: bool) -> Self {
        self.flatten_fields = flatten_fields;
        self
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

impl<S, N> FormatEvent<S, N> for JsonEventFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let now = chrono::Utc::now();

        let mut buffer = Vec::new();
        let mut binding = serde_json::Serializer::new(&mut buffer);
        let mut serializer = binding.serialize_map(None).map_err(Error::Serde)?;

        serializer
            .serialize_entry(
                self.level_name,
                &event.metadata().level().to_string().to_lowercase(),
            )
            .map_err(Error::Serde)?;

        if matches!(
            self.timestamp_format,
            TimestampFormat::Unix | TimestampFormat::UnixMillis
        ) {
            serializer
                .serialize_entry(
                    self.timestamp_name,
                    &self.timestamp_format.format_number(&now),
                )
                .map_err(Error::Serde)?;
        } else {
            serializer
                .serialize_entry(
                    self.timestamp_name,
                    &self.timestamp_format.format_string(&now),
                )
                .map_err(Error::Serde)?;
        }

        serializer
            .serialize_entry(self.target_name, event.metadata().target())
            .map_err(Error::Serde)?;

        if self.flatten_fields {
            let mut visitor = tracing_serde::SerdeMapVisitor::new(serializer);
            event.record(&mut visitor);

            serializer = visitor.take_serializer().map_err(|_| Error::Unknown)?;
        } else {
            serializer
                .serialize_entry("fields", &event.field_map())
                .map_err(Error::Serde)?;
        };

        let span = event
            .parent()
            .and_then(|id| ctx.span(id))
            .or_else(|| ctx.lookup_current());

        // Write all fields from spans
        if let Some(leaf_span) = span {
            for span in leaf_span.scope().from_root() {
                let ext = span.extensions();
                let data = ext
                    .get::<FormattedFields<N>>()
                    .expect("Unable to find FormattedFields in extensions; this is a bug");

                if !data.is_empty() {
                    let obj: Option<serde_json::Value> = serde_json::from_str(data.as_str()).ok();
                    if matches!(obj, Some(serde_json::Value::Object(_))) {
                        let obj = obj.expect("matched object");
                        for (key, value) in obj.as_object().unwrap() {
                            serializer
                                .serialize_entry(key, value)
                                .map_err(Error::Serde)?;
                        }
                    }
                }
            }
        }

        serializer.end().map_err(Error::Serde)?;
        writer.write_str(std::str::from_utf8(&buffer).map_err(Error::Utf8)?)?;
        writer.write_char('\n')?;

        Ok(())
    }
}

pub struct FieldsFormatter {}

impl FieldsFormatter {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for FieldsFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl<'writer> FormatFields<'writer> for FieldsFormatter {
    fn format_fields<R>(&self, mut writer: format::Writer<'writer>, fields: R) -> fmt::Result
    where
        R: tracing_subscriber::field::RecordFields,
    {
        let mut buffer = Vec::new();
        let mut binding = serde_json::Serializer::new(&mut buffer);
        let mut serializer = binding.serialize_map(None).map_err(Error::Serde)?;
        let mut visitor = tracing_serde::SerdeMapVisitor::new(serializer);

        fields.record(&mut visitor);

        serializer = visitor.take_serializer().map_err(|_| Error::Unknown)?;
        serializer.end().map_err(Error::Serde)?;
        writer.write_str(std::str::from_utf8(&buffer).map_err(Error::Utf8)?)?;

        Ok(())
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
///         tracing_ndjson::builder()
///             .with_level_name("severity")
///             .with_message_name("msg")
///             .with_timestamp_name("ts")
///             .with_timestamp_format(tracing_ndjson::TimestampFormat::Unix)
///             .layer(),
///     ).init();
///
/// tracing::info!(life = 42, "Hello, world!");
pub struct Builder {
    events: JsonEventFormatter,
    fields: FieldsFormatter,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            events: JsonEventFormatter::new(),
            fields: FieldsFormatter::new(),
        }
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builder() -> Builder {
    Builder::default()
}

impl Builder {
    pub fn with_level_name(mut self, level_name: &'static str) -> Self {
        self.events = self.events.with_level_name(level_name);
        self
    }

    pub fn with_message_name(mut self, message_name: &'static str) -> Self {
        self.events = self.events.with_message_name(message_name);
        self
    }

    pub fn with_target_name(mut self, target_name: &'static str) -> Self {
        self.events = self.events.with_target_name(target_name);
        self
    }

    pub fn with_timestamp_name(mut self, timestamp_name: &'static str) -> Self {
        self.events = self.events.with_timestamp_name(timestamp_name);
        self
    }

    pub fn with_timestamp_format(mut self, timestamp_format: TimestampFormat) -> Self {
        self.events = self.events.with_timestamp_format(timestamp_format);
        self
    }

    pub fn with_flatten_fields(mut self, flatten_fields: bool) -> Self {
        self.events = self.events.with_flatten_fields(flatten_fields);
        self
    }

    pub fn layer<S>(self) -> Layer<S, FieldsFormatter, JsonEventFormatter>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        tracing_subscriber::fmt::layer()
            .event_format(self.events)
            .fmt_fields(self.fields)
    }

    pub fn subscriber_builder(self) -> SubscriberBuilder<FieldsFormatter, JsonEventFormatter> {
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
        let formatter = JsonEventFormatter::new();
        let subscriber = tracing_subscriber::fmt()
            .fmt_fields(FieldsFormatter::new())
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

        let formatter = JsonEventFormatter::new()
            .with_level_name("severity")
            .with_message_name("msg")
            .with_timestamp_name("ts")
            .with_timestamp_format(TimestampFormat::Unix);

        let subscriber = tracing_subscriber::fmt()
            .fmt_fields(FieldsFormatter::new())
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
            });
        });
    }
}
