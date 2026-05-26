//! List the DIDs on the account with a generated typed response struct.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//!     cargo run --example list_dids
//! ```

use voip_ms::{Client, GetDIDsInfoParams, GetDIDsInfoResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if dry_run_enabled("VOIP_MS_DRY_RUN") {
        println!("dry run enabled via VOIP_MS_DRY_RUN=true; skipping API call");
        return Ok(());
    }

    let (username, password) = credentials()?;
    let client = Client::new(username, password);

    let response: GetDIDsInfoResponse = client.get_dids_info(&GetDIDsInfoParams::default()).await?;

    let dids = response.dids.unwrap_or_default();
    if dids.is_empty() {
        println!("No DIDs found.");
    } else {
        for did in &dids {
            println!(
                "{}  {}  -> {}  sms_available={}  sms_enabled={}",
                did.did.as_deref().unwrap_or("(unknown DID)"),
                did.description.as_deref().unwrap_or("(no description)"),
                did.routing
                    .as_ref()
                    .map(|r| r.to_string())
                    .unwrap_or_else(|| "(no routing)".to_string()),
                did.sms_available
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "(unknown)".to_string()),
                did.sms_enabled
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "(unknown)".to_string()),
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

fn dry_run_enabled(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "y" | "on"
            )
        })
        .unwrap_or(false)
}
