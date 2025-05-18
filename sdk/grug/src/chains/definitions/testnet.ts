import { defineChain } from "../defineChain.js";

export const testnet = /*#__PURE__*/ defineChain({
  id: "dev-6",
  name: "Testnet",
  nativeCoin: "dango",
  blockExplorers: {
    default: {
      name: "Testnet Explorer",
      txPage: "/tx/${txHash}",
      accountPage: "/account/${address}",
    },
  },
  urls: {
    indexer: "https://testnet.dango.exchange/graphql",
  },
});
