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
use voip_ms::chrono::NaiveDate;
use voip_ms::rust_decimal::Decimal;

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

    /// Opt into the fax area's `--depth costly` fixture: order a fax number,
    /// configure it, read it back, then cancel it. Mirrors `--order-test-did`;
    /// off by default even at costly depth -- a real purchase.
    #[arg(long)]
    pub order_test_fax: bool,

    /// Canadian province to search for a purchasable fax number (values from
    /// `getFaxProvinces`, e.g. `ON`). Only consulted with `--order-test-fax`.
    #[arg(long, default_value = "ON")]
    pub fax_search_province: String,

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

    // --- e911 (costly) -----------------------------------------------------
    /// A DID to exercise the e911 area's costly-depth path against. Supplying
    /// it plus the address fields runs `e911Validate` (validate-only, no
    /// provisioning). Provisioning additionally requires `--e911-provision`.
    #[arg(long)]
    pub e911_did: Option<String>,

    /// Actually provision (and then read + cancel) the e911 address for
    /// `--e911-did`, instead of only validating it. Off by default: validation
    /// is preferred over provisioning, which carries a fee and records a
    /// physical address.
    #[arg(long)]
    pub e911_provision: bool,

    /// e911 full name for validation/provisioning (required with `--e911-did`).
    #[arg(long)]
    pub e911_full_name: Option<String>,

    /// e911 street number (required with `--e911-did`).
    #[arg(long)]
    pub e911_street_number: Option<u64>,

    /// e911 street name (required with `--e911-did`).
    #[arg(long)]
    pub e911_street_name: Option<String>,

    /// e911 city (required with `--e911-did`).
    #[arg(long)]
    pub e911_city: Option<String>,

    /// e911 state/province (required with `--e911-did`).
    #[arg(long)]
    pub e911_state: Option<String>,

    /// e911 country, `US` or `CA` (required with `--e911-did`).
    #[arg(long)]
    pub e911_country: Option<String>,

    /// e911 zip/postal code (required with `--e911-did`).
    #[arg(long)]
    pub e911_zip: Option<String>,

    /// e911 language, `EN` or `FR` (required for CA addresses; carried through
    /// to provisioning).
    #[arg(long)]
    pub e911_language: Option<String>,

    // --- account (costly) --------------------------------------------------
    /// Reseller client id for the account area's input-gated reads
    /// (`getCharges`, `getDeposits`) and for the `addCharge`/`addPayment`
    /// mutators. Absent: those methods record skip (no input).
    #[arg(long)]
    pub account_client_id: Option<u64>,

    /// Amount to credit via `addPayment` for `--account-client-id`. Supplying
    /// it is the only way `addPayment` fires -- absence is the safety, since
    /// this moves money and has no dry-run.
    #[arg(long)]
    pub payment_amount: Option<Decimal>,

    /// Amount to debit via `addCharge` for `--account-client-id`. Supplying it
    /// is the only way `addCharge` fires.
    #[arg(long)]
    pub charge_amount: Option<Decimal>,

    /// Start date (`YYYY-MM-DD`) for `getTransactionHistory`; required together
    /// with `--transaction-date-to` for that probe to run.
    #[arg(long)]
    pub transaction_date_from: Option<NaiveDate>,

    /// End date (`YYYY-MM-DD`) for `getTransactionHistory`.
    #[arg(long)]
    pub transaction_date_to: Option<NaiveDate>,

    // --- porting (costly) --------------------------------------------------
    /// A DID to probe `getPortability` against (the read-side, non-committing
    /// portability check). Absent: `getPortability` records skip (no input).
    #[arg(long)]
    pub portability_did: Option<String>,

    /// An existing LNP port id for the id-scoped porting reads (`getLNPDetails`,
    /// `getLNPStatus`, `getLNPNotes`, `getLNPList`, `getLNPAttachList`). Absent:
    /// those record skip (no input).
    #[arg(long)]
    pub port_id: Option<u64>,

    /// Actually submit an LNP port via `addLNPPort`. Off by default and gated:
    /// a port submission commits to moving a number between carriers and has no
    /// dry-run, so it fires only with this flag AND `--port-*` detail inputs.
    #[arg(long)]
    pub submit_port: bool,

    /// Port type code for `addLNPPort` (required with `--submit-port`).
    #[arg(long)]
    pub port_type: Option<u64>,

    /// Number(s) to port for `addLNPPort` (required with `--submit-port`).
    #[arg(long)]
    pub port_numbers: Option<String>,

    /// Losing-carrier account name for `addLNPPort` (required with
    /// `--submit-port`): statement name, provider name, and provider account.
    #[arg(long)]
    pub port_statement_name: Option<String>,

    /// Losing-carrier provider name for `addLNPPort`.
    #[arg(long)]
    pub port_provider_name: Option<String>,

    /// Losing-carrier account number for `addLNPPort`.
    #[arg(long)]
    pub port_provider_account: Option<String>,

    /// Authorizing person's first name for `addLNPPort`.
    #[arg(long)]
    pub port_first_name: Option<String>,

    /// Authorizing person's last name for `addLNPPort`.
    #[arg(long)]
    pub port_last_name: Option<String>,

    /// Service address line 1 for `addLNPPort`.
    #[arg(long)]
    pub port_address: Option<String>,

    /// Service address city for `addLNPPort`.
    #[arg(long)]
    pub port_city: Option<String>,

    /// Service address state/province for `addLNPPort`.
    #[arg(long)]
    pub port_state: Option<String>,

    /// Service address zip/postal code for `addLNPPort`.
    #[arg(long)]
    pub port_zip: Option<String>,

    /// Service address country for `addLNPPort` (`US`/`CA`).
    #[arg(long)]
    pub port_country: Option<String>,

    // --- reseller (costly) -------------------------------------------------
    /// A reseller client id for the reseller area's input-gated reads
    /// (`getClientPackages`, `getClientThreshold`, `getResellerBalance`).
    /// Absent: those record skip (no input).
    #[arg(long)]
    pub reseller_client_id: Option<String>,

    /// Actually create a reseller client via `signupClient`. Off by default and
    /// gated: it moves money (a new billable client) and has no dry-run, so it
    /// fires only with this flag AND the `--signup-*` detail inputs.
    #[arg(long)]
    pub signup_reseller_client: bool,

    /// New client first name for `signupClient` (required with
    /// `--signup-reseller-client`).
    #[arg(long)]
    pub signup_first_name: Option<String>,

    /// New client last name for `signupClient`.
    #[arg(long)]
    pub signup_last_name: Option<String>,

    /// New client email for `signupClient` (also used as the confirmation
    /// email).
    #[arg(long)]
    pub signup_email: Option<String>,

    /// New client password for `signupClient` (also used as the confirmation
    /// password). Runtime-only, redacted in Debug.
    #[arg(long)]
    pub signup_password: Option<String>,

    /// New client street address for `signupClient`.
    #[arg(long)]
    pub signup_address: Option<String>,

    /// New client city for `signupClient`.
    #[arg(long)]
    pub signup_city: Option<String>,

    /// New client state/province for `signupClient`.
    #[arg(long)]
    pub signup_state: Option<String>,

    /// New client country for `signupClient` (values from `getCountries`).
    #[arg(long)]
    pub signup_country: Option<String>,

    /// New client zip/postal code for `signupClient`.
    #[arg(long)]
    pub signup_zip: Option<String>,

    /// New client phone number for `signupClient`.
    #[arg(long)]
    pub signup_phone: Option<String>,

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
    pub order_test_fax: bool,
    pub fax_search_province: String,
    /// The sms/mms `--depth costly` fixture's dedicated DID and destination,
    /// present only when both `--test-did` and `--sms-dst` were supplied.
    pub sms_fixture: Option<SmsFixtureConfig>,
    pub mms_media_url: Option<String>,
    pub e911: E911Config,
    pub account: AccountConfig,
    pub porting: PortingConfig,
    pub reseller: ResellerConfig,
}

/// Inputs for the sms/mms `--depth costly` fixtures, required together.
#[derive(Debug, Clone)]
pub struct SmsFixtureConfig {
    pub test_did: String,
    pub sms_dst: String,
}

/// e911 costly-depth inputs. `did` present + a complete address runs
/// `e911Validate`; provisioning additionally requires `provision`. Each mutator
/// is fired only when its inputs are complete, so an incomplete address records
/// skip (no input) rather than failing.
#[derive(Debug, Clone, Default)]
pub struct E911Config {
    pub did: Option<String>,
    pub provision: bool,
    pub full_name: Option<String>,
    pub street_number: Option<u64>,
    pub street_name: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zip: Option<String>,
    pub language: Option<String>,
}

/// account costly-depth inputs. The money-movers fire only when their amount
/// (and a client id) is supplied; the input-gated reads fire only with their
/// required id/date-window.
#[derive(Debug, Clone, Default)]
pub struct AccountConfig {
    pub client_id: Option<u64>,
    pub payment_amount: Option<Decimal>,
    pub charge_amount: Option<Decimal>,
    pub transaction_date_from: Option<NaiveDate>,
    pub transaction_date_to: Option<NaiveDate>,
}

/// porting costly-depth inputs. `getPortability`/`getLNP*` fire with their id
/// inputs; `addLNPPort` fires only when `submit` is set and the detail fields
/// are complete.
#[derive(Debug, Clone, Default)]
pub struct PortingConfig {
    pub portability_did: Option<String>,
    pub port_id: Option<u64>,
    pub submit: bool,
    pub port_type: Option<u64>,
    pub numbers: Option<String>,
    pub statement_name: Option<String>,
    pub provider_name: Option<String>,
    pub provider_account: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
}

/// reseller costly-depth inputs. The input-gated reads fire with a client id;
/// `signupClient` fires only when `signup` is set and the detail fields are
/// complete. Secrets are redacted in `Debug`.
#[derive(Clone, Default)]
pub struct ResellerConfig {
    pub client_id: Option<String>,
    pub signup: bool,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zip: Option<String>,
    pub phone: Option<String>,
}

impl std::fmt::Debug for ResellerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResellerConfig")
            .field("client_id", &self.client_id)
            .field("signup", &self.signup)
            .field("first_name", &self.first_name)
            .field("last_name", &self.last_name)
            .field("email", &self.email)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("address", &self.address)
            .field("city", &self.city)
            .field("state", &self.state)
            .field("country", &self.country)
            .field("zip", &self.zip)
            .field("phone", &self.phone)
            .finish()
    }
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

        let e911 = E911Config {
            did: cli.e911_did.filter(|s| !s.trim().is_empty()),
            provision: cli.e911_provision,
            full_name: cli.e911_full_name.filter(|s| !s.trim().is_empty()),
            street_number: cli.e911_street_number,
            street_name: cli.e911_street_name.filter(|s| !s.trim().is_empty()),
            city: cli.e911_city.filter(|s| !s.trim().is_empty()),
            state: cli.e911_state.filter(|s| !s.trim().is_empty()),
            country: cli.e911_country.filter(|s| !s.trim().is_empty()),
            zip: cli.e911_zip.filter(|s| !s.trim().is_empty()),
            language: cli.e911_language.filter(|s| !s.trim().is_empty()),
        };

        let account = AccountConfig {
            client_id: cli.account_client_id,
            payment_amount: cli.payment_amount,
            charge_amount: cli.charge_amount,
            transaction_date_from: cli.transaction_date_from,
            transaction_date_to: cli.transaction_date_to,
        };

        let porting = PortingConfig {
            portability_did: cli.portability_did.filter(|s| !s.trim().is_empty()),
            port_id: cli.port_id,
            submit: cli.submit_port,
            port_type: cli.port_type,
            numbers: cli.port_numbers.filter(|s| !s.trim().is_empty()),
            statement_name: cli.port_statement_name.filter(|s| !s.trim().is_empty()),
            provider_name: cli.port_provider_name.filter(|s| !s.trim().is_empty()),
            provider_account: cli.port_provider_account.filter(|s| !s.trim().is_empty()),
            first_name: cli.port_first_name.filter(|s| !s.trim().is_empty()),
            last_name: cli.port_last_name.filter(|s| !s.trim().is_empty()),
            address: cli.port_address.filter(|s| !s.trim().is_empty()),
            city: cli.port_city.filter(|s| !s.trim().is_empty()),
            state: cli.port_state.filter(|s| !s.trim().is_empty()),
            zip: cli.port_zip.filter(|s| !s.trim().is_empty()),
            country: cli.port_country.filter(|s| !s.trim().is_empty()),
        };

        let reseller = ResellerConfig {
            client_id: cli.reseller_client_id.filter(|s| !s.trim().is_empty()),
            signup: cli.signup_reseller_client,
            first_name: cli.signup_first_name.filter(|s| !s.trim().is_empty()),
            last_name: cli.signup_last_name.filter(|s| !s.trim().is_empty()),
            email: cli.signup_email.filter(|s| !s.trim().is_empty()),
            password: cli.signup_password.filter(|s| !s.trim().is_empty()),
            address: cli.signup_address.filter(|s| !s.trim().is_empty()),
            city: cli.signup_city.filter(|s| !s.trim().is_empty()),
            state: cli.signup_state.filter(|s| !s.trim().is_empty()),
            country: cli.signup_country.filter(|s| !s.trim().is_empty()),
            zip: cli.signup_zip.filter(|s| !s.trim().is_empty()),
            phone: cli.signup_phone.filter(|s| !s.trim().is_empty()),
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
            order_test_fax: cli.order_test_fax,
            fax_search_province: cli.fax_search_province,
            sms_fixture,
            mms_media_url: cli.mms_media_url.filter(|s| !s.trim().is_empty()),
            e911,
            account,
            porting,
            reseller,
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
