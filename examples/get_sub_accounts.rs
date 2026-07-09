//! List sub-accounts on the account.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//!     cargo run --example get_sub_accounts
//! ```

use voip_ms::{Client, GetSubAccountsParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if dry_run_enabled("VOIP_MS_DRY_RUN") {
        println!("dry run enabled via VOIP_MS_DRY_RUN=true; skipping API call");
        return Ok(());
    }

    let (username, password) = credentials()?;
    let client = Client::new(username, password);

    let response = client
        .get_sub_accounts(&GetSubAccountsParams::default())
        .await?;

    let status = response.status.as_deref().unwrap_or("(missing)");
    println!("status: {status}");

    let accounts = response.accounts;
    if accounts.is_empty() {
        println!("No sub-accounts found.");
    } else {
        for account in &accounts {
            println!(
                "{}  {}  auth_type={}  device_type={}",
                account.account.as_deref().unwrap_or("(unknown)"),
                account.description.as_deref().unwrap_or("(no description)"),
                account
                    .auth_type
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "(unknown)".to_string()),
                account
                    .device_type
                    .map(|v| v.to_string())
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
