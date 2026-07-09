//! Live API verification harness for CI/workflows.
//!
//! This example is intended for dedicated sandbox credentials in GitHub Actions.
//! It defaults to read-only smoke checks and requires explicit opt-in for
//! state-changing or potentially costly checks.
//!
//! Required environment variables:
//! - VOIP_MS_USERNAME
//! - VOIP_MS_PASSWORD
//!
//! Optional environment variables:
//! - LIVE_VERIFY_MODE=smoke|extended (default: smoke)
//! - LIVE_VERIFY_DRY_RUN=true|false (default: false)
//! - LIVE_VERIFY_ALLOW_STATE_CHANGES=true|false (default: false)
//! - LIVE_VERIFY_ALLOW_COSTLY=true|false (default: false)
//! - LIVE_VERIFY_ENABLE_SMS_SETTINGS_CHECK=true|false (default: false)
//! - LIVE_VERIFY_ENABLE_SUBACCOUNT_CHECK=true|false (default: false)
//! - LIVE_VERIFY_ENABLE_SMS_SEND_CHECK=true|false (default: false)
//! - LIVE_VERIFY_ENABLE_LOCATION_TYPED_CHECKS=true|false (default: false)
//! - LIVE_VERIFY_DIDS_CAN_PROVINCE=ON
//! - LIVE_VERIFY_DIDS_USA_STATE=NY
//! - VOIP_MS_TEST_DID=5551234567
//! - VOIP_MS_SMS_DST=5557654321
//! - VOIP_MS_SMS_MESSAGE=Live verification ping

use std::env;
use std::error::Error;
use std::future::Future;
use std::time::{SystemTime, UNIX_EPOCH};
use voip_ms::{
    Client, CreateSubAccountParams, CreateSubAccountResponse, DelSubAccountParams,
    GetAllowedCodecsParams, GetAllowedCodecsResponse, GetAuthTypesParams, GetAuthTypesResponse,
    GetBalanceParams, GetCountriesParams, GetCountriesResponse, GetDIDCountriesParams,
    GetDIDCountriesResponse, GetDIDsCANParams, GetDIDsCANResponse, GetDIDsInfoParams,
    GetDIDsInfoResponse, GetDIDsUSAParams, GetDIDsUSAResponse, GetDTMFModesParams,
    GetDTMFModesResponse, GetDeviceTypesParams, GetDeviceTypesResponse, GetLanguagesParams,
    GetLanguagesResponse, GetLocalesParams, GetLocalesResponse, GetProtocolsParams,
    GetProtocolsResponse, GetSMSParams, GetSMSResponse, GetServersInfoParams,
    GetServersInfoResponse, GetStatesParams, GetStatesResponse, GetSubAccountsParams,
    GetSubAccountsResponse, SendSMSParams, SetSMSParams, SetSMSResponse,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Smoke,
    Extended,
}

impl Mode {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "smoke" => Ok(Self::Smoke),
            "extended" => Ok(Self::Extended),
            other => Err(format!("unsupported LIVE_VERIFY_MODE `{other}`")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Extended => "extended",
        }
    }
}

#[derive(Debug)]
struct Config {
    username: String,
    password: String,
    mode: Mode,
    dry_run: bool,
    allow_state_changes: bool,
    allow_costly: bool,
    enable_sms_settings_check: bool,
    enable_subaccount_check: bool,
    enable_sms_send_check: bool,
    enable_location_typed_checks: bool,
    dids_can_province: Option<String>,
    dids_usa_state: Option<String>,
    test_did: Option<String>,
    sms_dst: Option<String>,
    sms_message: Option<String>,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let mode = Mode::parse(&env::var("LIVE_VERIFY_MODE").unwrap_or_else(|_| "smoke".into()))?;
        let dry_run = parse_bool_env("LIVE_VERIFY_DRY_RUN", false)?;

        Ok(Self {
            username: if dry_run {
                env::var("VOIP_MS_USERNAME").unwrap_or_else(|_| "dry-run-user".to_string())
            } else {
                env::var("VOIP_MS_USERNAME")
                    .map_err(|_| "VOIP_MS_USERNAME is not set".to_string())?
            },
            password: if dry_run {
                env::var("VOIP_MS_PASSWORD").unwrap_or_else(|_| "dry-run-password".to_string())
            } else {
                env::var("VOIP_MS_PASSWORD")
                    .map_err(|_| "VOIP_MS_PASSWORD is not set".to_string())?
            },
            mode,
            dry_run,
            allow_state_changes: parse_bool_env("LIVE_VERIFY_ALLOW_STATE_CHANGES", false)?,
            allow_costly: parse_bool_env("LIVE_VERIFY_ALLOW_COSTLY", false)?,
            enable_sms_settings_check: parse_bool_env(
                "LIVE_VERIFY_ENABLE_SMS_SETTINGS_CHECK",
                false,
            )?,
            enable_subaccount_check: parse_bool_env("LIVE_VERIFY_ENABLE_SUBACCOUNT_CHECK", false)?,
            enable_sms_send_check: parse_bool_env("LIVE_VERIFY_ENABLE_SMS_SEND_CHECK", false)?,
            enable_location_typed_checks: parse_bool_env(
                "LIVE_VERIFY_ENABLE_LOCATION_TYPED_CHECKS",
                false,
            )?,
            dids_can_province: env::var("LIVE_VERIFY_DIDS_CAN_PROVINCE").ok(),
            dids_usa_state: env::var("LIVE_VERIFY_DIDS_USA_STATE").ok(),
            test_did: env::var("VOIP_MS_TEST_DID").ok(),
            sms_dst: env::var("VOIP_MS_SMS_DST").ok(),
            sms_message: env::var("VOIP_MS_SMS_MESSAGE").ok(),
        })
    }
}

/// Accumulates per-check outcomes so a single failure never stops the rest of
/// the harness from running. Holds the names of every check that failed.
#[derive(Default)]
struct Checks {
    failures: Vec<String>,
}

impl Checks {
    /// Runs one check, printing its outcome and recording the name on failure.
    /// Always returns so the caller proceeds to the next check.
    async fn run<F>(&mut self, name: &str, check: F)
    where
        F: Future<Output = Result<(), Box<dyn Error>>>,
    {
        match check.await {
            Ok(()) => println!("[pass] {name}"),
            Err(error) => {
                eprintln!("[fail] {name}: {error}");
                self.failures.push(name.to_string());
            }
        }
    }

    /// `Ok` if every recorded check passed, otherwise an error naming all
    /// failures so the process exits non-zero.
    fn into_result(self) -> Result<(), Box<dyn Error>> {
        if self.failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} check(s) failed: {}",
                self.failures.len(),
                self.failures.join(", ")
            )
            .into())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    println!("live verification mode: {}", config.mode.as_str());

    if config.dry_run {
        println!("live verification dry run enabled; skipping API calls");
        run_dry_run_checks(&config)?;
        println!("live verification dry run completed successfully");
        return Ok(());
    }

    let client = Client::new(config.username.clone(), config.password.clone());

    let mut checks = Checks::default();
    run_smoke_checks(&client, &config, &mut checks).await;

    if config.mode == Mode::Extended {
        run_extended_checks(&client, &config, &mut checks).await;
    }

    checks.into_result()?;
    println!("live verification completed successfully");
    Ok(())
}

fn run_dry_run_checks(config: &Config) -> Result<(), Box<dyn Error>> {
    println!("[dry-run] smoke checks would run");

    if config.enable_location_typed_checks {
        let _ = config
            .dids_can_province
            .as_deref()
            .ok_or("LIVE_VERIFY_DIDS_CAN_PROVINCE is required when LIVE_VERIFY_ENABLE_LOCATION_TYPED_CHECKS=true")?;
        let _ = config.dids_usa_state.as_deref().ok_or(
            "LIVE_VERIFY_DIDS_USA_STATE is required when LIVE_VERIFY_ENABLE_LOCATION_TYPED_CHECKS=true",
        )?;
        println!("[dry-run] location-dependent checks configured");
    } else {
        println!("[dry-run] location-dependent checks disabled");
    }

    if config.mode == Mode::Extended {
        println!("[dry-run] extended checks would run");

        if config.enable_sms_settings_check {
            if !config.allow_state_changes {
                return Err(
                    "LIVE_VERIFY_ENABLE_SMS_SETTINGS_CHECK=true requires LIVE_VERIFY_ALLOW_STATE_CHANGES=true"
                        .into(),
                );
            }
            let _ = config
                .test_did
                .as_deref()
                .ok_or("VOIP_MS_TEST_DID is required for SMS settings check")?;
            println!("[dry-run] SMS settings check configured");
        } else {
            println!("[dry-run] SMS settings check disabled");
        }

        if config.enable_subaccount_check {
            if !config.allow_state_changes {
                return Err(
                    "LIVE_VERIFY_ENABLE_SUBACCOUNT_CHECK=true requires LIVE_VERIFY_ALLOW_STATE_CHANGES=true"
                        .into(),
                );
            }
            println!("[dry-run] sub-account lifecycle check configured");
        } else {
            println!("[dry-run] sub-account lifecycle check disabled");
        }

        if config.enable_sms_send_check {
            if !config.allow_costly {
                return Err(
                    "LIVE_VERIFY_ENABLE_SMS_SEND_CHECK=true requires LIVE_VERIFY_ALLOW_COSTLY=true"
                        .into(),
                );
            }
            let _ = config
                .test_did
                .as_deref()
                .ok_or("VOIP_MS_TEST_DID is required for SMS send check")?;
            let _ = config
                .sms_dst
                .as_deref()
                .ok_or("VOIP_MS_SMS_DST is required for SMS send check")?;
            let _ = config
                .sms_message
                .as_deref()
                .ok_or("VOIP_MS_SMS_MESSAGE is required for SMS send check")?;
            println!("[dry-run] SMS send check configured");
        } else {
            println!("[dry-run] SMS send check disabled");
        }
    }

    Ok(())
}

async fn run_smoke_checks(client: &Client, config: &Config, checks: &mut Checks) {
    checks
        .run("get_balance", async {
            client
                .get_balance(&GetBalanceParams {
                    advanced: Some(true),
                })
                .await?;
            Ok(())
        })
        .await;

    checks
        .run("get_servers_info", async {
            let servers: GetServersInfoResponse = client
                .get_servers_info(&GetServersInfoParams::default())
                .await?;
            println!(
                "[info] server count: {}",
                servers.servers.len()
            );
            Ok(())
        })
        .await;

    checks
        .run("get_dids_info", async {
            let dids: GetDIDsInfoResponse =
                client.get_dids_info(&GetDIDsInfoParams::default()).await?;
            println!(
                "[info] DID count: {}",
                dids.dids.len()
            );
            Ok(())
        })
        .await;

    checks
        .run("get_sub_accounts", async {
            let sub_accounts: GetSubAccountsResponse = client
                .get_sub_accounts(&GetSubAccountsParams::default())
                .await?;
            println!(
                "[info] sub-account count: {}",
                sub_accounts.accounts.len()
            );
            Ok(())
        })
        .await;

    checks
        .run("get_sms", async {
            let sms: GetSMSResponse = client.get_sms(&GetSMSParams::default()).await?;
            println!(
                "[info] sms count: {}",
                sms.sms.len()
            );
            Ok(())
        })
        .await;

    run_typed_reference_checks(client, checks).await;

    if config.enable_location_typed_checks {
        run_typed_location_checks(client, config, checks).await;
    } else {
        println!("[skip] location-dependent typed checks disabled");
    }
}

async fn run_typed_reference_checks(client: &Client, checks: &mut Checks) {
    checks
        .run("get_allowed_codecs", async {
            let _: GetAllowedCodecsResponse = client
                .get_allowed_codecs(&GetAllowedCodecsParams::default())
                .await?;
            Ok(())
        })
        .await;
    checks
        .run("get_auth_types", async {
            let _: GetAuthTypesResponse = client
                .get_auth_types(&GetAuthTypesParams::default())
                .await?;
            Ok(())
        })
        .await;
    checks
        .run("get_countries", async {
            let _: GetCountriesResponse =
                client.get_countries(&GetCountriesParams::default()).await?;
            Ok(())
        })
        .await;
    checks
        .run("get_did_countries", async {
            let _: GetDIDCountriesResponse = client
                .get_did_countries(&GetDIDCountriesParams {
                    r#type: Some("geographic".to_string()),
                    ..Default::default()
                })
                .await?;
            Ok(())
        })
        .await;
    checks
        .run("get_device_types", async {
            let _: GetDeviceTypesResponse = client
                .get_device_types(&GetDeviceTypesParams::default())
                .await?;
            Ok(())
        })
        .await;
    checks
        .run("get_dtmf_modes", async {
            let _: GetDTMFModesResponse = client
                .get_dtmf_modes(&GetDTMFModesParams::default())
                .await?;
            Ok(())
        })
        .await;
    checks
        .run("get_languages", async {
            let _: GetLanguagesResponse =
                client.get_languages(&GetLanguagesParams::default()).await?;
            Ok(())
        })
        .await;
    checks
        .run("get_locales", async {
            let _: GetLocalesResponse = client.get_locales(&GetLocalesParams::default()).await?;
            Ok(())
        })
        .await;
    checks
        .run("get_protocols", async {
            let _: GetProtocolsResponse =
                client.get_protocols(&GetProtocolsParams::default()).await?;
            Ok(())
        })
        .await;
    checks
        .run("get_states", async {
            let _: GetStatesResponse = client.get_states(&GetStatesParams::default()).await?;
            Ok(())
        })
        .await;
}

async fn run_typed_location_checks(client: &Client, config: &Config, checks: &mut Checks) {
    checks
        .run("get_dids_can", async {
            let dids_can_province = config
                .dids_can_province
                .as_deref()
                .ok_or("LIVE_VERIFY_DIDS_CAN_PROVINCE is required when LIVE_VERIFY_ENABLE_LOCATION_TYPED_CHECKS=true")?;
            let _: GetDIDsCANResponse = client
                .get_dids_can(&GetDIDsCANParams {
                    province: Some(dids_can_province.to_string()),
                    ..Default::default()
                })
                .await?;
            Ok(())
        })
        .await;

    checks
        .run("get_dids_usa", async {
            let dids_usa_state = config.dids_usa_state.as_deref().ok_or(
                "LIVE_VERIFY_DIDS_USA_STATE is required when LIVE_VERIFY_ENABLE_LOCATION_TYPED_CHECKS=true",
            )?;
            let _: GetDIDsUSAResponse = client
                .get_dids_usa(&GetDIDsUSAParams {
                    state: Some(dids_usa_state.to_string()),
                    ..Default::default()
                })
                .await?;
            Ok(())
        })
        .await;
}

async fn run_extended_checks(client: &Client, config: &Config, checks: &mut Checks) {
    if config.enable_sms_settings_check {
        checks
            .run("set_sms settings", async {
                if !config.allow_state_changes {
                    return Err(
                        "LIVE_VERIFY_ENABLE_SMS_SETTINGS_CHECK=true requires LIVE_VERIFY_ALLOW_STATE_CHANGES=true"
                            .into(),
                    );
                }
                let did = config
                    .test_did
                    .as_deref()
                    .ok_or("VOIP_MS_TEST_DID is required for SMS settings check")?;
                verify_sms_settings_endpoint(client, did).await
            })
            .await;
    } else {
        println!("[skip] SMS settings check disabled");
    }

    if config.enable_subaccount_check {
        checks
            .run("sub-account lifecycle", async {
                if !config.allow_state_changes {
                    return Err(
                        "LIVE_VERIFY_ENABLE_SUBACCOUNT_CHECK=true requires LIVE_VERIFY_ALLOW_STATE_CHANGES=true"
                            .into(),
                    );
                }
                verify_subaccount_lifecycle(client).await
            })
            .await;
    } else {
        println!("[skip] sub-account lifecycle check disabled");
    }

    if config.enable_sms_send_check {
        checks
            .run("send_sms", async {
                if !config.allow_costly {
                    return Err(
                        "LIVE_VERIFY_ENABLE_SMS_SEND_CHECK=true requires LIVE_VERIFY_ALLOW_COSTLY=true"
                            .into(),
                    );
                }
                let did = config
                    .test_did
                    .as_deref()
                    .ok_or("VOIP_MS_TEST_DID is required for SMS send check")?;
                let dst = config
                    .sms_dst
                    .as_deref()
                    .ok_or("VOIP_MS_SMS_DST is required for SMS send check")?;
                let message = config
                    .sms_message
                    .as_deref()
                    .ok_or("VOIP_MS_SMS_MESSAGE is required for SMS send check")?;
                verify_send_sms(client, did, dst, message).await
            })
            .await;
    } else {
        println!("[skip] SMS send check disabled");
    }
}

async fn verify_sms_settings_endpoint(client: &Client, did: &str) -> Result<(), Box<dyn Error>> {
    println!("[check] set_sms idempotent update for DID {did}");

    let dids: GetDIDsInfoResponse = client
        .get_dids_info(&GetDIDsInfoParams {
            did: Some(did.to_string()),
            ..Default::default()
        })
        .await?;

    let did_info = dids
        .dids
        .into_iter()
        .find(|item| item.did.as_deref() == Some(did))
        .ok_or_else(|| format!("DID {did} not found"))?;

    if !did_info.sms_available.unwrap_or(false) {
        return Err(format!("DID {did} does not have SMS available").into());
    }

    let response: SetSMSResponse = client
        .set_sms(&SetSMSParams {
            did: Some(did.to_string()),
            enable: did_info.sms_enabled,
            ..Default::default()
        })
        .await?;

    if response.status.as_deref() != Some("success") {
        return Err(format!("setSMS did not return success for DID {did}").into());
    }

    Ok(())
}

async fn verify_subaccount_lifecycle(client: &Client) -> Result<(), Box<dyn Error>> {
    println!("[check] create_sub_account + del_sub_account lifecycle");

    let suffix = unique_suffix()?;
    let username = format!("lv{suffix}");
    let password = format!("Lv{suffix}Pass1");

    let create: CreateSubAccountResponse = client
        .create_sub_account(&CreateSubAccountParams {
            username: Some(username.clone()),
            protocol: Some(1),
            description: Some("live verify temporary sub-account".to_string()),
            auth_type: Some(1),
            password: Some(password),
            ..Default::default()
        })
        .await?;

    let id = create
        .id
        .ok_or("createSubAccount returned success without an id")?;
    println!(
        "[info] created sub-account id={id} account={:?}",
        create.account
    );

    let delete_result = client
        .del_sub_account(&DelSubAccountParams {
            id: Some(i64::try_from(id)?),
        })
        .await;

    match delete_result {
        Ok(_) => Ok(()),
        Err(error) => {
            Err(format!("failed to delete temporary sub-account id={id}: {error}").into())
        }
    }
}

async fn verify_send_sms(
    client: &Client,
    did: &str,
    dst: &str,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    println!("[check] send_sms from {did} to {dst}");

    let response = client
        .send_sms(&SendSMSParams {
            did: Some(did.to_string()),
            dst: Some(dst.to_string()),
            message: Some(message.to_string()),
        })
        .await?;

    if response.status.as_deref() != Some("success") {
        return Err("sendSMS did not return success".into());
    }

    println!("[info] sent sms id={:?}", response.sms);
    Ok(())
}

fn parse_bool_env(name: &str, default: bool) -> Result<bool, String> {
    match env::var(name) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "y" | "on" => Ok(true),
            "0" | "false" | "no" | "n" | "off" => Ok(false),
            other => Err(format!("invalid boolean value for {name}: `{other}`")),
        },
        Err(_) => Ok(default),
    }
}

fn unique_suffix() -> Result<u64, Box<dyn Error>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(now.as_secs())
}
