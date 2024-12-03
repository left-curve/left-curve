use {
    dango_testing::setup_test_naive,
    dango_types::{account_factory::ExecuteMsg, auth::OtpKey},
    grug::{Addressable, Coin, Coins, Json, Op, ResultExt},
};

#[test]
fn otp() {
    let (mut suite, accounts, _codes, contracts) = setup_test_naive();

    let mut sender = accounts.owner;

    // create a otp key
    sender.with_random_otp();
    // disable it
    sender.use_otp = false;

    let otp_key = sender.opt.clone().unwrap().1;

    // Configure the OTP key at username level
    {
        suite
            .execute(
                &mut sender,
                contracts.account_factory,
                &ExecuteMsg::ConfigureUserOtp {
                    key: Op::Insert(OtpKey {
                        key: otp_key,
                        policy: Json::null(),
                    }),
                },
                Coins::default(),
            )
            .should_succeed();
    }

    // Send a tx without using OTP.
    // This should succeed since the account is not registered to use OTP.
    {
        suite
            .transfer(
                &mut sender,
                accounts.relayer.address(),
                Coin::new("uusdc", 100).unwrap(),
            )
            .should_succeed();
    }

    // Enable OTP for the account onchain.
    {
        suite
            .execute(
                &mut sender,
                contracts.account_factory,
                &ExecuteMsg::ConfigureAccountOtp { enabled: true },
                Coins::default(),
            )
            .should_succeed();
    }

    // Send a tx without using OTP.
    // This should fails (we are not signing with OTP enable on TestAccount).
    {
        suite
            .transfer(
                &mut sender,
                accounts.relayer.address(),
                Coin::new("uusdc", 100).unwrap(),
            )
            .should_fail_with_error("otp key and signature must be both present or both absent");

        sender.sequence -= 1;
    }

    // Send a tx with OTP enabling the OTP in the TestAccount.
    // This should succeed (enable in both TestAccount and onchain).
    {
        sender.use_otp = true;
        suite
            .transfer(
                &mut sender,
                accounts.relayer.address(),
                Coin::new("uusdc", 100).unwrap(),
            )
            .should_succeed();
    }

    // Disable OTP for the account onchain.
    {
        suite
            .execute(
                &mut sender,
                contracts.account_factory,
                &ExecuteMsg::ConfigureAccountOtp { enabled: false },
                Coins::default(),
            )
            .should_succeed();
    }

    // Send a tx using OTP in the TestAccount.
    // This should fails (we have disable onchain but not on TestAccount).
    {
        sender.use_otp = true;
        suite
            .transfer(
                &mut sender,
                accounts.relayer.address(),
                Coin::new("uusdc", 100).unwrap(),
            )
            .should_fail_with_error("otp key and signature must be both present or both absent");
        sender.sequence -= 1;
    }

    // Disable OTP in the TestAccount.
    // This should succeed (disabled in both TestAccount and onchain).
    {
        sender.use_otp = false;
        suite
            .execute(
                &mut sender,
                contracts.account_factory,
                &ExecuteMsg::ConfigureAccountOtp { enabled: false },
                Coins::default(),
            )
            .should_succeed();
    }

    // Renable OTP onchain.
    {
        suite
            .execute(
                &mut sender,
                contracts.account_factory,
                &ExecuteMsg::ConfigureAccountOtp { enabled: true },
                Coins::default(),
            )
            .should_succeed();
    }

    // Remove the OTP at username level.
    {
        sender.use_otp = true;
        suite
            .execute(
                &mut sender,
                contracts.account_factory,
                &ExecuteMsg::ConfigureUserOtp { key: Op::Delete },
                Coins::default(),
            )
            .should_succeed();
    }

    // Try send a tx with OTP enabled in the TestAccount.
    // This should fails. Removing for the username it should disable OTP
    // for each account that have OTP enabled.
    {
        suite
            .transfer(
                &mut sender,
                accounts.relayer.address(),
                Coin::new("uusdc", 100).unwrap(),
            )
            .should_fail_with_error("otp key and signature must be both present or both absent");
    }
}
