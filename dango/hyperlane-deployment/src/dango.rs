use {
    crate::config::{Config, dango::DangoConfig, evm::EVMConfig},
    alloy::primitives::Address,
    dango_client::{Secp256k1, SingleSigner},
    dango_types::{
        auth::Nonce,
        config::AppConfig,
        gateway::{self, Origin, Remote},
    },
    grug::{
        BroadcastClientExt, Coins, Defined, GasOption, HexByteArray, QueryClientExt,
        SearchTxClient, StdResult,
    },
    hyperlane_types::{Addr32, isms::multisig::ValidatorSet},
    indexer_client::HttpClient,
    std::{collections::BTreeSet, str::FromStr},
};

pub async fn set_warp_routes(
    dango_client: &HttpClient,
    dango_config: &DangoConfig,
    signer: &mut SingleSigner<Secp256k1, Defined<Nonce>>,
    remote_domain: u32,
    routes: BTreeSet<(String, Address)>,
) -> anyhow::Result<()> {
    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    let routes = routes
        .into_iter()
        .map(|(symbol, address)| {
            Ok((
                Origin::Remote(symbol.try_into()?),
                app_cfg.addresses.warp,
                Remote::Warp {
                    domain: remote_domain,
                    contract: Addr32::from_inner(address.into_word().into()),
                },
            ))
        })
        .collect::<StdResult<BTreeSet<_>>>()?;

    println!("Setting routes on Dango gateway: {routes:#?}");

    dango_client
        .execute(
            signer,
            app_cfg.addresses.gateway,
            &gateway::ExecuteMsg::SetRoutes(routes),
            Coins::new(),
            grug::GasOption::Predefined {
                gas_limit: 1_000_000_u64,
            },
            dango_config.chain_id.as_str(),
        )
        .await?;

    println!("Done!");

    Ok(())
}

pub async fn set_ism_validator_set(
    dango_client: &HttpClient,
    config: &Config,
    signer: &mut SingleSigner<Secp256k1, Defined<Nonce>>,
    evm_config: &EVMConfig,
) -> anyhow::Result<()> {
    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;

    // Query mailbox Config
    let mailbox_config = dango_client
        .query_wasm_smart(
            app_cfg.addresses.hyperlane.mailbox,
            hyperlane_types::mailbox::QueryConfigRequest {},
            None,
        )
        .await?;
    println!("mailbox_config: {:#?}", mailbox_config);

    // Query the mock validator set from the ISM contract
    let validator_sets = dango_client
        .query_wasm_smart(
            app_cfg.addresses.hyperlane.ism,
            hyperlane_types::isms::multisig::QueryValidatorSetsRequest {
                start_after: None,
                limit: None,
            },
            None,
        )
        .await?;

    println!("validator_sets: {:#?}", validator_sets);

    // Get the validator set for the remote domain from the config
    let validator_set = config
        .dango
        .isms
        .iter()
        .find(|(domain, _)| *domain == evm_config.hyperlane_domain)
        .unwrap()
        .1
        .clone();

    let new_validator_set = ValidatorSet {
        threshold: validator_set.threshold,
        validators: validator_set
            .validators
            .into_iter()
            .map(|validator| HexByteArray::from_str(&validator).unwrap())
            .collect(),
    };

    match validator_sets.get(&evm_config.hyperlane_domain) {
        Some(validator_set) => {
            println!("Current validator_set: {:#?}", validator_set);
            if validator_set.threshold == new_validator_set.threshold
                && validator_set.validators == new_validator_set.validators
            {
                println!("Validator set is already the correct validator set.");
                return Ok(());
            }
        },
        None => {
            println!("No validator_set found for the remote domain");
        },
    }

    println!("Setting the mock validator set for the remote domain to the mock validator set...");

    // Set the validators set for the remote domain
    let set_validators_msg = hyperlane_types::isms::multisig::ExecuteMsg::SetValidators {
        domain: evm_config.hyperlane_domain,
        threshold: new_validator_set.threshold,
        validators: new_validator_set.validators.clone(),
    };
    let outcome = dango_client
        .execute(
            signer,
            app_cfg.addresses.hyperlane.ism,
            &set_validators_msg,
            Coins::new(),
            GasOption::Predefined { gas_limit: 1000000 },
            config.dango.chain_id.as_str(),
        )
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let outcome = dango_client.search_tx(outcome.tx_hash).await?;
    println!("outcome: {:#?}", outcome);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    println!("Querying the validator set for the remote domain...");
    // Query the validator set for the remote domain
    let validator_set_after = dango_client
        .query_wasm_smart(
            app_cfg.addresses.hyperlane.ism,
            hyperlane_types::isms::multisig::QueryValidatorSetRequest {
                domain: evm_config.hyperlane_domain,
            },
            None,
        )
        .await?;
    println!("validator_set_after: {:#?}", validator_set_after);

    if validator_set_after.threshold == new_validator_set.threshold
        && validator_set_after.validators == new_validator_set.validators
    {
        println!("Validator set is now {:#?}.", validator_set_after);
    } else {
        return Err(anyhow::anyhow!(
            "Failed to set the mock validator set for the remote domain"
        ));
    }

    Ok(())
}
