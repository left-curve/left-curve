import { defineChain } from "../defineChain";

export const localhost = /*#__PURE__*/ defineChain({
  id: "grug-1",
  name: "Localhost",
  nativeCurrency: {
    decimals: 6,
    name: "USD Circle",
    symbol: "USDC",
    denom: "usdc",
    type: "native",
  },
  rpcUrls: {
    default: { http: ["http://127.0.0.1:26657"] },
  },
});
