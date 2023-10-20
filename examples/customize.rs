use tracing_subscriber::prelude::*;
fn main() {
    let subscriber = tracing_subscriber::registry().with(
        tracing_ndjson::builder()
            .with_level_name("severity")
            .with_level_value_casing(tracing_ndjson::Casing::Uppercase)
            .with_timestamp_name("ts")
            .with_timestamp_format(tracing_ndjson::TimestampFormat::UnixMillis)
            .with_message_name("msg")
            .with_line_numbers(true)
            .with_file_names(true)
            .layer(),
    );

    tracing::subscriber::set_global_default(subscriber).unwrap();

    tracing::info!(life = 42, "Hello, world!");
    // {"life":42,"msg":"Hello, world!","target":"customize","ts":1697836630814,"file":"examples/customize.rs","line":17,"severity":"INFO"}

    let span = tracing::info_span!("hello", "request.uri" = "https://example.com");
    span.in_scope(|| {
        tracing::info!("Hello, world!");
        // {"severity":"INFO","target":"customize","file":"examples/customize.rs","msg":"Hello, world!","ts":1697836630814,"line":22,"request.uri":"https://example.com"}
    });
}
