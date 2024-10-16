use {
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{Coins, Empty, Response, ResultExt, StdResult},
    grug_vm_rust::{ContractBuilder, ContractWrapper},
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
