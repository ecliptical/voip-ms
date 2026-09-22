# voip-ms

[![Crates.io](https://img.shields.io/crates/v/voip-ms.svg)](https://crates.io/crates/voip-ms/)
[![Docs.rs](https://docs.rs/voip-ms/badge.svg)](https://docs.rs/voip-ms/)
[![CI](https://github.com/ecliptical/voip-ms/actions/workflows/rust-ci.yaml/badge.svg)](https://github.com/ecliptical/voip-ms/actions/workflows/rust-ci.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Async Rust client for the [VoIP.ms](https://voip.ms) REST API.

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

A call is a GET, except `set_recording`, `send_fax_message`, `send_mms`, and
`add_lnp_file`, whose base64 file parameter does not fit the request line
VoIP.ms accepts; those are sent as a `multipart/form-data` POST. The client
picks the transport per method, so nothing about the call site changes.

## Installation

```toml
[dependencies]
voip-ms = "0.13"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

By default the crate enables `rustls` verifying against the OS trust store. To
use the platform's native TLS stack instead:

```toml
voip-ms = { version = "0.13", default-features = false, features = ["native-tls"] }
```

`chrono`, `chrono_tz`, `reqwest`, `rust_decimal`, `serde`, and `serde_json`
appear in this crate's public API and are re-exported from its root, so you can
name their types without adding an independently-versioned dependency of your
own.

## Authentication

VoIP.ms uses two pieces of credential, both of which you control entirely:

* `api_username` — your account email.
* `api_password` — a **distinct** password generated on the
  *SOAP and REST/JSON API* page in the VoIP.ms customer portal.

You must also allow-list the source IP address(es) you'll be calling from on
that same page. This crate does not load credentials from the environment,
files, or any other source — pass them when you construct the [`Client`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html).

## Usage

```rust,no_run
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

Every API method follows the same pattern: construct a `*Params` struct, then
call either:

* `client.some_method(...)` for typed deserialization into a
  `SomeMethodResponse` struct, or
* `client.some_method_raw(...)` for a `serde_json::Value` envelope.

The eight methods the API declares no parameters for (`get_ip`, `get_states`,
…) take no argument at all.

Every `*Params` field is `Option<T>` and omitted from the request when `None`,
so you fill in only what you need. A struct whose documented required fields
are few enough to read positionally also has a `new` constructor for them:

```rust
use voip_ms::SendSMSParams;

let params = SendSMSParams::new("5551234567", "5557654321", "Hello from Rust");
```

Response fields are `Option<T>` too -- except `status`, which every envelope
carries, and list/map fields, which are a bare `Vec`/`HashMap` defaulting to
empty. An omitted field never fails deserialization. Consult the
[VoIP.ms API documentation](https://voip.ms/m/apidocs.php) for which
parameters each method actually requires; the `new` constructors follow the
docs, which are not always exhaustive.

```rust,no_run
use voip_ms::{Client, SendSMSParams};

#[tokio::main]
async fn main() -> voip_ms::Result<()> {
    let client = Client::new("you@example.com", "your-api-password");

    let resp = client
        .send_sms(&SendSMSParams {
            did: Some("5551234567".into()),
            dst: Some("5557654321".into()),
            message: Some("Hello from Rust".into()),
        })
        .await?;

    println!("{resp:#?}");
    Ok(())
}
```

### Reading typed responses

```rust,no_run
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

Both families derive `PartialEq` and `Eq`, so a test can assert a whole
response and a consumer can dedupe or diff records without writing them out
field by field.

### Picking a nested field with a JSON pointer

When you only want one nested field, use
[`Client::call_at`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.call_at)
with a JSON pointer and your own type:

```rust,no_run
use voip_ms::serde::Deserialize;
use voip_ms::{Client, GetDIDsInfoParams};

#[derive(Debug, Deserialize)]
#[serde(crate = "voip_ms::serde")]
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
use voip_ms::{Client, reqwest};

let http = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .unwrap();

let client = Client::builder("you@example.com", "api-password")
    .http_client(http)
    .build();
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

If VoIP.ms adds an API method that isn't yet exposed as a typed call, use
[`Client::call_raw`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.call_raw)
directly with any `serde`-serializable parameter set:

```rust,no_run
use voip_ms::{Client, serde_json};

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

If that method takes a base64 file, it has to be a `multipart/form-data` POST:
a payload larger than the request line reaches the API no other way. Call
[`Client::call_multipart_raw`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.call_multipart_raw)
for it. The choice cannot be made for you here -- this crate knows which methods
carry a file only for the ones it has been regenerated for, and a method it has
never seen is not among them.

For a method the crate *does* know, dispatching by wire name rather than through
the generated method,
[`Client::call_raw_by_name`](https://docs.rs/voip-ms/latest/voip_ms/struct.Client.html#method.call_raw_by_name)
picks the transport itself. It returns what VoIP.ms sent, so the record-listing
methods' timestamps come back without their offset: send those methods an
explicit `timezone`, and complete the envelope with
[`attach_offset`](https://docs.rs/voip-ms/latest/voip_ms/fn.attach_offset.html)
over the paths
[`offset_timestamps`](https://docs.rs/voip-ms/latest/voip_ms/fn.offset_timestamps.html)
answers for the method.

## Error model

All errors surface through [`voip_ms::Error`](https://docs.rs/voip-ms/latest/voip_ms/enum.Error.html). The variants are:

* `Error::Http` -- the request failed at the transport or HTTP-status level.
* `Error::Api(ApiStatus)` -- the response was a well-formed JSON envelope but
  the `status` field was something other than `success`. `ApiStatus` is an
  enum with a variant per documented code (`ApiStatus::InvalidCredentials`,
  `ApiStatus::APINotEnabled`, …) for ergonomic match arms, plus an
  `ApiStatus::Unknown(String)` catch-all that preserves any code VoIP.ms
  hasn't documented. `ApiStatus::description()` returns the documented
  human-readable meaning (or `None` for `Unknown`), `as_str()` gives the
  verbatim wire string, and `is_documented()` reports whether it's a known
  variant. `Display` on the error renders both, so a log line reads
  `API status: did_in_use (DID Number is already in use)`.

  One exception: VoIP.ms returns a distinct `no_*` status per list method when
  the collection is empty (`no_sms`, `no_cdr`, `no_messages`, …). The typed
  methods treat such a status (`ApiStatus::is_empty_collection()`) as a successful empty
  response -- the collection field comes back `None` rather than `Err` -- so you
  don't pattern-match a "no SMS" code where an empty list is the natural answer.
  The `*_raw` methods keep the verbatim contract and still surface it as
  `Error::Api`.

  ```rust,no_run
  # use voip_ms::{Client, GetBalanceParams};
  # async fn run() -> voip_ms::Result<()> {
  # let client = Client::new("you@example.com", "your-api-password");
  # let params = GetBalanceParams::default();
  match client.get_balance(&params).await {
      Ok(balance) => { /* … */ }
      Err(voip_ms::Error::Api(voip_ms::ApiStatus::InvalidCredentials)) => {
          eprintln!("check your API username/password");
      }
      Err(e) => return Err(e),
  }
  # Ok(()) }
  ```
* `Error::InvalidResponse` -- the response was not the expected JSON envelope
  (e.g. missing `status` field).
* `Error::InvalidParams(ParamsError)` -- the parameters could not be converted
  to their wire form, so nothing was sent. `ParamsError` names which check
  failed (today only `ParamsError::Timezone`).

### Classifying a transport failure

`Error::transport()` reduces a failure to a `TransportFailure`, or `None` when
the failure is not a transport one:

```rust,no_run
# use voip_ms::{Client, GetBalanceParams};
# async fn run() -> voip_ms::Result<()> {
# let client = Client::new("you@example.com", "your-api-password");
# let params = GetBalanceParams::default();
use voip_ms::RetryOutlook;

match client.get_balance(&params).await {
    Ok(balance) => { /* … */ }
    Err(e) => match e.transport() {
        // An allow-list rejection is not here: VoIP.ms answers it on a 200
        // with `ApiStatus::IPNotEnabled`, so it stays an `Error::Api`.
        None => return Err(e),
        Some(failure) => match failure.retry_outlook() {
            RetryOutlook::Worthwhile => { /* try again now */ }
            RetryOutlook::AfterWaiting => { /* back off, then retry */ }
            RetryOutlook::Futile => { /* fix something first */ }
            // Repeat only what is safe to repeat: `getBalance` yes,
            // `addCharge` no, unless `failure.never_reached_upstream()`.
            RetryOutlook::Unknown => { /* … */ }
        },
    },
}
# Ok(()) }
```

`never_reached_upstream()` answers whether any account state can have changed;
`retry_outlook()` answers whether repeating the identical call is worth
anything. They are separate questions -- a 401 changed nothing and is still
futile to repeat. Neither type implements `Display`: the classification is
shared, the wording is yours.

## Development and release

Contributor and maintainer workflows (regeneration, verification, and release)
are documented in [DEVELOPMENT.md](DEVELOPMENT.md).

See [AGENTS.md](AGENTS.md) for design decisions and project-specific guidance.

## License

Licensed under the [MIT license](LICENSE).
