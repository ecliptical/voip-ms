//! Call any wire method by name and print the JSON envelope it returns.
//!
//! The typed methods answer what a value *is*; this answers what VoIP.ms
//! actually sent, which is what you need when the docs and the response
//! sample disagree, or when VoIP.ms adds a method this crate has not been
//! regenerated for.
//!
//! Parameters are `key=value` pairs, passed through verbatim -- no `*Params`
//! struct is involved, so a value travels exactly as typed.
//!
//! Run with:
//!
//! ```bash
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//!     cargo run --example call_raw -- getMusicOnHold
//!
//! VOIP_MS_USERNAME=you@example.com \
//! VOIP_MS_PASSWORD=your-api-password \
//!     cargo run --example call_raw -- getQueues queue=4764
//! ```
//!
//! A non-`success` status is printed rather than raised, so an error envelope
//! can be read as easily as a successful one.

use std::collections::BTreeMap;

use voip_ms::{Client, requires_multipart, serde_json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let method = args
        .next()
        .ok_or("usage: call_raw <wireMethod> [key=value ...]")?;
    let mut params: BTreeMap<String, String> = BTreeMap::new();
    for arg in args {
        let (key, value) = arg
            .split_once('=')
            .ok_or("each parameter must be `key=value`")?;
        params.insert(key.to_string(), value.to_string());
    }

    let (username, password) = credentials()?;
    let client = Client::new(username, password);

    // The transport is the method's, not a choice: a base64 file parameter
    // does not fit the request line a GET puts it on.
    let response = if requires_multipart(&method) {
        client.call_multipart_raw(&method, &params).await
    } else {
        client.call_raw(&method, &params).await
    };

    match response {
        Ok(envelope) => println!("{}", serde_json::to_string_pretty(&envelope)?),
        // A non-success status is the answer to some questions, so it prints
        // instead of ending the run.
        Err(voip_ms::Error::Api(status)) => println!("API status: {status}"),
        Err(error) => return Err(error.into()),
    }

    Ok(())
}

fn credentials() -> Result<(String, String), &'static str> {
    let username = std::env::var("VOIP_MS_USERNAME").map_err(|_| "VOIP_MS_USERNAME is not set")?;
    let password = std::env::var("VOIP_MS_PASSWORD").map_err(|_| "VOIP_MS_PASSWORD is not set")?;
    Ok((username, password))
}
