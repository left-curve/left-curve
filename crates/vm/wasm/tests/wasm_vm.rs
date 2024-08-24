use {
    grug_crypto::{sha2_256, sha2_512, Identity256, Identity512},
    grug_tester::{
        QueryEd25519BatchVerifyRequest, QueryRecoverSepc256k1Request, QueryVerifyEd25519Request,
        QueryVerifySecp256k1Request, QueryVerifySecp256r1Request,
    },
    grug_testing::TestBuilder,
    grug_types::{
        Binary, ByteArray, Coins, Hash256, Hash512, JsonSerExt, Message, MultiplyFraction, NonZero,
        NumberConst, QueryRequest, Udec128, Uint256,
    },
    grug_vm_wasm::{VmError, WasmVm},
    rand::rngs::OsRng,
    serde::{de::DeserializeOwned, Serialize},
    std::{collections::BTreeMap, fmt::Debug, fs, io, str::FromStr, vec},
    test_case::test_case,
};

const WASM_CACHE_CAPACITY: usize = 10;
const DENOM: &str = "ugrug";
const FEE_RATE: &str = "0.1";

fn read_wasm_file(filename: &str) -> io::Result<Binary> {
    let path = format!("{}/testdata/{filename}", env!("CARGO_MANIFEST_DIR"));
    fs::read(path).map(Into::into)
}

#[test]
fn bank_transfers() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(300_000_u128)))?
        .add_account("receiver", Coins::new())?
        .set_owner("owner")?
        .set_fee_denom(DENOM)
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .build()?;

    // Check that sender has been given 300,000 ugrug.
    // Sender needs to have sufficient tokens to cover gas fee and the transfers.
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(Uint256::from(300_000_u128));
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint256::ZERO);

    // Sender sends 70 ugrug to the receiver across multiple messages
    let outcome = suite.send_messages_with_gas(&accounts["sender"], 2_500_000, vec![
        Message::Transfer {
            to: accounts["receiver"].address,
            coins: Coins::one(DENOM, NonZero::new(10_u128)),
        },
        Message::Transfer {
            to: accounts["receiver"].address,
            coins: Coins::one(DENOM, NonZero::new(15_u128)),
        },
        Message::Transfer {
            to: accounts["receiver"].address,
            coins: Coins::one(DENOM, NonZero::new(20_u128)),
        },
        Message::Transfer {
            to: accounts["receiver"].address,
            coins: Coins::one(DENOM, NonZero::new(25_u128)),
        },
    ])?;

    outcome.result.should_succeed();

    // Sender remaining balance should be 300k - 70 - withhold + (withhold - charge).
    // = 300k - 70 - charge
    let fee = Uint256::from(outcome.gas_used).checked_mul_dec_ceil(Udec128::from_str(FEE_RATE)?)?;
    let sender_balance_after = Uint256::from(300_000_u128 - 70) - fee;

    // Check balances again
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(sender_balance_after);
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint256::from(70_u128));

    let info = suite.query_info().should_succeed();

    // List all holders of the denom
    suite
        .query_wasm_smart(info.config.bank, grug_bank::QueryHoldersRequest {
            denom: DENOM.to_string(),
            start_after: None,
            limit: None,
        })
        .should_succeed_and_equal(BTreeMap::from([
            (accounts["owner"].address, fee),
            (accounts["sender"].address, sender_balance_after),
            (accounts["receiver"].address, Uint256::from(70_u128)),
        ]));

    Ok(())
}

#[test]
fn gas_limit_too_low() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(200_000_u128)))?
        .add_account("receiver", Coins::new())?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .build()?;

    // Make a bank transfer with a small gas limit; should fail.
    // Bank transfers should take around ~1M gas.
    //
    // We can't easily tell whether gas will run out during the Wasm execution
    // (in which case, the error would be a `VmError::GasDepletion`) or during
    // a host function call (in which case, a `VmError::OutOfGas`). We can only
    // say that the error has to be one of the two. Therefore, we simply ensure
    // the error message contains the word "gas".
    let outcome = suite.send_message_with_gas(&accounts["sender"], 100_000, Message::Transfer {
        to: accounts["receiver"].address,
        coins: Coins::one(DENOM, NonZero::new(10_u128)),
    })?;

    outcome.result.should_fail();

    // The transfer should have failed, but gas fee already spent is still charged.
    let fee = Uint256::from(outcome.gas_used).checked_mul_dec_ceil(Udec128::from_str(FEE_RATE)?)?;
    let sender_balance_after = Uint256::from(200_000_u128) - fee;

    // Tx is went out of gas.
    // Balances should remain the same
    suite
        .query_balance(&accounts["sender"], DENOM)
        .should_succeed_and_equal(sender_balance_after);
    suite
        .query_balance(&accounts["receiver"], DENOM)
        .should_succeed_and_equal(Uint256::ZERO);

    Ok(())
}

#[test]
fn infinite_loop() -> anyhow::Result<()> {
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
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(32_100_000_u128)))?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .build()?;

    // Deploy the tester contract
    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["sender"],
        // Currently, deploying a contract consumes an exceedingly high amount
        // of gas because of the need to allocate hundreds ok kB of contract
        // bytecode into Wasm memory and have the contract deserialize it...
        320_000_000,
        read_wasm_file("grug_tester.wasm")?,
        "tester",
        &grug_tester::InstantiateMsg {},
        Coins::new(),
    )?;

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
        .should_fail_with_error(VmError::ReadOnly);

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
        .should_fail_with_error(VmError::ReadOnly);

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
    R: Clone + QueryRequest,
    R::Message: Serialize,
    R::Response: DeserializeOwned + Debug,
{
    let (mut suite, accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(32_100_000_u128)))?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .set_tracing_level(None)
        .build()?;

    // Deploy the tester contract
    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["sender"],
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
        .add_account("owner", Coins::new())?
        .add_account("sender", Coins::one(DENOM, NonZero::new(32_100_000_u128)))?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .set_tracing_level(None)
        .build()?;

    // Deploy the tester contract
    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["sender"],
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

    // Different msg, succeed but pk is different
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
        .add_account("sender", Coins::one(DENOM, NonZero::new(32_100_000_u128)))?
        .set_owner("owner")?
        .set_fee_rate(Udec128::from_str(FEE_RATE)?)
        .set_tracing_level(None)
        .build()?;

    // Deploy the tester contract
    let (_, tester) = suite.upload_and_instantiate_with_gas(
        &accounts["sender"],
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

    let mut query_msg = QueryEd25519BatchVerifyRequest {
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

    // Revert sign
    {
        query_msg.sigs.reverse();
        suite
            .query_wasm_smart(tester, query_msg)
            .should_fail_with_error("signature is unauthentic");
    }

    Ok(())
}
