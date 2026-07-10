//! Runtime configuration for the live harness.
//!
//! Everything sensitive -- the VoIP.ms API credentials and any API-server
//! add-on Basic auth -- is supplied at runtime via CLI flags or environment
//! variables. Nothing is defaulted to a committed value, and the
//! [`std::fmt::Debug`] impls here redact secrets so a config dump never leaks
//! them.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};

/// How far the harness goes within each selected area.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Depth {
    /// Read-only calls only (every `get*`/reference method), with raw-vs-typed
    /// drift diffing.
    Probe,
    /// Free create -> read -> delete fixtures, in addition to probes.
    Lifecycle,
    /// Money/irreversible methods, in addition to lifecycle. Requires the
    /// confirmation flag.
    Costly,
}

impl Depth {
    /// True when this depth is at least `other` (probe < lifecycle < costly).
    pub fn at_least(self, other: Depth) -> bool {
        self.rank() >= other.rank()
    }

    fn rank(self) -> u8 {
        match self {
            Depth::Probe => 0,
            Depth::Lifecycle => 1,
            Depth::Costly => 2,
        }
    }
}

/// Optional HTTP Basic auth an API server may require, separate from the VoIP.ms
/// API credentials. A server that emulates or fronts the VoIP.ms API (e.g. one
/// hosted on an allow-listed IP) may gate access this way.
#[derive(Clone)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for BasicAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicAuth")
            .field("username", &"<redacted>")
            .field("password", &"<redacted>")
            .finish()
    }
}

/// Parsed command line + environment. Secrets are redacted in `Debug`.
#[derive(Parser)]
#[command(
    name = "livetest",
    about = "Live VoIP.ms API integration harness (operator-local).",
    long_about = "Exercises the VoIP.ms API against a real account to catch response-shape \
                  drift. Selection is two-dimensional: functional AREAS (which method groups) \
                  and DEPTH (how far -- probe/lifecycle/costly). A pre-flight sweep reconciles \
                  leftover fixtures from prior runs before creating new ones."
)]
pub struct Cli {
    /// VoIP.ms API username (account email). Required for a run; optional for
    /// metadata-only invocations like --list-areas.
    #[arg(long, env = "VOIP_MS_USERNAME")]
    pub username: Option<String>,

    /// VoIP.ms API password (from the SOAP & REST/JSON API portal page).
    /// Required for a run; optional for --list-areas.
    #[arg(long, env = "VOIP_MS_PASSWORD")]
    pub password: Option<String>,

    /// API server REST endpoint URL. Defaults to the real VoIP.ms API server;
    /// point it at another server that emulates or fronts the VoIP.ms API
    /// (e.g. one on an allow-listed IP).
    #[arg(long, env = "LIVETEST_API_URL")]
    pub api_url: Option<String>,

    /// Username for the API server's optional add-on HTTP Basic auth, separate
    /// from the VoIP.ms API credentials.
    #[arg(long, env = "LIVETEST_API_BASIC_USERNAME")]
    pub api_basic_username: Option<String>,

    /// Password for the API server's optional add-on HTTP Basic auth.
    #[arg(long, env = "LIVETEST_API_BASIC_PASSWORD")]
    pub api_basic_password: Option<String>,

    /// Depth within each selected area.
    #[arg(long, value_enum, default_value_t = Depth::Probe)]
    pub depth: Depth,

    /// Run only these areas (comma-separated). Mutually exclusive with
    /// --all-areas. Default (neither flag): every free area, costly-by-nature
    /// areas (porting, e911, reseller, charges) excluded until named.
    #[arg(long, value_delimiter = ',')]
    pub areas: Vec<String>,

    /// Run every area, including costly-by-nature ones.
    #[arg(long, conflicts_with = "areas")]
    pub all_areas: bool,

    /// Remove these areas from the selected set (comma-separated).
    #[arg(long, value_delimiter = ',')]
    pub exclude: Vec<String>,

    /// Required acknowledgement to run any money/irreversible method at
    /// --depth costly. A deliberate speed bump, not a wall.
    #[arg(long)]
    pub i_understand_this_costs_money: bool,

    /// Path to the operator-local run-ledger (tracks un-markable resources like
    /// DIDs). Git-ignored; may contain DIDs/usernames. Defaults to
    /// ./livetest-ledger.jsonl in the current directory.
    #[arg(long, env = "LIVETEST_LEDGER_PATH")]
    pub ledger_path: Option<PathBuf>,

    /// Canadian province to search for a purchasable DID (values from
    /// `getProvinces`, e.g. `ON`). Only consulted with `--order-test-did`.
    #[arg(long, default_value = "ON")]
    pub did_search_province: String,

    /// Query for the DID search (a `contains` search over the number). Only
    /// consulted with `--order-test-did`.
    #[arg(long, default_value = "5")]
    pub did_search_query: String,

    /// Opt into the dids area's own `--depth costly` fixture: order a DID,
    /// enable SMS, read it back, then cancel it. Self-contained and off by
    /// default even at costly depth -- a real purchase, kept separate from the
    /// dedicated `--test-did` used by the sms/mms fixtures.
    #[arg(long)]
    pub order_test_did: bool,

    /// A dedicated, already SMS/MMS-enabled DID for the sms/mms `--depth
    /// costly` fixtures. Provisioned once by the operator outside the harness;
    /// the harness only ever reads and sends through it, never cancels it.
    /// Required together with `--sms-dst` for those fixtures to run.
    #[arg(long)]
    pub test_did: Option<String>,

    /// Destination number for the sms/mms `--depth costly` sendSMS/sendMMS
    /// fixtures. Required together with `--test-did`.
    #[arg(long)]
    pub sms_dst: Option<String>,

    /// Media URL for the mms `--depth costly` sendMMS fixture (optional;
    /// sendMMS works with just a message).
    #[arg(long)]
    pub mms_media_url: Option<String>,

    /// List the available areas and exit.
    #[arg(long)]
    pub list_areas: bool,
}

/// Validated configuration derived from [`Cli`].
#[derive(Debug)]
pub struct Config {
    pub username: String,
    pub password: String,
    /// API server REST endpoint. `None` means the default VoIP.ms server.
    pub api_url: Option<String>,
    /// The API server's optional add-on Basic auth.
    pub basic_auth: Option<BasicAuth>,
    pub depth: Depth,
    pub area_selection: AreaSelection,
    pub confirmed_costly: bool,
    pub ledger_path: PathBuf,
    pub did_search_province: String,
    pub did_search_query: String,
    pub order_test_did: bool,
    /// The sms/mms `--depth costly` fixture's dedicated DID and destination,
    /// present only when both `--test-did` and `--sms-dst` were supplied.
    pub sms_fixture: Option<SmsFixtureConfig>,
    pub mms_media_url: Option<String>,
}

/// Inputs for the sms/mms `--depth costly` fixtures, required together.
#[derive(Debug, Clone)]
pub struct SmsFixtureConfig {
    pub test_did: String,
    pub sms_dst: String,
}

/// Which areas to run, before intersecting with the known-area registry.
#[derive(Debug)]
pub enum AreaSelection {
    /// Default: all free areas minus `exclude`.
    DefaultFree { exclude: Vec<String> },
    /// `--all-areas` minus `exclude`.
    All { exclude: Vec<String> },
    /// `--areas a,b,c` minus `exclude`.
    Explicit {
        include: Vec<String>,
        exclude: Vec<String>,
    },
}

impl Config {
    /// Validate a parsed [`Cli`] into a [`Config`], failing on contradictions
    /// (e.g. costly depth without acknowledgement).
    pub fn from_cli(cli: Cli) -> Result<Self> {
        let username = cli
            .username
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("VOIP_MS_USERNAME / --username is required"))?;
        let password = cli
            .password
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("VOIP_MS_PASSWORD / --password is required"))?;

        if cli.depth == Depth::Costly && !cli.i_understand_this_costs_money {
            bail!(
                "--depth costly requires --i-understand-this-costs-money; \
                 it can order DIDs, send messages, and move money"
            );
        }

        let api_url = cli.api_url.filter(|s| !s.trim().is_empty());

        let basic_auth = match (
            cli.api_basic_username.filter(|s| !s.is_empty()),
            cli.api_basic_password.filter(|s| !s.is_empty()),
        ) {
            (Some(username), Some(password)) => Some(BasicAuth { username, password }),
            (None, None) => None,
            _ => bail!("--api-basic-username and --api-basic-password must be set together"),
        };

        let area_selection = if cli.all_areas {
            AreaSelection::All {
                exclude: cli.exclude,
            }
        } else if !cli.areas.is_empty() {
            AreaSelection::Explicit {
                include: cli.areas,
                exclude: cli.exclude,
            }
        } else {
            AreaSelection::DefaultFree {
                exclude: cli.exclude,
            }
        };

        let ledger_path = cli
            .ledger_path
            .unwrap_or_else(|| PathBuf::from("livetest-ledger.jsonl"));

        let test_did = cli.test_did.filter(|s| !s.trim().is_empty());
        let sms_dst = cli.sms_dst.filter(|s| !s.trim().is_empty());
        let sms_fixture = match (test_did, sms_dst) {
            (Some(test_did), Some(sms_dst)) => Some(SmsFixtureConfig { test_did, sms_dst }),
            (None, None) => None,
            _ => bail!("--test-did and --sms-dst must be set together"),
        };

        Ok(Config {
            username,
            password,
            api_url,
            basic_auth,
            depth: cli.depth,
            area_selection,
            confirmed_costly: cli.i_understand_this_costs_money,
            ledger_path,
            did_search_province: cli.did_search_province,
            did_search_query: cli.did_search_query,
            order_test_did: cli.order_test_did,
            sms_fixture,
            mms_media_url: cli.mms_media_url.filter(|s| !s.trim().is_empty()),
        })
    }

    /// Build a [`voip_ms::Client`] against the configured API server, applying
    /// the server's optional add-on Basic auth.
    pub fn build_client(&self) -> Result<voip_ms::Client> {
        let mut http = reqwest::Client::builder();

        // Add-on Basic auth (separate from the VoIP.ms API credentials) as a
        // default header on every request.
        if let Some(auth) = &self.basic_auth {
            let mut headers = reqwest::header::HeaderMap::new();
            let token = base64_encode(&format!("{}:{}", auth.username, auth.password));
            let mut value = reqwest::header::HeaderValue::from_str(&format!("Basic {token}"))
                .context("building the API server Basic-auth header")?;
            value.set_sensitive(true);
            headers.insert(reqwest::header::AUTHORIZATION, value);
            http = http.default_headers(headers);
        }

        let http = http.build().context("building the HTTP client")?;

        let mut builder = voip_ms::Client::builder(self.username.clone(), self.password.clone())
            .http_client(http);

        if let Some(api_url) = &self.api_url {
            let url = reqwest::Url::parse(api_url)
                .context("invalid --api-url (expected a full REST endpoint URL)")?;
            builder = builder.base_url(url);
        }

        builder.build().context("building the VoIP.ms client")
    }
}

/// Minimal standard-alphabet base64 for the Basic-auth header, avoiding a
/// dependency for one small encode.
fn base64_encode(input: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }

    out
}
