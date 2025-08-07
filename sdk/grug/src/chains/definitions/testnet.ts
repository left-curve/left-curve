import { defineChain } from "../defineChain.js";

export const testnet = /*#__PURE__*/ defineChain({
  id: "dev-6",
  name: "Testnet",
  nativeCoin: "dango",
  blockExplorer: {
    name: "Testnet Explorer",
    txPage: "/tx/${txHash}",
    accountPage: "/account/${address}",
    contractPage: "/contract/${address}",
  },
  urls: {
    indexer: "https://graphql.dango.exchange/graphql",
  },
});
