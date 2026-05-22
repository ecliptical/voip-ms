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
//!     cargo run --example send_sms -- "Hello from Rust"
//! ```

use voip_ms::{Client, SendSmsParams, StatusResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (username, password) = credentials()?;
    let from = std::env::var("VOIP_MS_FROM_DID").map_err(|_| "VOIP_MS_FROM_DID is not set")?;
    let to = std::env::var("VOIP_MS_TO").map_err(|_| "VOIP_MS_TO is not set")?;
    let message = std::env::args()
        .nth(1)
        .ok_or("pass the message body as the first argument")?;

    let client = Client::new(username, password);

    let response: StatusResponse = client
        .send_sms_typed(&SendSmsParams {
            did: Some(from),
            dst: Some(to),
            message: Some(message),
        })
        .await?;

    println!("status: {}", response.status);
    println!("extra fields: {}", response.extra.len());
    Ok(())
}

fn credentials() -> Result<(String, String), &'static str> {
    let username = std::env::var("VOIP_MS_USERNAME").map_err(|_| "VOIP_MS_USERNAME is not set")?;
    let password = std::env::var("VOIP_MS_PASSWORD").map_err(|_| "VOIP_MS_PASSWORD is not set")?;
    Ok((username, password))
}
