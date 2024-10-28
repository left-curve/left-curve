import { defineChain } from "../defineChain";

export const localhost = /*#__PURE__*/ defineChain({
  id: "grug-1",
  name: "Localhost",
  nativeCoin: {
    decimals: 6,
    name: "USD Circle",
    symbol: "USDC",
    denom: "uusdc",
    type: "native",
  },
  blockExplorers: {
    default: {
      name: "Localhost Explorer",
      txPage: "http://localhost/tx/${tx_hash}",
      accountPage: "http://localhost/account/${address}",
    },
  },
  rpcUrls: {
    default: { http: ["http://localhost:26657"] },
  },
});
