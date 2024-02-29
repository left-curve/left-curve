import { describe, expect, test } from "vitest";
import { Addr, Message, decodeUtf8, deserialize, encodeUtf8, serialize } from ".";

describe("serializing and deserializing complex types", () => {
  test.each([
    [
      "message update config",
      {
        updateConfig: {
          newCfg: {
            bank: Addr.fromString(
              "0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838",
            ),
          },
        },
      },
      '{"update_config":{"new_cfg":{"bank":"0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838"}}}',
    ],
    [
      "query balances request",
      {
        balances: {
          address: Addr.fromString(
            "0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838",
          ),
        },
      },
      '{"balances":{"address":"0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838"}}'
    ],
  ])("type = %s", (msgType, payload, json) => {
    expect(decodeUtf8(serialize(payload))).toStrictEqual(json);
    // TODO: deserialization doesn't work yet!!!
    // expect(deserialize(encodeUtf8(json))).toStrictEqual(payload);
  });
});
