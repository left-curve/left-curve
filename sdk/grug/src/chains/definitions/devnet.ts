import { defineChain } from "../defineChain.js";

export const devnet = /*#__PURE__*/ defineChain({
  id: "dev-9",
  name: "Devnet",
  nativeCoin: "dango",
  blockExplorers: {
    default: {
      name: "Devnet Explorer",
      txPage: "/tx/${txHash}",
      accountPage: "/account/${address}",
    },
  },
  urls: {
    indexer: "https://devnet.dango.exchange/graphql",
  },
});
