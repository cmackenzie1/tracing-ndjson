use tracing_subscriber::prelude::*;
fn main() {
    let subscriber = tracing_subscriber::registry().with(tracing_ndjson::layer());

    tracing::subscriber::set_global_default(subscriber).unwrap();

    tracing::info!(life = 42, "Hello, world!");
    // {"level":"info","target":"default","life":42,"timestamp":"2023-10-20T21:17:49Z","message":"Hello, world!"}

    let span = tracing::info_span!("hello", "request.uri" = "https://example.com");
    span.in_scope(|| {
        tracing::info!("Hello, world!");
        // {"message":"Hello, world!","request.uri":"https://example.com","level":"info","target":"default","timestamp":"2023-10-20T21:17:49Z"}
    });
}
