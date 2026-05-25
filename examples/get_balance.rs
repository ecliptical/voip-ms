//! Minimal example: print the account balance.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//!     cargo run --example get_balance
//! ```

use serde_json::Value;
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

    let status = response
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("(missing)");
    println!("status: {status}");

    if let Some(balance) = response.get("balance").and_then(Value::as_object) {
        let current_balance = balance
            .get("current_balance")
            .and_then(value_to_string)
            .unwrap_or_else(|| "(missing)".to_string());
        println!("current balance: {current_balance}");
    }

    Ok(())
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        _ => None,
    }
}

fn credentials() -> Result<(String, String), &'static str> {
    let username = std::env::var("VOIP_MS_USERNAME").map_err(|_| "VOIP_MS_USERNAME is not set")?;
    let password = std::env::var("VOIP_MS_PASSWORD").map_err(|_| "VOIP_MS_PASSWORD is not set")?;
    Ok((username, password))
}
