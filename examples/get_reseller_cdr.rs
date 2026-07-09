//! Fetch call detail records (CDR) for a specific reseller client.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//! VOIP_MS_CLIENT=500000 \
//! VOIP_MS_DATE_FROM=2024-01-01 \
//! VOIP_MS_DATE_TO=2024-01-31 \
//!     cargo run --example get_reseller_cdr
//! ```

use voip_ms::{Client, GetResellerCDRParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if dry_run_enabled("VOIP_MS_DRY_RUN") {
        println!("dry run enabled via VOIP_MS_DRY_RUN=true; skipping API call");
        return Ok(());
    }

    let (username, password) = credentials()?;
    let client_id: i64 = std::env::var("VOIP_MS_CLIENT")
        .map_err(|_| "VOIP_MS_CLIENT is not set")?
        .parse()
        .map_err(|_| "VOIP_MS_CLIENT must be a numeric client ID")?;
    let date_from =
        std::env::var("VOIP_MS_DATE_FROM").map_err(|_| "VOIP_MS_DATE_FROM is not set")?;
    let date_to = std::env::var("VOIP_MS_DATE_TO").map_err(|_| "VOIP_MS_DATE_TO is not set")?;

    let client = Client::new(username, password);

    let response = client
        .get_reseller_cdr(&GetResellerCDRParams {
            client: Some(client_id),
            date_from: Some(date_from),
            date_to: Some(date_to),
            answered: Some(true),
            ..Default::default()
        })
        .await?;

    let status = response.status.as_deref().unwrap_or("(missing)");
    println!("status: {status}");

    let cdr = response.cdr;
    if cdr.is_empty() {
        println!("No call records found.");
    } else {
        let mut total_seconds: u64 = 0;

        for call in &cdr {
            total_seconds += call.seconds.unwrap_or(0);
            println!(
                "{}  {:>15} -> {:<15}  {}  {}",
                call.date
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "(unknown)".to_string()),
                call.callerid.as_deref().unwrap_or("(unknown)"),
                call.destination
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| "(unknown)".to_string()),
                call.duration.as_deref().unwrap_or("0:00"),
                call.disposition.as_deref().unwrap_or("(unknown)"),
            );
        }

        let h = total_seconds / 3600;
        let m = (total_seconds % 3600) / 60;
        let s = total_seconds % 60;
        println!("\ncalls: {}  duration: {h}:{m:02}:{s:02}", cdr.len());
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
