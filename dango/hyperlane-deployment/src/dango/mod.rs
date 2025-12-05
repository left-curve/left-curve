use {
    crate::config::dango::DangoConfig,
    alloy::primitives::Address,
    dango_client::{Secp256k1, SingleSigner},
    dango_types::{
        auth::Nonce,
        config::AppConfig,
        gateway::{self, Origin, Remote},
    },
    grug::{Addr, BroadcastClientExt, Coins, Defined, QueryClientExt, StdError},
    hyperlane_types::Addr32,
    indexer_client::HttpClient,
    std::collections::BTreeSet,
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
        .collect::<Result<BTreeSet<(Origin, Addr, Remote)>, StdError>>()?;

    println!("Setting routes on Dango gateway: {:#?}", routes);
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
    println!("done!");

    Ok(())
}
