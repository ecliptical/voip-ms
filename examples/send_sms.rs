//! Send an SMS from one of your DIDs.
//!
//! This example actually sends a message and may incur charges on your account.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//! VOIP_MS_FROM_DID=5551234567 \
//! VOIP_MS_TO=5557654321 \
//! VOIP_MS_MESSAGE="Hello from Rust" \
//!     cargo run --example send_sms -- "Hello from Rust"
//! ```

use std::io::Error;
use voip_ms::{Client, GetDIDsInfoParams, GetDIDsInfoResponse, SendSMSParams, SendSMSResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if dry_run_enabled("VOIP_MS_DRY_RUN") {
        println!("dry run enabled via VOIP_MS_DRY_RUN=true; skipping API calls and SMS send");
        return Ok(());
    }

    let (username, password) = credentials()?;
    let from = std::env::var("VOIP_MS_FROM_DID").map_err(|_| "VOIP_MS_FROM_DID is not set")?;
    let to = std::env::var("VOIP_MS_TO").map_err(|_| "VOIP_MS_TO is not set")?;
    let message = message()?;

    let client = Client::new(username, password);

    let dids_response: GetDIDsInfoResponse = client
        .get_dids_info(&GetDIDsInfoParams {
            did: Some(from.clone()),
            ..Default::default()
        })
        .await?;
    let did = dids_response
        .dids
        .into_iter()
        .find(|did| did.did.as_deref() == Some(from.as_str()))
        .ok_or_else(|| Error::other(format!("DID {from} was not found on this account")))?;

    if !did.sms_available.unwrap_or(false) {
        return Err(Error::other(format!("DID {from} does not have SMS available")).into());
    }

    if !did.sms_enabled.unwrap_or(false) {
        return Err(Error::other(format!(
            "SMS is not enabled for DID {from}; enable SMS in VoIP.ms before running this example"
        ))
        .into());
    }

    let response: SendSMSResponse = client
        .send_sms(&SendSMSParams {
            did: Some(from),
            dst: Some(to),
            message: Some(message),
        })
        .await?;

    println!(
        "status: {}",
        response.status.as_deref().unwrap_or("(missing)")
    );

    Ok(())
}

fn credentials() -> Result<(String, String), &'static str> {
    let username = std::env::var("VOIP_MS_USERNAME").map_err(|_| "VOIP_MS_USERNAME is not set")?;
    let password = std::env::var("VOIP_MS_PASSWORD").map_err(|_| "VOIP_MS_PASSWORD is not set")?;
    Ok((username, password))
}

fn message() -> Result<String, &'static str> {
    if let Ok(message) = std::env::var("VOIP_MS_MESSAGE")
        && !message.trim().is_empty()
    {
        return Ok(message);
    }

    std::env::args()
        .nth(1)
        .ok_or("set VOIP_MS_MESSAGE or pass the message body as the first argument")
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
