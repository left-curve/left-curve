import { describe, expect, test } from "vitest";
import { deserialize, serialize } from "./binary.js";
import { decodeUtf8, encodeUtf8 } from "./utf8.js";

describe("serializing and deserializing complex types", () => {
  test.each([
    [
      "message update config",
      {
        updateConfig: {
          newCfg: {
            bank: "0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838",
          },
        },
      },
      '{"update_config":{"new_cfg":{"bank":"0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838"}}}',
    ],
    [
      "query balances request",
      {
        balances: {
          address: "0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838",
        },
      },
      '{"balances":{"address":"0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838"}}',
    ],
    [
      "account state response",
      {
        publicKey: "A9UPi4RnZTRm/kfjcn12SQHZZ2+J4qX+2FKnzHgDJgjH",
        sequence: 123,
      },
      '{"public_key":"A9UPi4RnZTRm/kfjcn12SQHZZ2+J4qX+2FKnzHgDJgjH","sequence":123}',
    ],
  ])("type = %s", (type, payload, json) => {
    expect(decodeUtf8(serialize(payload))).toStrictEqual(json);
    expect(deserialize(encodeUtf8(json))).toStrictEqual(payload);
  });
});
