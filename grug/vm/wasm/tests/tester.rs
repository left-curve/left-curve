use {
    grug_app::AppError,
    grug_crypto::{Identity256, Identity512, sha2_256, sha2_512},
    grug_db_memory::MemDb,
    grug_math::Udec128,
    grug_tester::{
        QueryRecoverSecp256k1Request, QueryVerifyEd25519BatchRequest, QueryVerifyEd25519Request,
        QueryVerifySecp256k1Request, QueryVerifySecp256r1Request,
    },
    grug_testing::{TestAccounts, TestBuilder, TestSuite},
    grug_types::{
        Addr, Binary, Coins, Denom, GenericResult, InnerMut, Message, QuerierExt, QueryRequest,
        ResultExt, VerificationError,
    },
    grug_vm_wasm::{VmError, WasmVm},
    rand::rngs::OsRng,
    serde::{Serialize, de::DeserializeOwned},
    std::{fmt::Debug, fs, str::FromStr, sync::LazyLock, vec},
    test_case::test_case,
};

const WASM_CACHE_CAPACITY: usize = 10;

const FEE_RATE: Udec128 = Udec128::new_percent(10);

static DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("ugrug").unwrap());

fn read_wasm_file(filename: &str) -> Binary {
    let path = format!("{}/testdata/{filename}", env!("CARGO_MANIFEST_DIR"));
    fs::read(path).unwrap().into()
}

fn setup_test() -> (TestSuite<MemDb, WasmVm>, TestAccounts, Addr) {
    let (mut suite, mut accounts) = TestBuilder::new_with_vm(WasmVm::new(WASM_CACHE_CAPACITY))
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(DENOM.clone(), 32_100_000).unwrap())
        .set_owner("owner")
        .set_fee_rate(FEE_RATE)
        .build();

    let tester = suite
        .upload_and_instantiate_with_gas(
            &mut accounts["sender"],
            320_000_000,
            read_wasm_file("grug_tester.wasm"),
            &grug_tester::InstantiateMsg {},
            "tester",
            Some("tester"),
            None,
            Coins::new(),
        )
        .should_succeed()
        .address;

    (suite, accounts, tester)
}

#[test]
fn infinite_loop() {
    let (mut suite, mut accounts, tester) = setup_test();

    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            1_000_000,
            Message::execute(
                tester,
                &grug_tester::ExecuteMsg::InfiniteLoop {},
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error("out of gas");
}

#[test]
fn immutable_state() {
    let (mut suite, mut accounts, tester) = setup_test();

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
        .send_message_with_gas(
            &mut accounts["sender"],
            2_000_000,
            Message::execute(
                tester,
                &grug_tester::ExecuteMsg::ForceWriteOnQuery {
                    key: "larry".to_string(),
                    value: "engineer".to_string(),
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error(VmError::ImmutableState);
}

#[test]
fn query_stack_overflow() {
    let (suite, _, tester) = setup_test();

    // The contract attempts to call with `QueryMsg::StackOverflow` to itself in
    // a loop. Should raise the "exceeded max query depth" error.
    suite
        .query_wasm_smart(tester, grug_tester::QueryStackOverflowRequest {})
        .should_fail_with_error(VmError::ExceedMaxQueryDepth);
}

#[test]
fn message_stack_overflow() {
    let (mut suite, mut accounts, tester) = setup_test();

    // The contract attempts to return a Response with `Execute::StackOverflow`
    // to itself in a loop. Should raise the "exceeded max message depth" error.
    suite
        .send_message_with_gas(
            &mut accounts["sender"],
            10_000_000,
            Message::execute(
                tester,
                &grug_tester::ExecuteMsg::StackOverflow {},
                Coins::default(),
            )
            .unwrap(),
        )
        .should_fail_with_error(AppError::ExceedMaxMessageDepth);
}

// ------------------------------- crypto tests --------------------------------

const MSG: &[u8] = b"finger but hole";
const WRONG_MSG: &[u8] = b"precious item ahead";

fn generate_secp256r1_verify_request() -> QueryVerifySecp256r1Request {
    use p256::ecdsa::{Signature, SigningKey, VerifyingKey, signature::DigestSigner};

    let sk = SigningKey::random(&mut OsRng);
    let vk = VerifyingKey::from(&sk);
    let msg_hash = Identity256::from(sha2_256(MSG));
    let sig: Signature = sk.sign_digest(msg_hash.clone());

    QueryVerifySecp256r1Request {
        pk: vk.to_sec1_bytes().to_vec().into(),
        sig: sig.to_bytes().to_vec().into(),
        msg_hash: msg_hash.into_bytes().into(),
    }
}

fn generate_secp256k1_verify_request() -> QueryVerifySecp256k1Request {
    use k256::ecdsa::{Signature, SigningKey, VerifyingKey, signature::DigestSigner};

    let sk = SigningKey::random(&mut OsRng);
    let vk = VerifyingKey::from(&sk);
    let msg_hash = Identity256::from(sha2_256(MSG));
    let sig: Signature = sk.sign_digest(msg_hash.clone());

    QueryVerifySecp256k1Request {
        pk: vk.to_sec1_bytes().to_vec().into(),
        sig: sig.to_bytes().to_vec().into(),
        msg_hash: msg_hash.into_bytes().into(),
    }
}

fn generate_ed25519_verify_request() -> QueryVerifyEd25519Request {
    use ed25519_dalek::{DigestSigner, SigningKey, VerifyingKey};

    let sk = SigningKey::generate(&mut OsRng);
    let vk = VerifyingKey::from(&sk);
    let msg_hash = Identity512::from(sha2_512(MSG));
    let sig = sk.sign_digest(msg_hash.clone());

    QueryVerifyEd25519Request {
        pk: vk.as_bytes().to_vec().into(),
        sig: sig.to_bytes().into(),
        msg_hash: msg_hash.into_bytes().into(),
    }
}

// ----- Secp256r1 -----
#[test_case(
    generate_secp256r1_verify_request,
    |req| req,
    GenericResult::Ok(());
    "valid secp256r1 signature"
)]
#[test_case(
    generate_secp256r1_verify_request,
    |mut req| {
        req.pk.inner_mut().pop();
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid secp256r1: incorrect pk length"
)]
#[test_case(
    generate_secp256r1_verify_request,
    |mut req| {
        req.sig.inner_mut().pop();
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid secp256r1: incorrect signature length"
)]
#[test_case(
    generate_secp256r1_verify_request,
    |mut req| {
        req.msg_hash.inner_mut().pop();
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid secp256r1: incorrect msg hash length"
)]
#[test_case(
    generate_secp256r1_verify_request,
    |mut req| {
        let sk = p256::ecdsa::SigningKey::random(&mut OsRng);
        let vk = p256::ecdsa::VerifyingKey::from(&sk);
        req.pk = vk.to_sec1_bytes().to_vec().into();
        req
    },
    GenericResult::Err(VerificationError::unauthentic().to_string());
    "invalid secp256r1: different pk"
)]
#[test_case(
    generate_secp256r1_verify_request,
    |mut req| {
        req.msg_hash = sha2_256(WRONG_MSG).to_vec().into();
        req
    },
    GenericResult::Err(VerificationError::unauthentic().to_string());
    "invalid secp256r1: different msg"
)]
// ----- Secp256k1 -----
#[test_case(
    generate_secp256k1_verify_request,
    |req| req,
    GenericResult::Ok(());
    "valid secp256k1 signature"
)]
#[test_case(
    generate_secp256k1_verify_request,
    |mut req| {
        req.pk.inner_mut().push(0);
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid secp256k1: incorrect pk length"
)]
#[test_case(
    generate_secp256k1_verify_request,
    |mut req| {
        req.sig.inner_mut().push(0);
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid secp256k1: incorrect signature length"
)]
#[test_case(
    generate_secp256k1_verify_request,
    |mut req| {
        req.msg_hash.inner_mut().push(0);
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid secp256k1: incorrect msg hash length"
)]
#[test_case(
    generate_secp256k1_verify_request,
    |mut req| {
        let sk = k256::ecdsa::SigningKey::random(&mut OsRng);
        let vk = k256::ecdsa::VerifyingKey::from(&sk);
        req.pk = vk.to_sec1_bytes().to_vec().into();
        req
    },
    GenericResult::Err(VerificationError::unauthentic().to_string());
    "invalid secp256k1: different pk"
)]
#[test_case(
    generate_secp256k1_verify_request,
    |mut req| {
        req.msg_hash = sha2_256(WRONG_MSG).to_vec().into();
        req
    },
    GenericResult::Err(VerificationError::unauthentic().to_string());
    "invalid secp256k1: different msg"
)]
// ----- Ed25519 -----
#[test_case(
    generate_ed25519_verify_request,
    |req| req,
    GenericResult::Ok(());
    "valid ed25519 signature"
)]
#[test_case(
    generate_ed25519_verify_request,
    |mut req| {
        req.pk.inner_mut().push(123);
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid ed25519: incorrect pk length"
)]
#[test_case(
    generate_ed25519_verify_request,
    |mut req| {
        req.sig.inner_mut().push(123);
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid ed25519: incorrect signature length"
)]
#[test_case(
    generate_ed25519_verify_request,
    |mut req| {
        req.msg_hash.inner_mut().push(123);
        req
    },
    GenericResult::Err(VerificationError::incorrect_length().to_string());
    "invalid ed25519: incorrect msg hash length"
)]
#[test_case(
    generate_ed25519_verify_request,
    |mut req| {
        let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let vk = ed25519_dalek::VerifyingKey::from(&sk);
        req.pk = vk.as_bytes().to_vec().into();
        req
    },
        GenericResult::Err(VerificationError::unauthentic().to_string());
    "invalid ed25519: different pk"
)]
#[test_case(
    generate_ed25519_verify_request,
    |mut req| {
        req.msg_hash = sha2_512(WRONG_MSG).to_vec().into();
        req
    },
    GenericResult::Err(VerificationError::unauthentic().to_string());
    "invalid ed25519: different msg"
)]
fn verifying_signature<G, M, R>(
    generate_request: G,
    malleate: M,
    expect: GenericResult<R::Response>,
) where
    G: FnOnce() -> R,
    M: FnOnce(R) -> R,
    R: QueryRequest,
    R::Message: Serialize,
    R::Response: DeserializeOwned + Debug + PartialEq,
{
    let (suite, _, tester) = setup_test();

    // Generate and malleate query request
    let req = generate_request();
    let req = malleate(req);

    suite.query_wasm_smart(tester, req).should_match(expect);
}

#[test]
fn recovering_secp256k1_pubkey() {
    let (suite, _, tester) = setup_test();

    // Generate a valid signature
    let (vk, req) = {
        use k256::ecdsa::{SigningKey, VerifyingKey};

        let sk = SigningKey::random(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let msg_hash = Identity256::from(sha2_256(MSG));
        let (sig, recovery_id) = sk.sign_digest_recoverable(msg_hash.clone()).unwrap();

        (vk, QueryRecoverSecp256k1Request {
            sig: sig.to_vec().into(),
            msg_hash: msg_hash.into_bytes().to_vec().into(),
            recovery_id: recovery_id.to_byte(),
            compressed: true,
        })
    };

    // Ok
    {
        suite
            .query_wasm_smart(tester, req.clone())
            .should_succeed_and_equal(Binary::from_inner(vk.to_sec1_bytes().to_vec()));
    }

    // Attempt to recover with a different msg. Should succeed but pk is different.
    {
        let mut false_req = req.clone();
        false_req.msg_hash = sha2_256(WRONG_MSG).into();

        suite
            .query_wasm_smart(tester, false_req)
            .should_succeed_but_not_equal(Binary::from_inner(vk.to_sec1_bytes().to_vec()));
    }

    // Attempt to recover with an invalid recovery ID. Should error.
    {
        let mut false_req = req;
        false_req.recovery_id = 123;

        suite
            .query_wasm_smart(tester, false_req)
            .should_fail_with_error(VerificationError::invalid_recovery_id());
    }
}

fn ed25519_sign(msg: &str) -> (Binary, Binary, Binary) {
    use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

    let sk = SigningKey::generate(&mut OsRng);
    let vk = VerifyingKey::from(&sk);
    let sig = sk.sign(msg.as_bytes());

    (
        msg.as_bytes().to_vec().into(),
        sig.to_bytes().into(),
        vk.to_bytes().into(),
    )
}

#[test]
fn wasm_ed25519_batch_verify() {
    let (suite, _, tester) = setup_test();

    let mut req = {
        let (prehash_msg1, sig1, vk1) = ed25519_sign("Jake");
        let (prehash_msg2, sig2, vk2) = ed25519_sign("Larry");
        let (prehash_msg3, sig3, vk3) = ed25519_sign("Rhaki");

        QueryVerifyEd25519BatchRequest {
            prehash_msgs: vec![prehash_msg1, prehash_msg2, prehash_msg3],
            sigs: vec![sig1, sig2, sig3],
            pks: vec![vk1, vk2, vk3],
        }
    };

    // Ok
    {
        suite.query_wasm_smart(tester, req.clone()).should_succeed();
    }

    // Create an invalid batch simply by shuffling the order of signatures.
    {
        req.sigs.reverse();

        suite
            .query_wasm_smart(tester, req)
            .should_fail_with_error(VerificationError::unauthentic());
    }
}
