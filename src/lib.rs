//! Async client for the [voip.ms](https://voip.ms) REST API.
//!
//! # Quick start
//!
//! ```no_run
//! use voip_ms::{Client, GetBalanceParams};
//!
//! # async fn run() -> voip_ms::Result<()> {
//! let client = Client::new("you@example.com", "your-api-password");
//! let balance = client
//!     .get_balance(&GetBalanceParams { advanced: Some(true) })
//!     .await?;
//! println!("{balance:#}");
//! # Ok(()) }
//! ```
//!
//! # Design
//!
//! Every voip.ms API method gets a typed `*Params` request struct (with all
//! fields wrapped in [`Option`] and skipped when `None`) and a method on
//! [`Client`]. All responses are returned as [`serde_json::Value`] — the
//! API's response shape varies by method and is not described by the WSDL,
//! so this crate doesn't pretend otherwise. Callers who want a typed
//! response can [`serde_json::from_value`] into their own struct, or use
//! [`Client::call`] directly with a custom deserializer.
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

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL};
pub use error::{ApiStatus, Error, Result};
pub use generated::*;
pub use responses::*;
