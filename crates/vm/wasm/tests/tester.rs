use {
    grug_app::AppError,
    grug_crypto::{sha2_256, sha2_512, Identity256, Identity512},
    grug_tester::{
        QueryRecoverSepc256k1Request, QueryVerifyEd25519BatchRequest, QueryVerifyEd25519Request,
        QueryVerifySecp256k1Request, QueryVerifySecp256r1Request,
    },
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{
        Addr, Binary, ByteArray, Coins, Hash256, Hash512, JsonSerExt, Message, NonZero,
        QueryRequest, Udec128,
    },
    grug_vm_wasm::{VmError, WasmVm},
    rand::rngs::OsRng,
    serde::{de::DeserializeOwned, Serialize},
    std::{fmt::Debug, fs, io, str::FromStr, vec},
    test_case::test_case,
};

const WASM_CACHE_CAPACITY: usize = 10;
const DENOM: &str = "ugrug";
const FEE_RATE: &str = "0.1";

fn read_wasm_file(filename: &str) -> io::Result<Binary> {
    let path = format!("{}/testdata/{filename}", env!("CARGO_MANIFEST_DIR"));
    fs::read(path).map(Into::into)
}

fn setup_test() -> anyhow::Result<(TestSuite<WasmVm>, TestAccounts, Addr)> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(32_100_000_u128)))?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .build()?;

    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["sender"],
        320_000_000,
        read_wasm_file("grug_tester.wasm")?,
        "tester",
        &grug_tester::InstantiateMsg {},
        Coins::new(),
    )?;

    Ok((suite, accounts, tester))
}

#[test]
fn infinite_loop() -> anyhow::Result<()> {
    let (mut suite, accounts, tester) = setup_test()?;

    suite
        .send_message_with_gas(&accounts["sender"], 1_000_000, Message::Execute {
            contract: tester,
            msg: grug_tester::ExecuteMsg::InfiniteLoop {}.to_json_value()?,
            funds: Coins::new(),
        })?
        .result
        .should_fail_with_error("out of gas");

    Ok(())
}

#[test]
fn immutable_state() -> anyhow::Result<()> {
    let (mut suite, accounts, tester) = setup_test()?;

    // Query the tester contract.
    //
    // During the query, the contract attempts to write to the state by directly
    // calling the `db_write` import.
    //
    // This tests how the VM handles state mutability while serving the `Query`
    // ABCI request.
    suite
        .query_wasm_smart(tester, grug_tester::QueryForceWriteRequest {
            key: "larry".to_string(),
            value: "engineer".to_string(),
        })
        .should_fail_with_error(VmError::ImmutableState);

    // Execute the tester contract.
    //
    // During the execution, the contract makes a query to itself and the query
    // tries to write to the storage.
    //
    // This tests how the VM handles state mutability while serving the
    // `FinalizeBlock` ABCI request.
    suite
        .send_message_with_gas(&accounts["sender"], 1_000_000, Message::Execute {
            contract: tester,
            msg: grug_tester::ExecuteMsg::ForceWriteOnQuery {
                key: "larry".to_string(),
                value: "engineer".to_string(),
            }
            .to_json_value()?,
            funds: Coins::new(),
        })?
        .result
        .should_fail_with_error(VmError::ImmutableState);

    Ok(())
}

#[test]
fn query_stack_overflow() -> anyhow::Result<()> {
    let (suite, _, tester) = setup_test()?;

    // The contract attempts to call with `QueryMsg::StackOverflow` to itself in
    // a loop. Should raise the "exceeded max query depth" error.
    suite
        .query_wasm_smart(tester, grug_tester::QueryStackOverflowRequest {})
        .should_fail_with_error(VmError::ExceedMaxQueryDepth);

    Ok(())
}

#[test]
fn message_stack_overflow() -> anyhow::Result<()> {
    let (mut suite, accounts, tester) = setup_test()?;

    // The contract attempts to return a Response with `Execute::StackOverflow`
    // to itself in a loop. Should raise the "exceeded max message depth" error.
    suite
        .send_message_with_gas(
            &accounts["sender"],
            10_000_000,
            Message::execute(
                tester,
                &grug_tester::ExecuteMsg::StackOverflow {},
                Coins::default(),
            )?,
        )?
        .result
        .should_fail_with_error(AppError::ExceedMaxMessageDepth);

    Ok(())
}

const MSG: &[u8] = b"finger but hole";
const WRONG_MSG: &[u8] = b"precious item ahead";

fn secp256k1() -> (
    QueryVerifySecp256k1Request,
    fn(&mut QueryVerifySecp256k1Request, &[u8]),
) {
    use k256::ecdsa::{signature::DigestSigner, Signature, SigningKey, VerifyingKey};

    let sk = SigningKey::random(&mut OsRng);
    let vk = VerifyingKey::from(&sk);
    let msg_hash = Identity256::from(sha2_256(MSG));
    let sig: Signature = sk.sign_digest(msg_hash.clone());

    (
        QueryVerifySecp256k1Request {
            pk: vk.to_sec1_bytes().to_vec().into(),
            sig: sig.to_bytes().to_vec().try_into().unwrap(),
            msg_hash: msg_hash.into_bytes().into(),
        },
        |request, with| {
            request.msg_hash = Hash256::from_array(sha2_256(with));
        },
    )
}

fn secp256r1() -> (
    QueryVerifySecp256r1Request,
    fn(&mut QueryVerifySecp256r1Request, &[u8]),
) {
    use p256::ecdsa::{signature::DigestSigner, Signature, SigningKey, VerifyingKey};

    let sk = SigningKey::random(&mut OsRng);
    let vk = VerifyingKey::from(&sk);
    let msg_hash = Identity256::from(sha2_256(MSG));
    let sig: Signature = sk.sign_digest(msg_hash.clone());

    (
        QueryVerifySecp256r1Request {
            pk: vk.to_sec1_bytes().to_vec().into(),
            sig: sig.to_bytes().to_vec().try_into().unwrap(),
            msg_hash: msg_hash.into_bytes().into(),
        },
        |request, with| {
            request.msg_hash = Hash256::from_array(sha2_256(with));
        },
    )
}

fn ed25519() -> (
    QueryVerifyEd25519Request,
    fn(&mut QueryVerifyEd25519Request, &[u8]),
) {
    use ed25519_dalek::{DigestSigner, SigningKey, VerifyingKey};

    let sk = SigningKey::generate(&mut OsRng);
    let vk = VerifyingKey::from(&sk);
    let msg_hash = Identity512::from(sha2_512(MSG));
    let sig = sk.sign_digest(msg_hash.clone());

    (
        QueryVerifyEd25519Request {
            pk: (*vk.as_bytes()).into(),
            sig: sig.to_bytes().into(),
            msg_hash: msg_hash.into_bytes().into(),
        },
        |request, with| {
            request.msg_hash = Hash512::from_array(sha2_512(with));
        },
    )
}

#[test_case(secp256k1; "wasm_secp256k1")]
#[test_case(secp256r1; "wasm_secp256r1")]
#[test_case(ed25519; "wasm_ed25519")]
fn export_crypto_verify<R>(clos: fn() -> (R, fn(&mut R, &[u8]))) -> anyhow::Result<()>
where
    R: QueryRequest + Clone,
    R::Message: Serialize,
    R::Response: DeserializeOwned + Debug,
{
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::one(DENOM, NonZero::new(32_100_000_u128)))?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .set_tracing_level(None)
        .build()?;

    // Deploy the tester contract
    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["owner"],
        // Currently, deploying a contract consumes an exceedingly high amount
        // of gas because of the need to allocate hundreds ok kB of contract
        // bytecode into Wasm memory and have the contract deserialize it...
        320_000_000,
        read_wasm_file("grug_tester.wasm")?,
        "tester",
        &grug_tester::InstantiateMsg {},
        Coins::new(),
    )?;

    let (mut query_msg, update_fn) = clos();

    // Ok
    {
        suite
            .query_wasm_smart(tester, query_msg.clone())
            .should_succeed();
    }

    // Err, signature is unauthentic
    {
        update_fn(&mut query_msg, WRONG_MSG);

        suite
            .query_wasm_smart(tester, query_msg.clone())
            .should_fail_with_error("signature is unauthentic");
    }

    Ok(())
}

#[test]
fn wasm_secp256k1_pubkey_recover() -> anyhow::Result<()> {
    use k256::ecdsa::{SigningKey, VerifyingKey};

    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::one(DENOM, NonZero::new(32_100_000_u128)))?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .set_tracing_level(None)
        .build()?;

    // Deploy the tester contract
    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["owner"],
        // Currently, deploying a contract consumes an exceedingly high amount
        // of gas because of the need to allocate hundreds ok kB of contract
        // bytecode into Wasm memory and have the contract deserialize it...
        320_000_000,
        read_wasm_file("grug_tester.wasm")?,
        "tester",
        &grug_tester::InstantiateMsg {},
        Coins::new(),
    )?;

    // Generate a valid signature
    let sk = SigningKey::random(&mut OsRng);
    let vk = VerifyingKey::from(&sk);
    let msg_hash = Identity256::from(sha2_256(MSG));
    let (sig, recovery_id) = sk.sign_digest_recoverable(msg_hash.clone()).unwrap();

    let mut query_msg = QueryRecoverSepc256k1Request {
        sig: sig.to_vec().try_into().unwrap(),
        msg_hash: Hash256::from_array(msg_hash.into_bytes()),
        recovery_id: recovery_id.to_byte(),
        compressed: true,
    };

    // Ok
    {
        let pk = suite
            .query_wasm_smart(tester, query_msg.clone())
            .should_succeed();

        assert_eq!(pk.to_vec(), vk.to_sec1_bytes().to_vec());
    }

    // Attempt to recover with a different msg. Should succeed but pk is different.
    {
        query_msg.msg_hash = sha2_256(WRONG_MSG).into();

        let pk = suite
            .query_wasm_smart(tester, query_msg.clone())
            .should_succeed();

        assert_ne!(pk.to_vec(), vk.to_sec1_bytes().to_vec());
    }

    Ok(())
}

#[test]
fn wasm_ed25519_batch_verify() -> anyhow::Result<()> {
    use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

    fn ed25519_sign(msg: &str) -> (Binary, ByteArray<64>, ByteArray<32>) {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let sig = sk.sign(msg.as_bytes());
        (
            msg.as_bytes().to_vec().into(),
            sig.to_bytes().into(),
            vk.to_bytes().into(),
        )
    }

    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .set_tracing_level(None)
        .build()?;

    // Deploy the tester contract
    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["owner"],
        // Currently, deploying a contract consumes an exceedingly high amount
        // of gas because of the need to allocate hundreds ok kB of contract
        // bytecode into Wasm memory and have the contract deserialize it...
        320_000_000,
        read_wasm_file("grug_tester.wasm")?,
        "tester",
        &grug_tester::InstantiateMsg {},
        Coins::new(),
    )?;

    let (prehash_msg1, sig1, vk1) = ed25519_sign("Jake");
    let (prehash_msg2, sig2, vk2) = ed25519_sign("Larry");
    let (prehash_msg3, sig3, vk3) = ed25519_sign("Rhaki");

    let mut query_msg = QueryVerifyEd25519BatchRequest {
        prehash_msgs: vec![prehash_msg1, prehash_msg2, prehash_msg3],
        sigs: vec![sig1, sig2, sig3],
        pks: vec![vk1, vk2, vk3],
    };

    // Ok
    {
        suite
            .query_wasm_smart(tester, query_msg.clone())
            .should_succeed();
    }

    // Create an invalid batch simply by shuffling the order of signatures.
    {
        query_msg.sigs.reverse();

        suite
            .query_wasm_smart(tester, query_msg)
            .should_fail_with_error("signature is unauthentic");
    }

    Ok(())
}
