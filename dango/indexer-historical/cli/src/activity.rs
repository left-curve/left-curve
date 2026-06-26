//! Build the activity projection's [`ActivityConfig`].
//!
//! The event-type filters come from config (each overriding the projection's
//! built-in default), and `involvement_blacklist` is the config addresses
//! merged with the deployment's **system contracts** — read from the node's
//! `app_config` and harvested with `Extractable`, so the per-network addresses
//! never have to be listed by hand.

use {
    crate::config::ActivitySettings,
    anyhow::{Context, bail},
    dango_indexer_historical_projection::ActivityConfig,
    dango_primitives::{Addr, Extractable, Json, QueryClientExt},
    dango_sdk::HttpClient,
    std::{collections::HashSet, str::FromStr, time::Duration},
};

/// How many times to try the `app_config` query before giving up.
const APP_CONFIG_ATTEMPTS: usize = 10;
/// Backoff between `app_config` query attempts.
const APP_CONFIG_BACKOFF: Duration = Duration::from_secs(3);

/// Resolve the activity projection's [`ActivityConfig`]: the event-type filters
/// (config override, else the built-in default), and `involvement_blacklist` =
/// the config addresses ∪ the node's system contracts (from `app_config`).
pub async fn config(settings: &ActivitySettings, node_url: &str) -> anyhow::Result<ActivityConfig> {
    let mut config = ActivityConfig::default();

    if let Some(filter) = settings.event_type_filter.clone() {
        config.event_type_filter = filter;
    }
    if let Some(filter) = settings.event_data_filter.clone() {
        config.event_data_filter = filter;
    }
    if let Some(filter) = settings.involvement_filter.clone() {
        config.involvement_filter = filter;
    }

    // involvement_blacklist = config addresses ∪ the node's system contracts.
    let mut blacklist = HashSet::new();
    for raw in &settings.involvement_blacklist {
        let addr = Addr::from_str(raw)
            .with_context(|| format!("invalid involvement_blacklist address `{raw}`"))?;
        blacklist.insert(addr);
    }
    blacklist.extend(system_contracts(node_url).await?);
    tracing::info!(
        addresses = blacklist.len(),
        "activity involvement blacklist assembled"
    );
    config.involvement_blacklist = blacklist;

    Ok(config)
}

/// Query the node's `app_config` and extract every address in it — the
/// deployment's system contracts. The block source needs the same node, so one
/// that never answers is fatal: retry [`APP_CONFIG_ATTEMPTS`] times with a
/// backoff, then give up and let the caller abort.
async fn system_contracts(node_url: &str) -> anyhow::Result<HashSet<Addr>> {
    let client =
        HttpClient::new(node_url).with_context(|| format!("invalid node url `{node_url}`"))?;

    let mut last_err = None;
    for attempt in 1..=APP_CONFIG_ATTEMPTS {
        match client.query_app_config::<Json>(None).await {
            Ok(app_config) => {
                let mut addresses = HashSet::new();
                app_config.extract_addresses(&mut addresses);
                tracing::info!(
                    count = addresses.len(),
                    "extracted system contracts from app_config"
                );
                return Ok(addresses);
            },
            Err(err) => {
                tracing::warn!(attempt, error = %err, "app_config query failed; retrying");
                last_err = Some(err);
                tokio::time::sleep(APP_CONFIG_BACKOFF).await;
            },
        }
    }

    bail!(
        "app_config query failed after {APP_CONFIG_ATTEMPTS} attempts: {}",
        last_err.expect("at least one attempt ran")
    );
}
