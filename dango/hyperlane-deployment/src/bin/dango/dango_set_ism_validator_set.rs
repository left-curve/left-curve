use {
    dango_client::{Secp256k1, Secret, SingleSigner},
    dango_types::config::AppConfig,
    grug::{
        Addr, BroadcastClientExt, Coins, GasOption, HexByteArray, QueryClientExt, SearchTxClient,
        addr,
    },
    hex_literal::hex,
    hyperlane_types::isms::multisig::ValidatorSet,
    indexer_client::HttpClient,
};

const CURRENT_OWNER: Addr = addr!("33361de42571d6aa20c37daa6da4b5ab67bfaad9");
const CURRENT_OWNER_USERNAME: &str = "owner";
const CURRENT_OWNER_PRIVATE_KEY: [u8; 32] =
    hex!("8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177");

const DANGO_API_URL: &str = "https://api-pr-1414-ovh2.dango.zone/";
const REMOTE_DOMAIN: u32 = 11155111;

const CHAIN_ID: &str = "pr-1414";

const REMOTE_VALIDATOR_SET: [HexByteArray<20>; 3] = [
    HexByteArray::from_inner(hex!("b22b65f202558adf86a8bb2847b76ae1036686a5")),
    HexByteArray::from_inner(hex!("469f0940684d147defc44f3647146cb90dd0bc8e")),
    HexByteArray::from_inner(hex!("d3c75dcf15056012a4d74c483a0c6ea11d8c2b83")),
];
const REMOTE_VALIDATOR_SET_THRESHOLD: u32 = 2;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dango_client = HttpClient::new(DANGO_API_URL)?;

    println!("querying contracts...");
    let app_cfg: AppConfig = dango_client.query_app_config(None).await?;
    println!("app_cfg: {:#?}", app_cfg);

    // Setup signer
    let mut signer = SingleSigner::new(
        CURRENT_OWNER_USERNAME,
        CURRENT_OWNER,
        Secp256k1::from_bytes(CURRENT_OWNER_PRIVATE_KEY)?,
    )?
    .with_query_nonce(&dango_client)
    .await?;

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

    let new_validator_set = ValidatorSet {
        threshold: REMOTE_VALIDATOR_SET_THRESHOLD,
        validators: REMOTE_VALIDATOR_SET.into_iter().collect(),
    };

    match validator_sets.get(&REMOTE_DOMAIN) {
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
        domain: REMOTE_DOMAIN,
        threshold: new_validator_set.threshold,
        validators: new_validator_set.validators.clone(),
    };
    let outcome = dango_client
        .execute(
            &mut signer,
            app_cfg.addresses.hyperlane.ism,
            &set_validators_msg,
            Coins::new(),
            GasOption::Predefined { gas_limit: 1000000 },
            CHAIN_ID,
        )
        .await?;

    let outcome = dango_client.search_tx(outcome.tx_hash).await?;
    println!("outcome: {:#?}", outcome);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    println!("Querying the validator set for the remote domain...");
    // Query the validator set for the remote domain
    let validator_set_after = dango_client
        .query_wasm_smart(
            app_cfg.addresses.hyperlane.ism,
            hyperlane_types::isms::multisig::QueryValidatorSetRequest {
                domain: REMOTE_DOMAIN,
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
