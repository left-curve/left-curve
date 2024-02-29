import { sha256 } from "@cosmjs/crypto";
import { describe, expect, test } from "vitest";
import { decodeHex, encodeUtf8, verifyMembershipProof, verifyNonMembershipProof } from ".";

// we use the same test case as in the cw-jmt crate
const rootHash = decodeHex("ae08c246d53a8ff3572a68d5bba4d610aaaa765e3ef535320c5653969aaa031b");

describe("verifying membership proofs", () => {
  test.each([
    [
      "r",
      "foo",
      {
        siblingHashes: [
          "e104e2bcf24027af737c021033cb9d8cbd710a463f54ae6f2ff9eb06c784c744",
          null,
          "cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222",
        ],
      },
    ],
    [
      "m",
      "bar",
      {
        siblingHashes: [
          "412341380b1e171077dd9da9af936ae2126ede2dd91dc5acb0f77363d46eb76b",
          "c8348e9a7a327e8b76e97096c362a1f87071ee4108b565d1f409529c189cb684",
          null,
          "cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222",
        ],
      },
    ],
    [
      "L",
      "fuzz",
      {
        siblingHashes: [
          "fd34e3f8d9840e7f6d6f639435b6f9b67732fc5e3d5288e268021aeab873f280",
          "c8348e9a7a327e8b76e97096c362a1f87071ee4108b565d1f409529c189cb684",
          null,
          "cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222",
        ],
      },
    ],
    [
      "a",
      "buzz",
      {
        siblingHashes: ["b843a96765fc40641227234e9f9a2736c2e0cdf8fb2dc54e358bb4fa29a61042"],
      },
    ],
  ])("key = %s, value = %s", (key, value, proof) => {
    expect(() => {
      const keyHash = sha256(encodeUtf8(key));
      const valueHash = sha256(encodeUtf8(value));
      return verifyMembershipProof(rootHash, keyHash, valueHash, proof);
    }).not.toThrow();
  });
});

describe("verifying non-membership proofs", () => {
  test.each([
    [
      "b",
      {
        node: {
          internal: {
            leftHash: null,
            rightHash: "521de0a3ef2b7791666435a872ca9ec402ce886aff07bb4401de28bfdde4a13b",
          },
        },
        siblingHashes: ["cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222"],
      },
    ],
    [
      "o",
      {
        node: {
          leaf: {
            keyHash: "62c66a7a5dd70c3146618063c344e531e6d4b59e379808443ce962b3abd63c5a",
            valueHash: "fcde2b2edba56bf408601fb721fe9b5c338d10ee429ea04fae5511b68fbf8fb9",
          },
        },
        siblingHashes: [
          "412341380b1e171077dd9da9af936ae2126ede2dd91dc5acb0f77363d46eb76b",
          "c8348e9a7a327e8b76e97096c362a1f87071ee4108b565d1f409529c189cb684",
          null,
          "cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222",
        ],
      },
    ],
  ])("key = %s", (key, proof) => {
    expect(() => {
      const keyHash = sha256(encodeUtf8(key));
      return verifyNonMembershipProof(rootHash, keyHash, proof);
    }).not.toThrow();
  });
});
