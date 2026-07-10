//! Runtime configuration for the live harness.
//!
//! Everything sensitive -- API credentials and the egress proxy (URL + auth) --
//! is supplied at runtime via CLI flags or environment variables. Nothing is
//! defaulted to a committed value, and the [`std::fmt::Debug`] impls here redact
//! secrets so a config dump never leaks them.

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

/// Egress proxy configuration. The VoIP.ms per-account API allow-list rejects
/// unknown source IPs (`ip_not_enabled`), so runs from a non-allow-listed host
/// route through an allow-listed proxy.
#[derive(Clone)]
pub struct Proxy {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl std::fmt::Debug for Proxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render the proxy URL (may embed userinfo) or the auth pair.
        f.debug_struct("Proxy")
            .field("url", &"<redacted>")
            .field("username", &self.username.as_ref().map(|_| "<redacted>"))
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
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

    /// Egress proxy URL (http/https/socks5). Its host must be on the VoIP.ms
    /// API allow-list. May embed userinfo, or supply --proxy-username/-password.
    #[arg(long, env = "LIVETEST_PROXY_URL")]
    pub proxy_url: Option<String>,

    /// Proxy basic-auth username.
    #[arg(long, env = "LIVETEST_PROXY_USERNAME")]
    pub proxy_username: Option<String>,

    /// Proxy basic-auth password.
    #[arg(long, env = "LIVETEST_PROXY_PASSWORD")]
    pub proxy_password: Option<String>,

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
    #[arg(long)]
    pub ledger_path: Option<PathBuf>,

    /// List the available areas and exit.
    #[arg(long)]
    pub list_areas: bool,
}

/// Validated configuration derived from [`Cli`].
#[derive(Debug)]
pub struct Config {
    pub username: String,
    pub password: String,
    pub proxy: Option<Proxy>,
    pub depth: Depth,
    pub area_selection: AreaSelection,
    pub confirmed_costly: bool,
    pub ledger_path: PathBuf,
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

        let proxy = match cli.proxy_url {
            Some(url) if !url.trim().is_empty() => Some(Proxy {
                url,
                username: cli.proxy_username.filter(|s| !s.is_empty()),
                password: cli.proxy_password.filter(|s| !s.is_empty()),
            }),
            _ => None,
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

        Ok(Config {
            username,
            password,
            proxy,
            depth: cli.depth,
            area_selection,
            confirmed_costly: cli.i_understand_this_costs_money,
            ledger_path,
        })
    }

    /// Build a [`voip_ms::Client`] routed through the configured proxy (if any).
    pub fn build_client(&self) -> Result<voip_ms::Client> {
        let mut http = reqwest::Client::builder();

        if let Some(proxy) = &self.proxy {
            let mut p = reqwest::Proxy::all(&proxy.url)
                .context("invalid proxy URL (expected http/https/socks5)")?;
            if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
                p = p.basic_auth(user, pass);
            }

            http = http.proxy(p);
        }

        let http = http.build().context("building the proxied HTTP client")?;
        voip_ms::Client::builder(self.username.clone(), self.password.clone())
            .http_client(http)
            .build()
            .context("building the VoIP.ms client")
    }
}
