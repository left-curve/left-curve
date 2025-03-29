import { PrivateKeySigner, createSignerClient, devnet, graphql } from "@left-curve/dango";
import { deserializeJson } from "@left-curve/dango/encoding";
import "dotenv/config";

import type { Address, AppConfig } from "@left-curve/dango/types";

const main = async () => {
  try {
    if (!process.env.OWNER_KEY) throw new Error("error: owner key not found as env variable");

    const client = createSignerClient({
      transport: graphql(),
      username: "owner",
      chain: devnet,
      signer: PrivateKeySigner.fromMnemonic(process.env.OWNER_KEY),
    });

    const accounts = await client.getAccountsByUsername({ username: "owner" });
    const sender = Object.keys(accounts)[0] as Address;

    const { addresses } = await client.getAppConfig<AppConfig>();

    const { VALIDATORS } = process.env;
    if (!VALIDATORS) throw new Error("error: validators not found as env variable");

    const validators: { address: string }[] = deserializeJson(VALIDATORS);

    await client.execute({
      sender,
      execute: [
        {
          contract: addresses.hyperlane.ism as Address,
          msg: {
            set_validators: {
              domain: 123,
              threshold: 2,
              validators: validators.map((v) => v.address),
            },
          },
        },
        {
          contract: addresses.warp as Address,
          msg: {
            set_route: {
              denom: "hyp/eth/usdc",
              destination_domain: 123,
              route: {
                address: "0000000000000000000000000000000000000000000000000000000000000000",
                fee: "0",
              },
            },
          },
        },
        {
          contract: addresses.warp as Address,
          msg: {
            set_route: {
              denom: "hyp/sol/sol",
              destination_domain: 123,
              route: {
                address: "0000000000000000000000000000000000000000000000000000000000000001",
                fee: "0",
              },
            },
          },
        },
        {
          contract: addresses.warp as Address,
          msg: {
            set_alloy: {
              underlying_denom: "hyp/sol/sol",
              alloyed_denom: "hyp/all/sol",
              destination_domain: 123,
            },
          },
        },
        {
          contract: addresses.warp as Address,
          msg: {
            set_route: {
              denom: "hyp/eth/wbtc",
              destination_domain: 123,
              route: {
                address: "0000000000000000000000000000000000000000000000000000000000000002",
                fee: "0",
              },
            },
          },
        },
        {
          contract: addresses.warp as Address,
          msg: {
            set_alloy: {
              underlying_denom: "hyp/eth/wbtc",
              alloyed_denom: "hyp/all/wbtc",
              destination_domain: 123,
            },
          },
        },
        {
          contract: addresses.warp as Address,
          msg: {
            set_route: {
              denom: "hyp/xrp/xrp",
              destination_domain: 123,
              route: {
                address: "0000000000000000000000000000000000000000000000000000000000000003",
                fee: "0",
              },
            },
          },
        },
        {
          contract: addresses.warp as Address,
          msg: {
            set_alloy: {
              underlying_denom: "hyp/xrp/xrp",
              alloyed_denom: "hyp/all/xrp",
              destination_domain: 123,
            },
          },
        },
      ],
    });

    const route = await client.queryWasmSmart<{ address: Address; fee: string }>({
      contract: addresses.warp,
      msg: {
        routes: {},
      },
    });

    const validatorSets = await client.queryWasmSmart({
      contract: addresses.hyperlane.ism as Address,
      msg: {
        validator_sets: {},
      },
    });

    const alloyTokens = await client.queryWasmSmart({
      contract: addresses.warp as Address,
      msg: {
        alloys: {},
      },
    });

    console.log(validatorSets, route, alloyTokens);
  } catch (err) {
    console.error(err);
  }
};

main();
