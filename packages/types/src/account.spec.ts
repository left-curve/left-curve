import { encodeHex } from "@leftcurve/encoding";
import { expect, test } from "vitest";
import { createSignBytes } from "./account";
import type { Message } from "./tx";

test("creating sign bytes", () => {
  const sender = "0xc3e1842184f9c0271b1dadd719c6f3d172e715ea60bf445d63489a4dbed1f6e4";
  const chainId = "dev-1";
  const sequence = 0;
  const msg: Message = {
    transfer: {
      to: "0xecd3d63044b62571eb98dea97bd62142bb7b32b9ab590ccaffa2b50134b19db2",
      coins: [
        {
          denom: "uatom",
          amount: "1234",
        },
        {
          denom: "uosmo",
          amount: "2345",
        },
      ],
    },
  };
  const signBytes = createSignBytes([msg], sender, chainId, sequence);
  expect(encodeHex(signBytes)).toStrictEqual(
    "baf97f186c39cbb4d4738ee98e8914e6738685cce45c59ca702d201f3375540a",
  );
});
