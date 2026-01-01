use {
    anyhow::{anyhow, ensure},
    dango_genesis::GenesisCodes,
    dango_types::account_factory::{self, Account, UserIndexOrName},
    grug::{
        Addr, BlockInfo, GENESIS_BLOCK_HASH, GENESIS_BLOCK_HEIGHT, GenesisState, JsonDeExt, Query,
        Timestamp,
    },
    grug_app::{App, NaiveProposalPreparer, NullIndexer, SimpleCommitment},
    grug_db_memory::MemDb,
    grug_vm_rust::RustVm,
    std::collections::BTreeMap,
};

const CHAIN_ID: &str = "dango-1";

const GENESIS_JSON: &str = r#"{
  "app_config": {
    "addresses": {
      "account_factory": "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b",
      "dex": "0xda32476efe31e535207f0ad690d337a4ebf54a22",
      "gateway": "0xc51e2cbe9636a90c86463ac3eb18fbee92b700d1",
      "hyperlane": {
        "ism": "0xdc68d6c82f5e4386294e7fda27317ab6ae8ff54c",
        "mailbox": "0x974e57564ed3ed7d8f99d0c359fd03f3d78259c7",
        "va": "0x75f38c6fcfc2fb8333e5c3ef89d13b7036abe3ff"
      },
      "oracle": "0xcedc5f73cbb963a48471b849c3650e6e34cd3b6d",
      "taxman": "0xda70a9c1417aee00f960fe896add9d571f9c365b",
      "warp": "0x981e6817442143ce5128992c7ab4a317321f00e9"
    },
    "maker_fee_rate": "0.0002",
    "minimum_deposit": {
      "bridge/usdc": "10000000"
    },
    "taker_fee_rate": "0.0005"
  },
  "config": {
    "bank": "0xe0b49f70991ecab05d5d7dc1f71e4ede63c8f2b7",
    "cronjobs": {
      "0xc51e2cbe9636a90c86463ac3eb18fbee92b700d1": "86400",
      "0xda32476efe31e535207f0ad690d337a4ebf54a22": "0"
    },
    "max_orphan_age": "604800",
    "owner": "0x149a2e2bc3ed63aeb0410416b9123d886af1f9cd",
    "permissions": {
      "instantiate": {
        "somebodies": [
          "0x18d28bafcdf9d4574f920ea004dea2d13ec16f6b"
        ]
      },
      "upload": "nobody"
    },
    "taxman": "0xda70a9c1417aee00f960fe896add9d571f9c365b"
  },
  "msgs": [
    {
      "upload": {
        "code": "AAAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "AQAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "AgAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "AwAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "BAAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "BQAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "BgAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "BwAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "CAAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "CQAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "CgAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "CwAAAAAAAAA="
      }
    },
    {
      "upload": {
        "code": "DAAAAAAAAAA="
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "AF5570F5A1810B7AF78CAF4BC70A660F0DF51E42BAF91D4DE5B2328DE0E83DFC",
        "funds": {},
        "label": "dango/account_factory",
        "msg": {
          "code_hashes": {
            "multi": "7C9FA136D4413FA6173637E883B6998D32E1D675F88CDDFF9DCBCF331820F4B8",
            "single": "D86E8112F3C4C4442126F8E9F44F16867DA487F29052BF91B810457DB34209A4"
          },
          "users": [
            {
              "key": {
                "secp256k1": "A2Uj5mWM1u9AL+r7Q7ZYekezdiOXgx5YIJM1yJ1m1RPq"
              },
              "key_hash": "4682F5BE77499C54F63BDEF6734F8E388275342BFD845322D9377A5C312B7F8B",
              "seed": 0
            }
          ]
        },
        "salt": "ZGFuZ28vYWNjb3VudF9mYWN0b3J5"
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "23D7F42B1CDC1F0D492EBD756ED0FE8003995DDA554D99418D47A81813650207",
        "funds": {},
        "label": "hyperlane/ism/multisig",
        "msg": {
          "validator_sets": {
            "1": {
              "threshold": 6,
              "validators": [
                "03c842db86a6a3e524d4a6615390c1ea8e2b9541",
                "94438a7de38d4548ae54df5c6010c4ebc5239eae",
                "5450447aee7b544c462c9352bef7cad049b0c2dc",
                "b3ac35d3988bca8c2ffd195b1c6bee18536b317b",
                "b683b742b378632a5f73a2a5a45801b3489bba44",
                "3786083ca59dc806d894104e65a13a70c2b39276",
                "4f977a59fdc2d9e39f6d780a84d5b4add1495a36",
                "29d783efb698f9a2d3045ef4314af1f5674f52c5",
                "36a669703ad0e11a0382b098574903d2084be22c"
              ]
            }
          }
        },
        "salt": "aHlwZXJsYW5lL2lzbS9tdWx0aXNpZw=="
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "220DDE27AFEE4D537CAC96D85D6546F825153B90E828931B74E103807541BC42",
        "funds": {},
        "label": "dango/warp",
        "msg": {
          "mailbox": "0x974e57564ed3ed7d8f99d0c359fd03f3d78259c7"
        },
        "salt": "ZGFuZ28vd2FycA=="
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "AAE89FC0F03E2959AE4D701A80CC3915918C950B159F6ABB6C92C1433B1A8534",
        "funds": {},
        "label": "hyperlane/mailbox",
        "msg": {
          "config": {
            "default_ism": "0xdc68d6c82f5e4386294e7fda27317ab6ae8ff54c",
            "local_domain": 88888888
          }
        },
        "salt": "aHlwZXJsYW5lL21haWxib3g="
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "6CC16ABD70EEFB90DC0BA0D14FB088630873B2C6AD943F7442356735984C35A3",
        "funds": {},
        "label": "hyperlane/va",
        "msg": {
          "announce_fee_per_byte": {
            "amount": "100",
            "denom": "bridge/usdc"
          },
          "mailbox": "0x974e57564ed3ed7d8f99d0c359fd03f3d78259c7"
        },
        "salt": "aHlwZXJsYW5lL3Zh"
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "F0A0278E4372459CCA6159CD5E71CFEE638302A7B9CA9B05C34181AC0A65AC5D",
        "funds": {},
        "label": "dango/dex",
        "msg": {
          "pairs": [
            {
              "base_denom": "bridge/eth",
              "params": {
                "bucket_sizes": [
                  "0.00000000000001",
                  "0.0000000000001",
                  "0.000000000001",
                  "0.00000000001",
                  "0.00000000005",
                  "0.0000000001"
                ],
                "lp_denom": "dex/pool/eth/usdc",
                "min_order_size_base": "500000000000000",
                "min_order_size_quote": "1000000",
                "pool_type": {
                  "geometric": {
                    "limit": 1,
                    "ratio": "1",
                    "spacing": "0.0001"
                  }
                },
                "swap_fee_rate": "0.0001"
              },
              "quote_denom": "bridge/usdc"
            }
          ]
        },
        "salt": "ZGFuZ28vZGV4"
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "F13EE6ED54EA2AAE9FC49A9FAEB5DA6E8DDEF0E12ED5D30D35A624AE813E0485",
        "funds": {},
        "label": "dango/gateway",
        "msg": {
          "rate_limits": {
            "bridge/eth": "0.1",
            "bridge/usdc": "0.1"
          },
          "routes": [],
          "withdrawal_fees": []
        },
        "salt": "ZGFuZ28vZ2F0ZXdheQ=="
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "35BE322D094F9D154A8ABA4733B8497F180353BD7AE7B0A15F90B586B549F28B",
        "funds": {},
        "label": "dango/bank",
        "msg": {
          "balances": {},
          "metadatas": {
            "bridge/eth": {
              "decimals": 18,
              "name": "Ether",
              "symbol": "ETH"
            },
            "bridge/usdc": {
              "decimals": 6,
              "name": "USD Coin",
              "symbol": "USDC"
            }
          },
          "namespaces": {
            "bridge": "0xc51e2cbe9636a90c86463ac3eb18fbee92b700d1",
            "dex": "0xda32476efe31e535207f0ad690d337a4ebf54a22"
          }
        },
        "salt": "ZGFuZ28vYmFuaw=="
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "A111F275CC2E7588000001D300A31E76336D15B9D314CD1A1D8F3D3556975EED",
        "funds": {},
        "label": "dango/taxman",
        "msg": {
          "config": {
            "fee_denom": "bridge/usdc",
            "fee_rate": "0"
          }
        },
        "salt": "ZGFuZ28vdGF4bWFu"
      }
    },
    {
      "instantiate": {
        "admin": "0xc4a8f7bbadd1457092a8cd182480230c0a848331",
        "code_hash": "CBBD5F990C53684D7AE650B40FCB5656E02261B53DA5F6A7D8C819C92F2828F8",
        "funds": {},
        "label": "dango/oracle",
        "msg": {
          "price_sources": {
            "bridge/eth": {
              "pyth": {
                "channel": "real_time",
                "id": 2,
                "precision": 18
              }
            },
            "bridge/usdc": {
              "pyth": {
                "channel": "real_time",
                "id": 7,
                "precision": 6
              }
            }
          },
          "trusted_signers": {
            "A6Q4DwETbrJkD5DBfh4xngK7r77vLm5n3EivU/mCfhVb": "340282366920938463463374607431.768211455"
          }
        },
        "salt": "ZGFuZ28vb3JhY2xl"
      }
    }
  ]
}"#;

fn main() -> anyhow::Result<()> {
    // Deserialize the genesis state.
    let genesis_state: GenesisState = GENESIS_JSON.deserialize_json()?;

    // Create grug app.
    let _codes = RustVm::genesis_codes();
    let vm = RustVm::new();
    let db = MemDb::<SimpleCommitment>::new();
    let app = App::new(
        db,
        vm,
        NaiveProposalPreparer,
        NullIndexer,
        u64::MAX,
        None,
        "0.0.0",
    );

    // Run genesis.
    app.do_init_chain(
        CHAIN_ID.to_string(),
        BlockInfo {
            height: GENESIS_BLOCK_HEIGHT,
            timestamp: Timestamp::ZERO, // doesn't matter for this dry run
            hash: GENESIS_BLOCK_HASH,
        },
        genesis_state,
    )?;

    // Ensure the owner's account is correct.
    let owner = app
        .do_query_app(Query::config(), None, false)?
        .into_config()
        .owner;
    let account_factory = app
        .do_query_app(Query::app_config(), None, false)?
        .into_app_config()
        .deserialize_json::<dango_types::config::AppConfig>()?
        .addresses
        .account_factory;
    let (user0, _) = app
        .do_query_app(
            Query::wasm_smart(
                account_factory,
                &account_factory::QueryMsg::AccountsByUser {
                    user: UserIndexOrName::Index(0),
                },
            )?,
            None,
            false,
        )?
        .into_wasm_smart()
        .deserialize_json::<BTreeMap<Addr, Account>>()?
        .pop_last()
        .ok_or(anyhow!("no account found for user index 0"))?;

    ensure!(
        owner == user0,
        "owner address does not equal user0! owner: {owner}, user0: {user0}"
    );

    println!("ok");

    Ok(())
}
