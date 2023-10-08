use std::fmt;

use serde::{ser::SerializeMap, Serializer};

use tracing_core::{Event, Subscriber};
use tracing_serde::fields::AsMap;
use tracing_subscriber::{
    fmt::{format, FmtContext, FormatEvent, FormatFields, FormattedFields},
    registry::LookupSpan,
};

use crate::Error;

/// A JSON formatter for tracing events.
/// This is used to format the event field in the JSON output.
pub struct JsonEventFormatter {
    level_name: &'static str,
    message_name: &'static str,
    target_name: &'static str,
    timestamp_name: &'static str,
    timestamp_format: crate::TimestampFormat,
    flatten_fields: bool,
}

impl Default for JsonEventFormatter {
    fn default() -> Self {
        Self {
            level_name: "level",
            message_name: "message",
            target_name: "target",
            timestamp_name: "timestamp",
            timestamp_format: crate::TimestampFormat::Rfc3339,
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

    pub fn with_timestamp_format(mut self, timestamp_format: crate::TimestampFormat) -> Self {
        self.timestamp_format = timestamp_format;
        self
    }

    pub fn with_flatten_fields(mut self, flatten_fields: bool) -> Self {
        self.flatten_fields = flatten_fields;
        self
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
            crate::TimestampFormat::Unix | crate::TimestampFormat::UnixMillis
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

#[cfg(test)]
mod tests {

    use std::{
        io,
        sync::{Arc, Mutex},
    };

    use crate::builder;

    use super::*;
    use chrono::Utc;
    use tracing::{info, info_span};
    use tracing_subscriber::fmt::{MakeWriter, SubscriberBuilder};

    #[derive(Clone, Debug)]
    struct MockWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    #[derive(Clone, Debug)]
    struct MockMakeWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl MockMakeWriter {
        fn new() -> Self {
            Self {
                buf: Arc::new(Mutex::new(Vec::new())),
            }
        }
        fn get_content(&self) -> String {
            let buf = self.buf.lock().unwrap();
            std::str::from_utf8(&buf[..]).unwrap().to_owned()
        }
    }

    impl MockWriter {
        fn new(buf: Arc<Mutex<Vec<u8>>>) -> Self {
            Self { buf }
        }
    }

    impl io::Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buf.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.buf.lock().unwrap().flush()
        }
    }

    impl<'a> MakeWriter<'a> for MockMakeWriter {
        type Writer = MockWriter;

        fn make_writer(&'a self) -> Self::Writer {
            MockWriter::new(self.buf.clone())
        }
    }

    fn subscriber() -> SubscriberBuilder<FieldsFormatter, JsonEventFormatter> {
        builder().subscriber_builder()
    }

    #[test]
    fn test_json_event_formatter() {
        use tracing::subscriber;

        let mock_writer = MockMakeWriter::new();
        let subscriber = subscriber().with_writer(mock_writer.clone()).finish();

        subscriber::with_default(subscriber, || {
            info!(life = 42, "Hello, world!");
        });

        let content = mock_writer.get_content();

        println!("{:?}", content);

        let obj: Option<serde_json::Value> = serde_json::from_str(&content).ok();
        assert!(matches!(obj, Some(serde_json::Value::Object(_))));
        let obj = obj.expect("matched object");
        assert_eq!(
            obj.get("level").unwrap(),
            &serde_json::Value::String("info".to_owned())
        );
        assert_eq!(
            obj.get("message").unwrap(),
            &serde_json::Value::String("Hello, world!".to_owned())
        );
        assert_eq!(
            obj.get("target").unwrap(),
            &serde_json::Value::String("tracing_ndjson::formatter::tests".to_owned())
        );
        assert_eq!(
            obj.get("life").unwrap(),
            &serde_json::Value::Number(serde_json::Number::from(42))
        );

        let timestamp = obj
            .get("timestamp")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<Utc>>()
            .unwrap();
        assert!(timestamp > Utc::now() - chrono::Duration::seconds(1));
    }

    #[test]
    fn test_json_event_formatter_span() {
        use tracing::subscriber;

        let mock_writer = MockMakeWriter::new();
        let subscriber = subscriber().with_writer(mock_writer.clone()).finish();

        subscriber::with_default(subscriber, || {
            let span = info_span!("hello", "request.uri" = "https://example.com");
            span.in_scope(|| {
                info!("Hello, world!");
            });
        });

        let content = mock_writer.get_content();

        println!("{:?}", content);

        let obj: Option<serde_json::Value> = serde_json::from_str(&content).ok();
        assert!(matches!(obj, Some(serde_json::Value::Object(_))));
        let obj = obj.expect("matched object");
        assert_eq!(
            obj.get("level").unwrap(),
            &serde_json::Value::String("info".to_owned())
        );
        assert_eq!(
            obj.get("message").unwrap(),
            &serde_json::Value::String("Hello, world!".to_owned())
        );
        assert_eq!(
            obj.get("target").unwrap(),
            &serde_json::Value::String("tracing_ndjson::formatter::tests".to_owned())
        );
        assert_eq!(
            obj.get("request.uri").unwrap(),
            &serde_json::Value::String("https://example.com".to_owned())
        );

        let timestamp = obj
            .get("timestamp")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<chrono::DateTime<Utc>>()
            .unwrap();
        assert!(timestamp > Utc::now() - chrono::Duration::seconds(1));
    }
}
