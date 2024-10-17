use {
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{
        Addr, Coins, Config, ConfigUpdates, Empty, Json, Op, Response, ResultExt, StdResult,
    },
    grug_vm_rust::{ContractBuilder, ContractWrapper},
    std::collections::BTreeMap,
    test_case::test_case,
};

fn empty_contract() -> ContractWrapper {
    ContractBuilder::new(Box::new(|_, _: Empty| -> StdResult<Response> {
        Ok(Response::new())
    }))
    .build()
}

fn setup() -> (TestSuite, TestAccounts) {
    TestBuilder::new()
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::new())
        .set_owner("owner")
        .build()
}

#[test]
fn upload_same_code() {
    let (mut suite, mut accounts) = setup();
    let contract = empty_contract();

    suite
        .upload(&mut accounts["sender"], contract)
        .should_succeed();

    suite.
        upload(&mut accounts["sender"], contract)
        .should_fail_with_error("code with hash `35BE322D094F9D154A8ABA4733B8497F180353BD7AE7B0A15F90B586B549F28B` already exists");
}

#[test]
fn instantiate_same_address() {
    let (mut suite, mut accounts) = setup();
    let contract = empty_contract();

    let code_hash = suite
        .upload_and_instantiate(
            &mut accounts["sender"],
            contract,
            &Empty {},
            "salt",
            None::<String>,
            None,
            Coins::new(),
        )
        .should_succeed()
        .code_hash;

    suite
        .instantiate(
            &mut accounts["sender"],
            code_hash,
            &Empty {},
            "salt",
            None::<String>,
            None,
            Coins::new(),
        )
        .should_fail_with_error(
            "account with address `0xcf580adbd298e67ee43ce479b533aa6db6e4f7e4` already exists",
        );
}

#[test_case(
    "owner",
    None,
    None,
    Ok(()),
    |_, _| {};
    "configure with no updates and no app updates"
)]
#[test_case(
    "sender",
    None,
    None,
    Err("sender is not the owner!"),
    |_, _| {};
    "non owner"
)]
#[test_case(
    "owner",
    Some(ConfigUpdates{
         owner: Some(Addr::mock(1)),
         bank: Some(Addr::mock(2)),
         taxman: None,
         cronjobs: None,
         permissions: None
         }),
    None,
    Ok(()),
    |_, config| {
        assert!(config.owner == Addr::mock(1));
        assert!(config.bank == Addr::mock(2));
        // assert!(config.taxman == Addr::mock(3));
    };
    "non aowner"
)]
fn configure<F>(
    sender: &str,
    updates: Option<ConfigUpdates>,
    app_updates: Option<BTreeMap<String, Op<Json>>>,
    result: Result<(), &str>,
    callback: F,
) where
    F: FnOnce(BTreeMap<String, Json>, Config),
{
    let (mut suite, mut accounts) = setup();

    let configure_outcome = suite.configure(
        &mut accounts[sender],
        updates.unwrap_or_default(),
        app_updates.unwrap_or_default(),
    );

    match (result, &configure_outcome.result) {
        (Ok(()), Ok(_)) => {},
        (Err(err), Err(_)) => {
            configure_outcome.result.should_fail_with_error(err);
        },
        (Ok(()), Err(err)) => {
            panic!("expected success, but got error: {}", err);
        },
        (Err(err), Ok(_)) => {
            panic!("expected error, but got success: {}", err);
        },
    }

    let app_configs = suite.query_app_configs().should_succeed();
    let config = suite.query_config().should_succeed();

    callback(app_configs, config);
}
