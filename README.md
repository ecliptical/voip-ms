# voip-ms

[![Crates.io](https://img.shields.io/crates/v/voip-ms.svg)](https://crates.io/crates/voip-ms/)
[![Docs.rs](https://docs.rs/voip-ms/badge.svg)](https://docs.rs/voip-ms/)
[![CI](https://github.com/ecliptical/voip-ms/actions/workflows/rust-ci.yaml/badge.svg)](https://github.com/ecliptical/voip-ms/actions/workflows/rust-ci.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Async client for the [voip.ms](https://voip.ms) REST API.

A thin, idiomatic Rust wrapper around every method exposed by the voip.ms
REST endpoint (`https://voip.ms/api/v1/rest.php`). Each WSDL operation gets a
typed `*Params` request struct and methods on [`Client`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html). The default method
deserializes into a generated `*Response` struct, and each operation also has
a `*_raw` variant that returns `serde_json::Value`.

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
files, or any other source — pass them when you construct the [`Client`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html).

## Usage

```rust
use voip_ms::{Client, GetBalanceParams};

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let balance = client
        .get_balance(&GetBalanceParams { advanced: Some(true) })
        .await?;
    println!("{balance:#?}");
    Ok(())
}
```

Every API method follows the same pattern: construct a `*Params` struct
(every field is `Option<T>` and omitted from the request when `None`), then
call either:

* `client.some_method(...)` for typed deserialization into the generated
    `SomeMethodResponse` struct, or
* `client.some_method_raw(...)` for a raw `serde_json::Value` envelope.

```rust
use voip_ms::{Client, SendSmsParams};

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let resp = client
    .send_sms_raw(&SendSmsParams {
        did: Some("5551234567".into()),
        dst: Some("5557654321".into()),
        message: Some("Hello from Rust".into()),
        ..Default::default()
    })
    .await?;

    println!("{resp:#?}");
    Ok(())
}
```

### Customizing the HTTP client

Use [`Client::builder`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.builder) to plug in your own `reqwest::Client` — for proxies,
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
same generic `arrayResponse`), so this crate generates per-method `*Response`
structs inferred from the official HTML docs. Each unsuffixed method returns
its generated `*Response` type, and `*_raw` is available as an escape hatch
when you want the full JSON envelope:

```rust
use voip_ms::{Client, GetBalanceParams, GetBalanceResponse};

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let resp: GetBalanceResponse = client
    .get_balance(&GetBalanceParams { advanced: Some(true) })
    .await?;
    if let Some(balance) = resp.balance.as_ref() {
        println!("{}", balance.current_balance.unwrap_or_default());
    }

    Ok(())
}
```

All fields in the generated `*Response` structs are `Option<T>` so unknown
omissions or future shape drift don't fail deserialization. If you need a
shape the generated struct doesn't capture, use `*_raw` and deserialize
manually, or drop down to `call` / `call_at` with your own type.

For methods where you only want a nested field, use
[`Client::call_at`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.call_at) with a JSON pointer:

```rust
use serde::Deserialize;
use voip_ms::{Client, GetDIDsInfoParams};

#[derive(Debug, Deserialize)]
struct Did {
    did: String,
}

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let dids: Vec<Did> = client
    .call_at("getDIDsInfo", &GetDIDsInfoParams::default(), "/dids")
    .await?;

    println!("DID count: {}", dids.len());
    Ok(())
}
```

### Running the examples

The [`examples/`](examples/) directory contains small runnable programs that
read credentials from `VOIP_MS_USERNAME` and `VOIP_MS_PASSWORD`:

```bash
VOIP_MS_USERNAME=you@example.com \
VOIP_MS_PASSWORD=your-api-password \
    cargo run --example get_balance
```

```bash
VOIP_MS_USERNAME=you@example.com \
VOIP_MS_PASSWORD=your-api-password \
    cargo run --example list_dids
```

```bash
VOIP_MS_USERNAME=you@example.com \
VOIP_MS_PASSWORD=your-api-password \
VOIP_MS_FROM_DID=5551234567 \
VOIP_MS_TO=5557654321 \
VOIP_MS_MESSAGE="Hello from Rust" \
    cargo run --example send_sms
```

`send_sms` requires a DID with SMS enabled. You can pass the message body either
through `VOIP_MS_MESSAGE` or as the first argument after `--`.

### Calling methods this crate hasn't been regenerated for

If voip.ms adds an API method that isn't yet in this crate, use
[`Client::call_raw`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.call_raw) directly with a `serde`-serializable parameter set:

```rust
use voip_ms::Client;

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let resp = client
    .call_raw("someBrandNewMethod", &serde_json::json!({ "id": 42 }))
    .await?;

    println!("{resp:#?}");
    Ok(())
}
```

## Error model

All errors surface through [`voip_ms::Error`](https://docs.rs/voip-ms/latest/voip_ms/enum.Error.html). The three variants are:

* `Error::Http` — the request failed at the transport or HTTP-status level.
* `Error::Api(ApiStatus)` — the response was a well-formed JSON envelope but
  the `status` field was something other than `success` (e.g.
  `invalid_credentials`, `missing_method`, `api_not_enabled`). The wire
  string is exposed verbatim — the set of values is per-method and not
  stable, so consult the voip.ms documentation for the methods you use.
* `Error::InvalidResponse` — the response was not the expected JSON envelope
  (e.g. missing `status` field).

## Development and release

Contributor and maintainer workflows (regeneration, verification, and release)
are documented in [DEVELOPMENT.md](DEVELOPMENT.md).

See [AGENTS.md](AGENTS.md) for design decisions and project-specific guidance.

## License

Licensed under the [MIT license](LICENSE).
