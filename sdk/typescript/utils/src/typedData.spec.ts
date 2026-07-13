import { hashTypedData, sha256, toHex } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { describe, expect, it } from "vitest";
import {
  canonicalJson,
  composeOnboardTypedData,
  composeSessionTypedData,
  composeTxTypedData,
} from "./typedData.js";

describe("canonicalJson", () => {
  it("sorts object keys alphabetically", () => {
    expect(canonicalJson({ b: 1, a: 2 })).toBe('{"a":2,"b":1}');
  });

  it("recurses into nested objects", () => {
    expect(canonicalJson({ z: { b: 1, a: 2 }, a: 0 })).toBe('{"a":0,"z":{"a":2,"b":1}}');
  });

  it("preserves array element order", () => {
    expect(canonicalJson([{ b: 1, a: 2 }, { c: 3 }])).toBe('[{"a":2,"b":1},{"c":3}]');
  });

  it("emits no whitespace", () => {
    expect(canonicalJson({ a: 1, b: 2 })).not.toMatch(/\s/);
  });
});

describe("composeSessionTypedData", () => {
  it("matches the canonical Rust schema", () => {
    const typedData = composeSessionTypedData({
      kind: "session",
      chainId: "dev-1",
      sessionKey: "Aw/fkcaAsRJnpEXVdITRGxi5zit26Di5HmxnK3HblE7E",
      expireAt: "340282366920938463463374607431.768211455",
    });

    expect(typedData.primaryType).toBe("Message");
    expect(typedData.domain).toEqual({
      name: "dango",
      chainId: 1,
      verifyingContract: "0x0000000000000000000000000000000000000000",
    });
    expect(typedData.types.Message).toEqual([
      { name: "chain_id", type: "string" },
      { name: "session_key", type: "string" },
      { name: "expire_at", type: "string" },
    ]);
    expect(typedData.message).toEqual({
      chain_id: "dev-1",
      session_key: "Aw/fkcaAsRJnpEXVdITRGxi5zit26Di5HmxnK3HblE7E",
      expire_at: "340282366920938463463374607431.768211455",
    });
  });
});

describe("composeOnboardTypedData", () => {
  it("serializes the Key enum as a canonical JSON string", () => {
    const typedData = composeOnboardTypedData({
      kind: "onboard",
      chainId: "dev-1",
      key: { ethereum: "0xaf1f9c713011315b891a401f07f3abd9a37cb975" },
      keyHash: "182FFC10DD3C6E854C0754FD09A14DCE132D33674596EB48BAC9F0AAD36A7BFE",
      seed: 0,
    });

    expect(typedData.message.key).toBe(
      '{"ethereum":"0xaf1f9c713011315b891a401f07f3abd9a37cb975"}',
    );
    expect(typedData.message).toEqual({
      chain_id: "dev-1",
      key: '{"ethereum":"0xaf1f9c713011315b891a401f07f3abd9a37cb975"}',
      key_hash: "182FFC10DD3C6E854C0754FD09A14DCE132D33674596EB48BAC9F0AAD36A7BFE",
      seed: 0,
    });
    expect(typedData.types.Message).toEqual([
      { name: "chain_id", type: "string" },
      { name: "key", type: "string" },
      { name: "key_hash", type: "string" },
      { name: "seed", type: "uint32" },
    ]);
  });

  it("appends a referrer field when present", () => {
    const typedData = composeOnboardTypedData({
      kind: "onboard",
      chainId: "dev-1",
      key: { ethereum: "0xaf1f9c713011315b891a401f07f3abd9a37cb975" },
      keyHash: "DEADBEEF",
      seed: 1,
      referrer: 42,
    });

    expect(typedData.message.referrer).toBe(42);
    expect(typedData.types.Message).toContainEqual({ name: "referrer", type: "uint32" });
  });
});

describe("composeTxTypedData", () => {
  it("collapses each Message variant into kind/payload entries", () => {
    const typedData = composeTxTypedData({
      sender: "0x1e9221f68bb9ccbad6feff6acc56b633b9b3f43a",
      gasLimit: 1000,
      messages: [
        {
          transfer: {
            "0x3097760988e63aad38c49974b08477d168a82f82": { "bridge/usdc": "100000000" },
          },
        },
      ],
      data: {
        chainId: "dev-1",
        userIndex: 268179343,
        nonce: 6,
      },
    });

    expect(typedData.primaryType).toBe("Message");
    expect(typedData.domain).toEqual({
      name: "dango",
      chainId: 1,
      verifyingContract: "0x1e9221f68bb9ccbad6feff6acc56b633b9b3f43a",
    });
    expect(typedData.types.TxMessage).toEqual([
      { name: "kind", type: "string" },
      { name: "payload", type: "string" },
    ]);
    expect(typedData.message.messages).toEqual([
      {
        kind: "transfer",
        payload:
          '{"0x3097760988e63aad38c49974b08477d168a82f82":{"bridge/usdc":"100000000"}}',
      },
    ]);
    expect(typedData.message.data).toEqual({
      user_index: 268179343,
      chain_id: "dev-1",
      nonce: 6,
    });
  });

  it("adds an expiry field to Metadata only when present", () => {
    const withExpiry = composeTxTypedData({
      sender: "0x0000000000000000000000000000000000000001",
      gasLimit: 1,
      messages: [{ upload: { code: "AA==" } }],
      data: { chainId: "x", userIndex: 1, nonce: 1, expiry: "123" },
    });
    expect(withExpiry.types.Metadata).toContainEqual({ name: "expiry", type: "string" });
    expect(withExpiry.message.data).toMatchObject({ expiry: "123" });

    const withoutExpiry = composeTxTypedData({
      sender: "0x0000000000000000000000000000000000000001",
      gasLimit: 1,
      messages: [{ upload: { code: "AA==" } }],
      data: { chainId: "x", userIndex: 1, nonce: 1 },
    });
    expect(withoutExpiry.types.Metadata).not.toContainEqual({ name: "expiry", type: "string" });
    expect(withoutExpiry.message.data).not.toHaveProperty("expiry");
  });

  it("translates camelCase keys in the inner payload to snake_case", () => {
    const typedData = composeTxTypedData({
      sender: "0x0000000000000000000000000000000000000001",
      gasLimit: 1,
      messages: [
        {
          execute: {
            contract: "0x0000000000000000000000000000000000000002",
            msg: { fooBar: "baz" },
            funds: {},
          },
        },
      ],
      data: { chainId: "x", userIndex: 1, nonce: 1 },
    });

    expect(typedData.message.messages[0].payload).toContain('"foo_bar"');
  });
});

// Cross-language digest fixtures. The hex values below come from
// `dango/auth/src/eip712.rs::tests`. If a digest mismatches, the TS canonical
// builder has drifted from the Rust canonical builder.
describe("EIP-712 digest equivalence with Rust", () => {
  it("transaction digest matches Rust fixture", () => {
    const typedData = composeTxTypedData({
      sender: "0x1234567890123456789012345678901234567890",
      gasLimit: 16075769052062025908n,
      messages: [
        {
          transfer: {
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd": { usdc: "1000000" },
          },
        },
        {
          execute: {
            contract: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            msg: { foo: "bar" },
            funds: {},
          },
        },
      ],
      data: { chainId: "dev-1", userIndex: 42, nonce: 5 },
    });

    const digest = hashTypedData({
      domain: typedData.domain,
      types: typedData.types,
      primaryType: typedData.primaryType,
      message: typedData.message,
    });

    expect(digest).toBe(
      "0xe2ec813e42e60bdc53b296440153dc9128b9317025f2c280d0ae90f57681550c",
    );
  });

  it("session digest matches Rust fixture", () => {
    // 33 bytes of 0x02 in base64.
    const sessionKey = "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgIC";

    const typedData = composeSessionTypedData({
      kind: "session",
      chainId: "dev-1",
      sessionKey,
      // Timestamp(1_700_000_000_000_000_000 ns) = exactly 1_700_000_000 s.
      // grug's `Dec` Display drops the trailing fractional when it's all zeros.
      expireAt: "1700000000",
    });

    const digest = hashTypedData({
      domain: typedData.domain,
      types: typedData.types,
      primaryType: typedData.primaryType,
      message: typedData.message,
    });

    expect(digest).toBe(
      "0x4adf7aa1c10e6d0f080afdbce4f032fca952b73a5d9cf22a56ae77f30c35d297",
    );
  });

  it("produces a transaction signature that the Rust verifier accepts", async () => {
    // Deterministic key so the Rust verifier test can hardcode the resulting
    // signature. The Ethereum address derived from `0x01...01` is below.
    const privateKey = `0x${"01".repeat(32)}` as const;
    const account = privateKeyToAccount(privateKey);
    expect(account.address.toLowerCase()).toBe(
      "0x1a642f0e3c3af545e7acbd38b07251b3990914f1",
    );

    const typedData = composeTxTypedData({
      sender: account.address.toLowerCase() as `0x${string}`,
      gasLimit: 16075769052062025908n,
      messages: [
        {
          transfer: {
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd": { usdc: "1000000" },
          },
        },
      ],
      data: { chainId: "dev-1", userIndex: 42, nonce: 5 },
    });

    const signature = await account.signTypedData({
      domain: typedData.domain,
      types: typedData.types,
      primaryType: typedData.primaryType,
      message: typedData.message,
    });

    // Deterministic for this fixed (key, digest) pair. If this value ever
    // changes, the matching Rust test `tests::ts_signed_eip712_verifies` in
    // `dango/auth/src/eip712.rs` must be updated with the new signature.
    expect(signature).toBe(
      "0x6884e0987ba28a0ed20576d1d5edb4e5eb297b1e304a76d7933523bdb6285a7f7bd936c8bc018e6db7c940629f3f7da4bef65db103df51201f63c5ec9955c6611c",
    );
  });

  it("onboard digest matches Rust fixture", () => {
    // Rust: `key_addr.as_ref().hash256()` where `key_addr` is 20 bytes of 0x11.
    const keyAddrBytes = new Uint8Array(20).fill(0x11);
    const keyHash = sha256(toHex(keyAddrBytes)).slice(2).toUpperCase();

    const typedData = composeOnboardTypedData({
      kind: "onboard",
      chainId: "dev-1",
      key: { ethereum: "0x1111111111111111111111111111111111111111" },
      keyHash,
      seed: 7,
    });

    const digest = hashTypedData({
      domain: typedData.domain,
      types: typedData.types,
      primaryType: typedData.primaryType,
      message: typedData.message,
    });

    expect(digest).toBe(
      "0xaab3d150835e6fb79fb2158db3848ab78dd6cf7afb8d08209fd6003392421731",
    );
  });
});
