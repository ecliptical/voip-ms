# voip-ms

[![Crates.io](https://img.shields.io/crates/v/voip-ms.svg)](https://crates.io/crates/voip-ms/)
[![Docs.rs](https://docs.rs/voip-ms/badge.svg)](https://docs.rs/voip-ms/)
[![CI](https://github.com/ecliptical/voip-ms/actions/workflows/rust-ci.yaml/badge.svg)](https://github.com/ecliptical/voip-ms/actions/workflows/rust-ci.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Async Rust client for the [voip.ms](https://voip.ms) REST API.

The goal is an idiomatic, ergonomic Rust surface over an API that is itself
inconsistent: fields that are really booleans, durations, enums, or routing
targets arrive on the wire as strings (`1`/`0`, `yes`/`no`, `none`,
`account:100001_VoIP`, …), and this crate evens that out into real Rust types
so callers don't have to decode the wire encoding by hand.

Every API method has a typed `*Params` request struct and a method on
[`Client`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html) that
deserializes the response into a typed `*Response` struct. A `*_raw` variant
returning `serde_json::Value` is available on every method as an escape
hatch.

## Installation

```toml
[dependencies]
voip-ms = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

By default the crate enables `rustls` verifying against the OS trust store. To
use the platform's native TLS stack instead:

```toml
voip-ms = { version = "0.2", default-features = false, features = ["native-tls"] }
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

* `client.some_method(...)` for typed deserialization into a
  `SomeMethodResponse` struct, or
* `client.some_method_raw(...)` for a `serde_json::Value` envelope.

All fields on both `*Params` and `*Response` structs are `Option<T>`, so
you only fill in what you need and unknown omissions never fail
deserialization. Consult the
[voip.ms API documentation](https://voip.ms/m/apidocs.php) for which
parameters each method actually requires.

```rust
use voip_ms::{Client, SendSmsParams};

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let resp = client
        .send_sms(&SendSmsParams {
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

### Reading typed responses

```rust
use voip_ms::{Client, GetBalanceParams};

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let resp = client
        .get_balance(&GetBalanceParams { advanced: Some(true) })
        .await?;

    if let Some(balance) = resp.balance.as_ref() {
        println!("{}", balance.current_balance.unwrap_or_default());
    }

    Ok(())
}
```

### Picking a nested field with a JSON pointer

When you only want one nested field, use
[`Client::call_at`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.call_at)
with a JSON pointer and your own type:

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

`send_sms` requires a DID with SMS enabled. You can pass the message body
either through `VOIP_MS_MESSAGE` or as the first argument after `--`.

### Calling a method that isn't in this crate yet

If voip.ms adds an API method that isn't yet exposed as a typed call, use
[`Client::call_raw`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.call_raw)
directly with any `serde`-serializable parameter set:

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
  the `status` field was something other than `success`. `ApiStatus` is an
  enum with a variant per documented code (`ApiStatus::InvalidCredentials`,
  `ApiStatus::APINotEnabled`, …) for ergonomic match arms, plus an
  `ApiStatus::Unknown(String)` catch-all that preserves any code voip.ms
  hasn't documented. `ApiStatus::description()` returns the documented
  human-readable meaning (or `None` for `Unknown`), `as_str()` gives the
  verbatim wire string, and `is_documented()` reports whether it's a known
  variant.

  One exception: voip.ms returns a distinct `no_*` status per list method when
  the collection is empty (`no_sms`, `no_cdr`, `no_messages`, …). The typed
  methods treat such a status (`ApiStatus::is_empty()`) as a successful empty
  response -- the collection field comes back `None` rather than `Err` -- so you
  don't pattern-match a "no SMS" code where an empty list is the natural answer.
  The `*_raw` methods keep the verbatim contract and still surface it as
  `Error::Api`.

  ```rust
  match client.get_balance(&params).await {
      Ok(balance) => { /* … */ }
      Err(voip_ms::Error::Api(voip_ms::ApiStatus::InvalidCredentials)) => {
          eprintln!("check your API username/password");
      }
      Err(e) => return Err(e),
  }
  ```
* `Error::InvalidResponse` — the response was not the expected JSON envelope
  (e.g. missing `status` field).

## Development and release

Contributor and maintainer workflows (regeneration, verification, and release)
are documented in [DEVELOPMENT.md](DEVELOPMENT.md).

See [AGENTS.md](AGENTS.md) for design decisions and project-specific guidance.

## License

Licensed under the [MIT license](LICENSE).
