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
//! "SOAP and REST/JSON API" page in the customer portal and allow-list the
//! IP address you'll be calling from.

mod client;
mod error;
mod generated;
mod responses;
mod types;

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL};
pub use error::{ApiStatus, Error, Result};
pub use generated::*;
pub use types::{Routing, RoutingParseError};
