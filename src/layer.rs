use std::collections::HashMap;

use serde_json::json;
use tracing_core::Subscriber;
use tracing_subscriber::{registry::LookupSpan, Layer};

use crate::{storage::JsonStorage, TimestampFormat};

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
        root.insert(
            self.level_name,
            match self.level_value_casing {
                crate::Casing::Lowercase => {
                    json!(event.metadata().level().to_string().to_lowercase())
                }
                crate::Casing::Uppercase => {
                    json!(event.metadata().level().to_string().to_uppercase())
                }
            },
        );

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
            visitor.values().iter().for_each(|(k, v)| {
                if *k == "message" {
                    root.insert(self.message_name, v.clone());
                } else {
                    root.insert(k, v.clone());
                }
            });
        } else {
            let mut fields = HashMap::new();
            visitor.values().iter().for_each(|(k, v)| {
                if *k == "message" {
                    fields.insert(self.message_name, v.clone());
                } else {
                    fields.insert(k, v.clone());
                }
            });
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
                    visitor.values().iter().for_each(|(k, v)| {
                        if *k == "message" {
                            fields.insert(self.message_name, v.clone());
                        } else {
                            fields.insert(k, v.clone());
                        }
                    });
                }
                if !fields.is_empty() {
                    spans.push(fields);
                }
            }
        }

        if !spans.is_empty() {
            if self.flatten_spans {
                spans.iter().for_each(|fields| {
                    fields.iter().for_each(|(k, v)| {
                        if *k == "message" {
                            root.insert(self.message_name, v.clone());
                        } else {
                            root.insert(k, v.clone());
                        }
                    });
                });
            } else {
                root.insert("spans", json!(spans));
            }
        }

        let output = serde_json::to_string(&root).unwrap();
        println!("{}", output);
    }
}
