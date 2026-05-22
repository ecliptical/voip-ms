# voip-ms

[![Crates.io](https://img.shields.io/crates/v/voip-ms.svg)](https://crates.io/crates/voip-ms/)
[![Docs.rs](https://docs.rs/voip-ms/badge.svg)](https://docs.rs/voip-ms/)
[![CI](https://github.com/ecliptical/voip-ms/actions/workflows/rust-ci.yaml/badge.svg)](https://github.com/ecliptical/voip-ms/actions/workflows/rust-ci.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Async client for the [voip.ms](https://voip.ms) REST API.

A thin, idiomatic Rust wrapper around every method exposed by the voip.ms
REST endpoint (`https://voip.ms/api/v1/rest.php`). Each WSDL operation gets a
typed `*Params` request struct and a method on [`Client`]; responses come
back as `serde_json::Value` so callers can pick the fields they need or
[`serde_json::from_value`] into a struct of their own.

## Installation

```toml
[dependencies]
voip-ms = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

By default the crate enables `rustls` with system root certificates. To use a
different TLS backend:

```toml
# Embed Mozilla's roots (good for scratch/distroless images):
voip-ms = { version = "0.1", default-features = false, features = ["rustls-tls-webpki-roots"] }

# Use the platform's native TLS stack:
voip-ms = { version = "0.1", default-features = false, features = ["native-tls"] }
```

## Authentication

voip.ms uses two pieces of credential, both of which you control entirely:

* `api_username` — your account email.
* `api_password` — a **distinct** password generated on the
  *SOAP and REST/JSON API* page in the voip.ms customer portal.

You must also allow-list the source IP address(es) you'll be calling from on
that same page. This crate does not load credentials from the environment,
files, or any other source — pass them when you construct the [`Client`].

## Usage

```rust
use voip_ms::{Client, GetBalanceParams};

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let balance = client
        .get_balance(&GetBalanceParams { advanced: Some(true) })
        .await?;
    println!("{balance:#}");
    Ok(())
}
```

Every API method follows the same pattern: construct a `*Params` struct
(every field is `Option<T>` and omitted from the request when `None`), pass
it to the matching `Client` method, and read the resulting JSON value.

```rust
use voip_ms::{Client, SendSmsParams};

# async fn run(client: voip_ms::Client) -> voip_ms::Result<()> {
let resp = client
    .send_sms(&SendSmsParams {
        did: Some("5551234567".into()),
        dst: Some("5557654321".into()),
        message: Some("Hello from Rust".into()),
        ..Default::default()
    })
    .await?;
# Ok(()) }
```

### Customizing the HTTP client

Use [`Client::builder`] to plug in your own `reqwest::Client` — for proxies,
custom timeouts, retry middleware, or anything else you'd configure on
reqwest directly.

```rust
use std::time::Duration;
use voip_ms::Client;

let http = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .unwrap();

let client = Client::builder("you@example.com", "api-password")
    .http_client(http)
    .build()
    .unwrap();
```

### Typed responses

The WSDL doesn't describe response shapes (all 222 operations declare the
same generic `arrayResponse`), so this crate intentionally hands back
`serde_json::Value`. When you know the shape, deserialize on your side:

```rust
use serde::Deserialize;
use voip_ms::{Client, GetBalanceParams};

#[derive(Deserialize)]
struct Balance {
    current_balance: String,
    spent_total: String,
}

# async fn run(client: Client) -> voip_ms::Result<()> {
let body = client
    .get_balance(&GetBalanceParams { advanced: Some(true) })
    .await?;
let balance: Balance = serde_json::from_value(body["balance"].clone()).unwrap();
# Ok(()) }
```

### Running the examples

The [`examples/`](examples/) directory contains small runnable programs that
read credentials from `VOIP_MS_USERNAME` and `VOIP_MS_PASSWORD`:

```bash
VOIP_MS_USERNAME=you@example.com \
VOIP_MS_PASSWORD=your-api-password \
    cargo run --example get_balance
```

Available examples: `get_balance`, `list_dids`, `send_sms`.

### Calling methods this crate hasn't been regenerated for

If voip.ms adds an API method that isn't yet in this crate, use
[`Client::call`] directly with a `serde`-serializable parameter set:

```rust
# async fn run(client: voip_ms::Client) -> voip_ms::Result<()> {
let resp = client
    .call("someBrandNewMethod", &serde_json::json!({ "id": 42 }))
    .await?;
# Ok(()) }
```

## Error model

All errors surface through [`voip_ms::Error`]. The three variants are:

* `Error::Http` — the request failed at the transport or HTTP-status level.
* `Error::Api(ApiStatus)` — the response was a well-formed JSON envelope but
  the `status` field was something other than `success` (e.g.
  `invalid_credentials`, `missing_method`, `api_not_enabled`). The wire
  string is exposed verbatim — the set of values is per-method and not
  stable, so consult the voip.ms documentation for the methods you use.
* `Error::InvalidResponse` — the response was not the expected JSON envelope
  (e.g. missing `status` field).

## Regenerating the API surface

The 222 typed request structs and `Client` methods are generated from
[`tools/server.wsdl`](tools/server.wsdl) by the `xtask` workspace member
([`xtask/src/main.rs`](xtask/src/main.rs)). To pick up new methods after
voip.ms updates the WSDL:

```bash
# Replace tools/server.wsdl with the new version, then:
cargo xtask gen
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test
```

See [AGENTS.md](AGENTS.md) for the design notes behind the generator.

## License

Licensed under the [MIT license](LICENSE).
