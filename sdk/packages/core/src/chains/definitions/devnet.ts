import { defineChain } from "../defineChain.js";

export const devnet = /*#__PURE__*/ defineChain({
  id: "dev-3",
  name: "Devnet",
  nativeCoin: {
    decimals: 6,
    name: "USD Circle",
    symbol: "USDC",
    denom: "uusdc",
    type: "native",
  },
  blockExplorers: {
    default: {
      name: "Devnet Explorer",
      txPage: "/${tx_hash}",
      accountPage: "/${address}",
    },
  },
  rpcUrls: {
    default: { http: ["https://devnet-rpc.dango.zone/"] },
  },
});
