//! List the DIDs on the account with partial typed response structs.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//!     cargo run --example list_dids
//! ```

use voip_ms::{Client, GetDidsInfoParams, GetDidsInfoResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (username, password) = credentials()?;
    let client = Client::new(username, password);

    let response: GetDidsInfoResponse = client
        .get_dids_info_typed(&GetDidsInfoParams::default())
        .await?;

    if response.dids.is_empty() {
        println!("No DIDs found.");
    } else {
        for did in &response.dids {
            println!(
                "{}  {}  -> {}",
                did.did.as_deref().unwrap_or("(unknown DID)"),
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
