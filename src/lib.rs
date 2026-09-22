//! Async client for the [VoIP.ms](https://voip.ms) REST API.
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
//! Every VoIP.ms API method gets a typed `*Params` request struct (with all
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
//! VoIP.ms uses an `api_username` (your account email) and an `api_password`
//! that is **distinct** from your portal password -- generate it under the
//! "SOAP and REST/JSON API" page in the customer portal and enable API access
//! there.
//!
//! ## IP allow-listing
//!
//! By default **no IP address** may consume the VoIP.ms API. Under
//! "Main Menu" → "SOAP & REST/JSON API" in the portal, add the IP address(es)
//! you'll call from and save. The portal accepts individual addresses, CIDR
//! ranges, wildcard forms (`192.168.1.*`), and DNS names. The sole exception
//! is `getIP` ([`Client::get_ip`]), which works without an allow-listed IP so
//! you can discover the address to add.
//!
//! # Wire format
//!
//! A call is an HTTP `GET` against the REST endpoint ([`DEFAULT_BASE_URL`],
//! `…/api/v1/rest.php`) with parameters in the query string, except one whose
//! parameters carry a base64-encoded file ([`Client::set_recording`],
//! [`Client::send_fax_message`], [`Client::send_mms`],
//! [`Client::add_lnp_file`]): a file does not fit the request line VoIP.ms
//! accepts, so those four are a `multipart/form-data` POST. [`Client`] picks
//! the transport per method, so nothing about the call site changes; a caller
//! dispatching by wire name gets the same choice from
//! [`Client::call_raw_by_name`], or asks [`requires_multipart`] directly --
//! both only for a method this crate was generated from, since one VoIP.ms has
//! added since is absent from the table and so reads as a GET. For that one,
//! choose [`Client::call_multipart_raw`] yourself.
//!
//! The REST endpoint returns the `{ "status": ... }` JSON envelope directly,
//! which this crate deserializes -- a status other than `success` surfaces as
//! [`Error::Api`], except an empty-collection status
//! ([`ApiStatus::is_empty_collection`], e.g. `no_sms`), which the typed methods
//! return as an empty response (the `*_raw` methods still surface it
//! verbatim). (The generic `…/api/v1/` endpoint instead
//! defaults to `text/html` and needs an explicit `content_type=json`; this
//! crate does not use it.)

mod client;
mod error;
mod form;
mod generated;
mod responses;
mod types;

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL, attach_offset};
pub use error::{Error, ParamsError, Result, RetryOutlook, TransportFailure};
pub use generated::*;
pub use types::{
    MaxMembers, Routing, RoutingParseError, Seconds, TimezoneName, TimezoneOffset,
    TimezoneOffsetError, WaitTime,
};

// Dependencies whose types appear in this crate's public API. Re-exported so
// callers can name those types (and `match` on [`Error::Http`]) without adding
// a separate, independently-versioned dependency of their own.
pub use {chrono, chrono_tz, reqwest, rust_decimal, serde, serde_json};

/// Compiles the README's Rust snippets as doctests, so a call site shown there
/// cannot drift from the surface it demonstrates. The README's own text stays
/// out of the rendered docs; only `cargo test` sees this.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct Readme;
