use std::collections::HashMap;

use serde_json::json;
use tracing_core::Subscriber;
use tracing_subscriber::{Layer, registry::LookupSpan};

use crate::{TimestampFormat, storage::JsonStorage};

type FieldFilter = Box<dyn Fn(&str) -> bool + Send + Sync>;

pub struct JsonFormattingLayer {
    pub(crate) level_name: &'static str,
    pub(crate) level_value_casing: crate::Casing,
    pub(crate) message_name: &'static str,
    pub(crate) target_name: &'static str,
    pub(crate) timestamp_name: &'static str,
    pub(crate) timestamp_format: crate::TimestampFormat,
    pub(crate) line_numbers: bool,
    pub(crate) file_names: bool,
    pub(crate) flatten_fields: bool,
    pub(crate) flatten_spans: bool,
    pub(crate) field_filter: Option<FieldFilter>,
}

impl Default for JsonFormattingLayer {
    fn default() -> Self {
        Self {
            level_name: "level",
            level_value_casing: crate::Casing::default(),
            message_name: "message",
            target_name: "target",
            timestamp_name: "timestamp",
            timestamp_format: crate::TimestampFormat::default(),
            line_numbers: false,
            file_names: false,
            flatten_fields: true,
            flatten_spans: true,
            field_filter: None,
        }
    }
}

impl JsonFormattingLayer {
    fn insert_fields<'k: 'v, 'v>(
        &self,
        source: impl Iterator<Item = (&'v &'k str, &'v serde_json::Value)>,
        dest: &mut HashMap<&'k str, serde_json::Value>,
    ) {
        for (&k, v) in source {
            if let Some(ref f) = self.field_filter {
                if !f(k) {
                    continue;
                }
            }
            let key = if k == "message" { self.message_name } else { k };
            dest.insert(key, v.clone());
        }
    }
}

impl<S> Layer<S> for JsonFormattingLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing_core::span::Attributes<'_>,
        id: &tracing_core::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        // Create a new visitor to store fields
        let mut visitor = JsonStorage::default();

        // Register all fields.
        // Fields on the new span should override fields on the parent span if there is a conflict.
        attrs.record(&mut visitor);

        // Associate the visitor with the Span for future usage via the Span's extensions
        let mut extensions = span.extensions_mut();
        extensions.insert(visitor);
    }

    fn on_record(
        &self,
        span: &tracing_core::span::Id,
        values: &tracing_core::span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span = ctx.span(span).expect("Span not found, this is a bug");

        // Before you can associate a record to an existing Span, well, that Span has to be created!
        // We can thus rely on the invariant that we always associate a JsonVisitor with a Span
        // on creation (`new_span` method), hence it's safe to unwrap the Option.
        let mut extensions = span.extensions_mut();
        let visitor = extensions
            .get_mut::<JsonStorage>()
            .expect("Visitor not found on 'record', this is a bug");
        // Register all new fields
        values.record(visitor);
    }

    fn on_event(
        &self,
        event: &tracing_core::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // Record the event fields
        let mut visitor = crate::storage::JsonStorage::default();
        event.record(&mut visitor);

        let mut root: HashMap<&str, serde_json::Value> = HashMap::new();

        // level
        let level = event.metadata().level();
        let level_str = match self.level_value_casing {
            crate::Casing::Lowercase => match *level {
                tracing_core::Level::TRACE => "trace",
                tracing_core::Level::DEBUG => "debug",
                tracing_core::Level::INFO => "info",
                tracing_core::Level::WARN => "warn",
                tracing_core::Level::ERROR => "error",
            },
            crate::Casing::Uppercase => match *level {
                tracing_core::Level::TRACE => "TRACE",
                tracing_core::Level::DEBUG => "DEBUG",
                tracing_core::Level::INFO => "INFO",
                tracing_core::Level::WARN => "WARN",
                tracing_core::Level::ERROR => "ERROR",
            },
        };
        root.insert(self.level_name, json!(level_str));

        // target
        root.insert(self.target_name, json!(event.metadata().target()));

        // timestamp
        let timestamp = match &self.timestamp_format {
            TimestampFormat::Unix | TimestampFormat::UnixMillis => {
                json!(self.timestamp_format.format_number(&chrono::Utc::now()))
            }
            TimestampFormat::Rfc3339 | TimestampFormat::Rfc3339Nanos => {
                json!(self.timestamp_format.format_string(&chrono::Utc::now()))
            }
            TimestampFormat::Custom(_) => {
                json!(self.timestamp_format.format_string(&chrono::Utc::now()))
            }
        };
        root.insert(self.timestamp_name, timestamp);

        if self.file_names && event.metadata().file().is_some() {
            root.insert("file", json!(event.metadata().file().expect("is some")));
        }

        if self.line_numbers && event.metadata().line().is_some() {
            root.insert("line", json!(event.metadata().line().expect("is some")));
        }

        // Serialize the event fields
        if self.flatten_fields {
            self.insert_fields(visitor.values().iter(), &mut root);
        } else {
            let mut fields = HashMap::new();
            self.insert_fields(visitor.values().iter(), &mut fields);
            root.insert("fields", json!(fields));
        }

        // Span fields (if any)
        let mut spans = vec![];
        if let Some(leaf_span) = _ctx.lookup_current() {
            for span in leaf_span.scope().from_root() {
                let mut fields = HashMap::new();
                let ext = span.extensions();
                let visitor = ext.get::<crate::storage::JsonStorage>();
                if let Some(visitor) = visitor {
                    self.insert_fields(visitor.values().iter(), &mut fields);
                }
                if !fields.is_empty() {
                    spans.push(fields);
                }
            }
        }

        if !spans.is_empty() {
            if self.flatten_spans {
                for fields in &spans {
                    self.insert_fields(fields.iter(), &mut root);
                }
            } else {
                root.insert("spans", json!(spans));
            }
        }

        let output = serde_json::to_string(&root).unwrap();
        println!("{}", output);
    }
}
