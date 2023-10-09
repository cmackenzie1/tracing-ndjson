use tracing_core::field::Visit;

pub struct Visitor<'a, W>
where
    W: serde::ser::SerializeMap,
{
    serializer: &'a mut W,
    state: Result<(), W::Error>,
    overwrite_message_name: Option<&'static str>,
}

impl<'a, W> Visitor<'a, W>
where
    W: serde::ser::SerializeMap,
{
    pub fn new(serializer: &'a mut W, overwrite_message_name: Option<&'static str>) -> Self {
        Self {
            serializer,
            state: Ok(()),
            overwrite_message_name,
        }
    }

    pub fn finish(self) -> Result<(), W::Error> {
        self.state
    }

    /// Serialize a key-value pair, replacing the message field if overwrite_message_name is set.
    pub fn serialize_entry<V>(&mut self, key: &str, value: V) -> Result<(), W::Error>
    where
        V: serde::Serialize,
    {
        if self.overwrite_message_name.is_some() && key == "message" {
            self.serializer
                .serialize_entry(self.overwrite_message_name.expect("message"), &value)
        } else {
            self.serializer.serialize_entry(key, &value)
        }
    }
}

impl<'a, W> Visit for Visitor<'a, W>
where
    W: serde::ser::SerializeMap,
{
    fn record_f64(&mut self, field: &tracing_core::Field, value: f64) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), value)
        }
    }

    fn record_i64(&mut self, field: &tracing_core::Field, value: i64) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), value)
        }
    }

    fn record_u64(&mut self, field: &tracing_core::Field, value: u64) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), value)
        }
    }

    fn record_i128(&mut self, field: &tracing_core::Field, value: i128) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), value)
        }
    }

    fn record_u128(&mut self, field: &tracing_core::Field, value: u128) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), value)
        }
    }

    fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), value)
        }
    }

    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), value)
        }
    }

    fn record_error(
        &mut self,
        field: &tracing_core::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), &value.to_string())
        }
    }

    fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn std::fmt::Debug) {
        if self.state.is_ok() {
            self.state = self.serialize_entry(field.name(), &format!("{:?}", value))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{ser::SerializeMap, Serializer};

    #[test]
    fn test_overwrite_message_name() {
        use super::Visitor;

        let mut binding = serde_json::Serializer::new(Vec::new());
        let mut serializer = binding.serialize_map(None).unwrap();
        let mut visitor = Visitor::new(&mut serializer, Some("msg"));

        let _ = visitor.serialize_entry("message", "hello");
        visitor.finish().unwrap();
        serializer.end().unwrap();

        let result = String::from_utf8(binding.into_inner()).unwrap();
        assert_eq!(result, r#"{"msg":"hello"}"#);
    }
}
