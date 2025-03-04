import { PrivateKeySigner, createSignerClient, devnet, graphql } from "@left-curve/dango";
import { deserializeJson } from "@left-curve/dango/encoding";
import { wait } from "@left-curve/dango/utils";

const v =
  '{"json":[{"address":"b420e624002f5c9227b933e9c4a796a9b978716a","secret":"jY8WShE2vWUOq0KllzZyNva92SuCtBINszlkWIwHkPE="},{"address":"eae2c7cdc5679911ef8bb16c52b4b90c41482a91","secret":"sM+KWO71JA8O0Of7y+vVuJx6K1YDO2nbwU/5Li1/4gM="},{"address":"2f1bdca1029174125eef9b36bf338118c0098fa7","secret":"FnZSU2ZExov1ukbroS93jec2iEr58zipF/l0QtbWDV8="},{"address":"f073ef82d55a07db07aa2884b5245d74cc421a67","secret":"VKvZZoUR+CfOg2Svm6dma8ADpkdGVnE9GAY3v1LqJi8="}],"meta":{"values":{"0.secret":[["custom","Uint8Array"]],"1.secret":[["custom","Uint8Array"]],"2.secret":[["custom","Uint8Array"]],"3.secret":[["custom","Uint8Array"]]}}}';

import type { Address, AppConfig } from "@left-curve/dango/types";

const main = async () => {
  try {
    const client = createSignerClient({
      transport: graphql(),
      username: "owner",
      chain: devnet,
      signer: PrivateKeySigner.fromMnemonic(
        "success away current amateur choose crystal busy labor cost genius industry cement rhythm refuse whale admit meadow truck edge tiger melt flavor weapon august",
      ),
    });

    const accounts = await client.getAccountsByUsername({ username: "owner" });
    const sender = Object.keys(accounts)[0] as Address;

    const { addresses } = await client.getAppConfig<AppConfig>();

    const validators: { address: string }[] = deserializeJson(v);

    await client.execute({
      contract: addresses.hyperlane.ism as Address,
      msg: {
        set_validators: {
          domain: 123,
          threshold: 2,
          validators: validators.map((v) => v.address),
        },
      },
      sender,
    });

    await wait(3000);

    await client.execute({
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
      sender,
    });

    await wait(3000);

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

    console.log(validatorSets, route);
  } catch (err) {
    console.error(err);
  }
};

main();
