import { expect, it } from "vitest";
import { Addr } from ".";

// a randomly generated address, as json string and as raw bytes
const json = '"0xccf2d114c23acb6735aa28418310cb20f042f1d2c3974969ac103376dae70838"';
const bytes = new Uint8Array([
  204, 242, 209, 20, 194, 58, 203, 103, 53, 170, 40, 65, 131, 16, 203, 32, 240, 66, 241, 210, 195,
  151, 73, 105, 172, 16, 51, 118, 218, 231, 8, 56,
]);

it("address should deserialize from json", () => {
  const addr = Addr.fromJSON(json);
  expect(addr).toStrictEqual(new Addr(bytes));
});

it("address should serialize to json", () => {
  const string = JSON.stringify(new Addr(bytes));
  expect(string).toStrictEqual(json);
});
