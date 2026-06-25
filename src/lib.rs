//! Async client for the [voip.ms](https://voip.ms) REST API.
//!
//! # Quick start
//!
//! ```no_run
//! use voip_ms::{Client, GetBalanceParams, GetBalanceResponse};
//!
//! # async fn run() -> voip_ms::Result<()> {
//! let client = Client::new("you@example.com", "your-api-password");
//! let balance: GetBalanceResponse = client
//!     .get_balance(&GetBalanceParams { advanced: Some(true) })
//!     .await?;
//! println!("{balance:#?}");
//! # Ok(()) }
//! ```
//!
//! # Design
//!
//! Every voip.ms API method gets a typed `*Params` request struct (with all
//! fields wrapped in [`Option`] and skipped when `None`) and a method on
//! [`Client`]. The default method deserializes into a generated `*Response`
//! struct; each generated method also has a `*_raw` variant that returns
//! [`serde_json::Value`]. The
//! crate ships a generated `*Response` struct per method (e.g.
//! `GetBalanceResponse`, `GetDIDsInfoResponse`) inferred from the official
//! API documentation's example output, so default calls can deserialize
//! into a known shape without callers writing their own structs.
//!
//! # Authentication
//!
//! voip.ms uses an `api_username` (your account email) and an `api_password`
//! that is **distinct** from your portal password — generate it under the
//! "SOAP and REST/JSON API" page in the customer portal and enable API access
//! there.
//!
//! ## IP allow-listing
//!
//! By default **no IP address** may consume the voip.ms API. Under
//! "Main Menu" → "SOAP & REST/JSON API" in the portal, add the IP address(es)
//! you'll call from and save. The portal accepts individual addresses, CIDR
//! ranges, wildcard forms (`192.168.1.*`), and DNS names. The sole exception
//! is `getIP` ([`Client::get_ip`]), which works without an allow-listed IP so
//! you can discover the address to add.
//!
//! # Wire format
//!
//! All calls are HTTP `GET` against the REST endpoint ([`DEFAULT_BASE_URL`],
//! `…/api/v1/rest.php`) with parameters in the query string. That endpoint
//! returns the `{ "status": ... }` JSON envelope directly, which this crate
//! deserializes — any status other than `success` surfaces as [`Error::Api`].
//! (The generic `…/api/v1/` endpoint instead defaults to `text/html` and
//! needs an explicit `content_type=json`; this crate does not use it.)

mod client;
mod error;
mod generated;
mod responses;
mod types;

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL};
pub use error::{Error, Result};
pub use generated::*;
pub use types::{Routing, RoutingParseError, Seconds, WaitTime};
