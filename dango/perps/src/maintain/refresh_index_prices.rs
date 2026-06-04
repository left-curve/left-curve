use {
    crate::{index_price::process_index_price, oracle, state::LAST_INDEX_PRICE_UPDATE},
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    grug_types::{MutableCtx, QuerierExt, Response},
};

pub fn refresh_index_prices(ctx: MutableCtx) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.contract || ctx.sender == ctx.querier.query_owner()?,
        "only the perps contract itself or the chain owner may refresh index prices"
    );

    ensure!(
        {
            let last_update = LAST_INDEX_PRICE_UPDATE.may_load(ctx.storage)?.unwrap_or(0);
            ctx.block.height > last_update
        },
        "index prices already updated this block"
    );

    let oracle_addr = oracle(ctx.querier);
    let mut oracle_querier =
        OracleQuerier::new_remote(oracle_addr, ctx.querier, ctx.block.timestamp);

    process_index_price(ctx.storage, ctx.block.timestamp, &mut oracle_querier)?;

    LAST_INDEX_PRICE_UPDATE.save(ctx.storage, &ctx.block.height)?;

    Ok(Response::new())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::state::PAIR_IDS,
        dango_types::config::AppConfig,
        grug_types::{
            Addr, Coins, Config, Duration, MockContext, MockQuerier, Permission, Permissions,
            ResultExt,
        },
        std::collections::BTreeMap,
    };

    const CONTRACT: Addr = Addr::mock(0);
    const ORACLE: Addr = Addr::mock(1);
    const OWNER: Addr = Addr::mock(2);

    fn mock_config() -> Config {
        Config {
            owner: OWNER,
            bank: Addr::mock(3),
            taxman: Addr::mock(4),
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Nobody,
                instantiate: Permission::Nobody,
            },
            max_orphan_age: Duration::from_seconds(0),
        }
    }

    fn mock_querier() -> MockQuerier {
        MockQuerier::new()
            .with_app_config(AppConfig {
                addresses: dango_types::config::AppAddresses {
                    oracle: ORACLE,
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap()
            .with_config(mock_config())
    }

    #[test]
    fn rejects_unauthorized_sender() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(Addr::mock(99))
            .with_funds(Coins::default())
            .with_block_height(10);

        refresh_index_prices(ctx.as_mutable()).should_fail_with_error(
            "only the perps contract itself or the chain owner may refresh index prices",
        );
    }

    #[test]
    fn owner_can_trigger() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(OWNER)
            .with_funds(Coins::default())
            .with_block_height(10);

        PAIR_IDS
            .save(&mut ctx.storage, &Default::default())
            .unwrap();

        refresh_index_prices(ctx.as_mutable()).should_succeed();
    }

    #[test]
    fn contract_can_trigger() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(CONTRACT)
            .with_funds(Coins::default())
            .with_block_height(10);

        PAIR_IDS
            .save(&mut ctx.storage, &Default::default())
            .unwrap();

        refresh_index_prices(ctx.as_mutable()).should_succeed();
    }

    #[test]
    fn once_per_block() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(CONTRACT)
            .with_funds(Coins::default())
            .with_block_height(10);

        LAST_INDEX_PRICE_UPDATE.save(&mut ctx.storage, &10).unwrap();

        refresh_index_prices(ctx.as_mutable())
            .should_fail_with_error("index prices already updated this block");
    }
}
