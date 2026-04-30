use {
    crate::{
        account_factory,
        referral::load_referral_data,
        state::{REFEREE_TO_REFERRER, REFERRER_TO_REFEREE_STATISTICS, USER_REFERRAL_DATA},
        volume::round_to_day,
    },
    anyhow::ensure,
    dango_types::{
        account_factory::{self, UserIndex},
        perps::{RefereeStats, ReferralSet},
    },
    grug::{MutableCtx, QuerierExt, Response},
};

/// Register a referral relationship between a referrer and a referee.
///
/// Caller must be one of:
///
/// - the account factory (during user registration, when the user provides
///   a referral code at sign-up);
/// - the chain's owner (admin override; still subject to every other
///   constraint below);
/// - an account owned by the referee (self-service after registration).
pub fn set_referral(
    ctx: MutableCtx,
    referrer: UserIndex,
    referee: UserIndex,
) -> anyhow::Result<Response> {
    // Referrer and referee must be different users.
    ensure!(referrer != referee, "a user cannot refer themselves");

    let account_factory = account_factory(ctx.querier);

    // Sender must be the account factory, the chain owner, or an account
    // owned by the referee. The owner lookup is lazy: short-circuit `&&`
    // skips the querier roundtrip when the cheaper comparison succeeds.
    if ctx.sender != account_factory && ctx.sender != ctx.querier.query_owner()? {
        // TODO: refactor to raw query (query_wasm_path).
        let account = ctx.querier.query_wasm_smart(
            account_factory,
            account_factory::QueryAccountRequest {
                address: ctx.sender,
            },
        )?;

        ensure!(
            account.owner == referee,
            "caller is not the account factory, chain owner, or an account owned by the referee"
        );
    }

    // The referral relationship is immutable once set.
    ensure!(
        !REFEREE_TO_REFERRER.has(ctx.storage, referee),
        "referee {referee} already has a referrer"
    );

    // Save the referee-to-referrer relation.
    REFEREE_TO_REFERRER.save(ctx.storage, referee, &referrer)?;

    // Initialize per-referee statistics for the referrer.
    REFERRER_TO_REFEREE_STATISTICS.save(ctx.storage, (referrer, referee), &RefereeStats {
        registered_at: ctx.block.timestamp,
        ..Default::default()
    })?;

    // Increment the referrer's referee count.
    {
        let today = round_to_day(ctx.block.timestamp);

        let mut data = load_referral_data(ctx.storage, referrer, None)?;
        data.referee_count += 1;

        USER_REFERRAL_DATA.save(ctx.storage, (referrer, today), &data)?;
    }

    Ok(Response::new().add_event(ReferralSet { referrer, referee })?)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::state::FEE_SHARE_RATIO,
        dango_types::{
            account_factory::Account,
            config::{AppAddresses, AppConfig},
            perps::FeeShareRatio,
        },
        grug::{
            Addr, Coins, Config, Duration, EventName, JsonDeExt, JsonSerExt, MockContext,
            MockQuerier, Permission, Permissions, ResultExt, Storage, Timestamp,
        },
        std::collections::BTreeMap,
    };

    const OWNER: Addr = Addr::mock(1);
    const ACCOUNT_FACTORY: Addr = Addr::mock(2);
    const REFEREE_ADDR: Addr = Addr::mock(10);
    const OUTSIDER_ADDR: Addr = Addr::mock(11);

    const REFERRER: UserIndex = 1;
    const REFEREE: UserIndex = 2;
    const OUTSIDER: UserIndex = 3;

    /// Non-zero block timestamp so `registered_at` and `round_to_day` produce
    /// deterministic, meaningful values in the post-condition test.
    const BLOCK_TIME: Timestamp = Timestamp::from_seconds(1_700_000_000);

    fn mock_config() -> Config {
        Config {
            owner: OWNER,
            bank: Addr::mock(100),
            taxman: Addr::mock(101),
            cronjobs: BTreeMap::new(),
            permissions: Permissions {
                upload: Permission::Everybody,
                instantiate: Permission::Everybody,
            },
            max_orphan_age: Duration::from_seconds(1000),
        }
    }

    /// Querier that exposes the chain owner and the account-factory address.
    /// Sufficient for every branch that doesn't fall through to the wasm
    /// smart query.
    fn base_querier() -> MockQuerier {
        MockQuerier::new()
            .with_config(mock_config())
            .with_app_config(AppConfig {
                addresses: AppAddresses {
                    account_factory: ACCOUNT_FACTORY,
                    ..Default::default()
                },
                ..Default::default()
            })
            .unwrap()
    }

    /// Extends `base_querier()` with a smart-query handler that answers any
    /// query to the account factory with an `Account` owned by
    /// `account_owner`. Used only by tests that reach the wasm smart query.
    fn querier_with_account_owner(account_owner: UserIndex) -> MockQuerier {
        base_querier().with_smart_query_handler(move |addr, _msg| {
            assert_eq!(addr, ACCOUNT_FACTORY, "unexpected smart query to {addr}");
            Ok(Account {
                index: 0,
                owner: account_owner,
            }
            .to_json_value()
            .unwrap())
        })
    }

    fn seed_referrer(storage: &mut dyn Storage) {
        FEE_SHARE_RATIO
            .save(storage, REFERRER, &FeeShareRatio::new_percent(25))
            .unwrap();
    }

    #[test]
    fn self_referral_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(base_querier())
            .with_sender(ACCOUNT_FACTORY)
            .with_funds(Coins::default());

        set_referral(ctx.as_mutable(), REFERRER, REFERRER)
            .should_fail_with_error("a user cannot refer themselves");
    }

    #[test]
    fn account_factory_can_set_referral() {
        let mut ctx = MockContext::new()
            .with_querier(base_querier())
            .with_sender(ACCOUNT_FACTORY)
            .with_funds(Coins::default());

        seed_referrer(&mut ctx.storage);

        set_referral(ctx.as_mutable(), REFERRER, REFEREE).should_succeed();
    }

    #[test]
    fn chain_owner_can_set_referral() {
        let mut ctx = MockContext::new()
            .with_querier(base_querier())
            .with_sender(OWNER)
            .with_funds(Coins::default());

        seed_referrer(&mut ctx.storage);

        set_referral(ctx.as_mutable(), REFERRER, REFEREE).should_succeed();
    }

    #[test]
    fn referee_own_account_can_set_referral() {
        let mut ctx = MockContext::new()
            .with_querier(querier_with_account_owner(REFEREE))
            .with_sender(REFEREE_ADDR)
            .with_funds(Coins::default());

        seed_referrer(&mut ctx.storage);

        set_referral(ctx.as_mutable(), REFERRER, REFEREE).should_succeed();
    }

    #[test]
    fn unauthorized_third_party_account_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(querier_with_account_owner(OUTSIDER))
            .with_sender(OUTSIDER_ADDR)
            .with_funds(Coins::default());

        seed_referrer(&mut ctx.storage);

        set_referral(ctx.as_mutable(), REFERRER, REFEREE).should_fail_with_error(
            "caller is not the account factory, chain owner, or an account owned by the referee",
        );
    }

    /// A user can be assigned as a referrer even when they have not chosen
    /// a fee share ratio — the missing ratio defaults to zero in
    /// `apply_fee_commissions`.
    #[test]
    fn referrer_without_fee_share_ratio_accepted() {
        let mut ctx = MockContext::new()
            .with_querier(base_querier())
            .with_sender(OWNER)
            .with_funds(Coins::default())
            .with_block_timestamp(BLOCK_TIME);

        // Deliberately do NOT seed FEE_SHARE_RATIO — the relationship should
        // still be created.

        set_referral(ctx.as_mutable(), REFERRER, REFEREE).should_succeed();

        assert_eq!(
            REFEREE_TO_REFERRER.load(&ctx.storage, REFEREE).unwrap(),
            REFERRER,
        );
    }

    /// Immutability still applies to the owner branch: a referee with an
    /// existing referrer cannot be overwritten, even by the chain owner.
    #[test]
    fn already_set_referee_rejected_for_owner() {
        let mut ctx = MockContext::new()
            .with_querier(base_querier())
            .with_sender(OWNER)
            .with_funds(Coins::default());

        seed_referrer(&mut ctx.storage);
        REFEREE_TO_REFERRER
            .save(&mut ctx.storage, REFEREE, &OUTSIDER)
            .unwrap();

        set_referral(ctx.as_mutable(), REFERRER, REFEREE)
            .should_fail_with_error("already has a referrer");
    }

    #[test]
    fn happy_path_writes_expected_state() {
        let mut ctx = MockContext::new()
            .with_querier(base_querier())
            .with_sender(OWNER)
            .with_funds(Coins::default())
            .with_block_timestamp(BLOCK_TIME);

        seed_referrer(&mut ctx.storage);

        let response = set_referral(ctx.as_mutable(), REFERRER, REFEREE).should_succeed();

        // Referee-to-referrer mapping saved.
        assert_eq!(
            REFEREE_TO_REFERRER.load(&ctx.storage, REFEREE).unwrap(),
            REFERRER,
        );

        // Per-referee stats initialized with the block timestamp.
        let stats = REFERRER_TO_REFEREE_STATISTICS
            .load(&ctx.storage, (REFERRER, REFEREE))
            .unwrap();
        assert_eq!(stats.registered_at, BLOCK_TIME);

        // Referrer's daily cumulative referral data incremented.
        let data = USER_REFERRAL_DATA
            .load(&ctx.storage, (REFERRER, round_to_day(BLOCK_TIME)))
            .unwrap();
        assert_eq!(data.referee_count, 1);

        // `ReferralSet` event emitted with the expected payload.
        let event = response
            .subevents
            .iter()
            .find(|e| e.ty == ReferralSet::EVENT_NAME)
            .expect("ReferralSet event missing");
        let payload: ReferralSet = event.data.clone().deserialize_json().unwrap();
        assert_eq!(payload.referrer, REFERRER);
        assert_eq!(payload.referee, REFEREE);
    }
}
