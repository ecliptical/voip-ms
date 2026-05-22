//! List the DIDs on the account, deserializing the response into a typed struct.
//!
//! Demonstrates the recommended pattern for turning the generic
//! [`serde_json::Value`] response into something with field access: define a
//! struct that matches just the parts you care about, then
//! `serde_json::from_value` the relevant subtree.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//!     cargo run --example list_dids
//! ```

use serde::Deserialize;
use voip_ms::{Client, GetDidsInfoParams};

#[derive(Debug, Deserialize)]
struct Did {
    did: String,
    description: Option<String>,
    routing: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (username, password) = credentials()?;
    let client = Client::new(username, password);

    let response = client.get_dids_info(&GetDidsInfoParams::default()).await?;

    let dids: Vec<Did> = serde_json::from_value(response["dids"].clone())?;
    if dids.is_empty() {
        println!("No DIDs found.");
    } else {
        for did in &dids {
            println!(
                "{}  {}  -> {}",
                did.did,
                did.description.as_deref().unwrap_or("(no description)"),
                did.routing.as_deref().unwrap_or("(no routing)"),
            );
        }
    }
    Ok(())
}

fn credentials() -> Result<(String, String), &'static str> {
    let username = std::env::var("VOIP_MS_USERNAME").map_err(|_| "VOIP_MS_USERNAME is not set")?;
    let password = std::env::var("VOIP_MS_PASSWORD").map_err(|_| "VOIP_MS_PASSWORD is not set")?;
    Ok((username, password))
}
