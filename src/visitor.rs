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
}

impl<'a, W> Visit for Visitor<'a, W>
where
    W: serde::ser::SerializeMap,
{
    fn record_f64(&mut self, field: &tracing_core::Field, value: f64) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value);
        }
    }

    fn record_i64(&mut self, field: &tracing_core::Field, value: i64) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value);
        }
    }

    fn record_u64(&mut self, field: &tracing_core::Field, value: u64) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value);
        }
    }

    fn record_i128(&mut self, field: &tracing_core::Field, value: i128) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value);
        }
    }

    fn record_u128(&mut self, field: &tracing_core::Field, value: u128) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value);
        }
    }

    fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
        if self.state.is_ok() {
            self.state = self.serializer.serialize_entry(field.name(), &value);
        }
    }

    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        if self.state.is_ok() {
            if self.overwrite_message_name.is_some() && field.name() == "message" {
                self.state = self
                    .serializer
                    .serialize_entry(self.overwrite_message_name.expect("message"), &value);
            } else {
                self.state = self.serializer.serialize_entry(field.name(), &value);
            }
        }
    }

    fn record_error(
        &mut self,
        field: &tracing_core::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if self.state.is_ok() {
            self.state = self
                .serializer
                .serialize_entry(field.name(), &format_args!("{}", value).to_string());
        }
    }

    fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn std::fmt::Debug) {
        if self.state.is_ok() {
            if self.overwrite_message_name.is_some() && field.name() == "message" {
                self.state = self.serializer.serialize_entry(
                    self.overwrite_message_name.expect("message"),
                    &format_args!("{:?}", value).to_string(),
                );
            } else {
                self.state = self
                    .serializer
                    .serialize_entry(field.name(), &format_args!("{:?}", value).to_string());
            }
        }
    }
}
