# POC Rust MTA implementation

- Tokio as async runtime, in particular tokio-rustls to handle secure network connections
- The Stalwart `mail-parser`, `smtp-proto` and `mail-sender` crates for message and protocol parsing and mail sending.
- Uses a postgres database as persistence via sqlx
- Exposes an API with axum

Run `cargo test` to run the tests.


## Development

Install pgtemp:

```bash
cargo install pgtemp --features cli
pgtemp postgresql://remails@localhost:5432/remails
```
