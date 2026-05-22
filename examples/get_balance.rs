//! Minimal example: print the account balance.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//!     cargo run --example get_balance
//! ```

use voip_ms::{Client, GetBalanceParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (username, password) = credentials()?;
    let client = Client::new(username, password);

    let response = client
        .get_balance(&GetBalanceParams {
            advanced: Some(true),
        })
        .await?;

    println!("{response:#}");
    Ok(())
}

fn credentials() -> Result<(String, String), &'static str> {
    let username = std::env::var("VOIP_MS_USERNAME").map_err(|_| "VOIP_MS_USERNAME is not set")?;
    let password = std::env::var("VOIP_MS_PASSWORD").map_err(|_| "VOIP_MS_PASSWORD is not set")?;
    Ok((username, password))
}
