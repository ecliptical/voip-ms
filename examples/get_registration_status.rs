//! Check whether a SIP sub-account is currently registered.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//! VOIP_MS_ACCOUNT=100001_VoIP \
//!     cargo run --example get_registration_status
//! ```

use voip_ms::{Client, GetRegistrationStatusParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if dry_run_enabled("VOIP_MS_DRY_RUN") {
        println!("dry run enabled via VOIP_MS_DRY_RUN=true; skipping API call");
        return Ok(());
    }

    let (username, password) = credentials()?;
    let account = std::env::var("VOIP_MS_ACCOUNT").map_err(|_| "VOIP_MS_ACCOUNT is not set")?;

    let client = Client::new(username, password);

    let response = client
        .get_registration_status(&GetRegistrationStatusParams {
            account: Some(account.clone()),
        })
        .await?;

    let status = response.status.as_deref().unwrap_or("(missing)");
    println!("status: {status}");

    let registered = response.registered.unwrap_or(false);
    println!("{account} registered: {registered}");

    for reg in response.registrations.unwrap_or_default() {
        println!(
            "  server={} ({})  ip={}  port={}  next_registration={}",
            reg.server_name.as_deref().unwrap_or("(unknown)"),
            reg.server_hostname.as_deref().unwrap_or("(unknown)"),
            reg.server_ip.as_deref().unwrap_or("(unknown)"),
            reg.register_port
                .map(|p| p.to_string())
                .unwrap_or_else(|| "(unknown)".to_string()),
            reg.register_next
                .map(|t| t.to_string())
                .unwrap_or_else(|| "(unknown)".to_string()),
        );
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
