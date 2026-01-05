import { defineChain } from "../defineChain.js";

export const mainnet = /*#__PURE__*/ defineChain({
  id: "dango-1",
  name: "Mainnet",
  nativeCoin: "dango",
  blockExplorer: {
    name: "Mainnet Explorer",
    txPage: "/tx/${txHash}",
    accountPage: "/account/${address}",
    contractPage: "/contract/${address}",
  },
  urls: {
    indexer: "https://api-mainnet.dango.zone",
  },
});
