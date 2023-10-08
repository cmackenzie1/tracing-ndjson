# examples

## Defaults

```bash
cargo run --example defaults

{"level":"info","timestamp":"2023-10-08T03:35:11Z","target":"defaults","message":"Hello, world!","life":42}
{"level":"info","timestamp":"2023-10-08T03:35:11Z","target":"defaults","message":"Hello, world!","request.uri":"https://example.com"}
```

## Customized

```bash
cargo run --example customize

{"severity":"info","ts":1696736208,"target":"customize","message":"Hello, world!","life":42}
{"severity":"info","ts":1696736208,"target":"customize","message":"Hello, world!","request.uri":"https://example.com"}
```
