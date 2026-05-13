import { defineChain } from "../defineChain.js";

export const mainnet = /*#__PURE__*/ defineChain({
  id: "dango-1",
  name: "Mainnet",
  nativeCoin: "dango",
  url: "https://api-mainnet.dango.zone",
  blockExplorer: {
    name: "Mainnet Explorer",
    txPage: "/tx/${txHash}",
    accountPage: "/account/${address}",
    contractPage: "/contract/${address}",
  },
});
