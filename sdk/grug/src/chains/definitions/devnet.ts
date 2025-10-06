import { defineChain } from "../defineChain.js";

export const devnet = /*#__PURE__*/ defineChain({
  id: "dev-9",
  name: "Devnet",
  nativeCoin: "dango",
  blockExplorer: {
    name: "Devnet Explorer",
    txPage: "/tx/${txHash}",
    accountPage: "/account/${address}",
    contractPage: "/contract/${address}",
  },
  urls: {
    indexer: "https://dev-api.dango.exchange/graphql",
  },
});
