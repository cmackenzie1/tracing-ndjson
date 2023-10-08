use tracing_subscriber::prelude::*;

fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_ndjson::builder()
                .with_level_name("severity")
                .with_message_name("msg")
                .with_timestamp_name("ts")
                .with_timestamp_format(tracing_ndjson::TimestampFormat::Unix)
                .layer(),
        )
        .init();

    tracing::info!(life = 42, "Hello, world!");
    // {"level":"info","timestamp":"2023-10-08T03:30:52Z","target":"default","message":"Hello, world!"}

    let span = tracing::info_span!("hello", "request.uri" = "https://example.com");
    span.in_scope(|| {
        tracing::info!("Hello, world!");
        // {"level":"info","timestamp":"2023-10-08T03:34:33Z","target":"defaults","message":"Hello, world!","request.uri":"https://example.com"}
    });
}
