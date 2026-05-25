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
    if dry_run_enabled("VOIP_MS_DRY_RUN") {
        println!("dry run enabled via VOIP_MS_DRY_RUN=true; skipping API call");
        return Ok(());
    }

    let (username, password) = credentials()?;
    let client = Client::new(username, password);

    let response = client
        .get_balance(&GetBalanceParams {
            advanced: Some(true),
        })
        .await?;

    let status = response.status.as_deref().unwrap_or("(missing)");
    println!("status: {status}");

    if let Some(balance) = response.balance.as_ref() {
        let current_balance = balance
            .current_balance
            .as_ref()
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| "(missing)".to_string());
        println!("current balance: {current_balance}");
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
