import { defineChain } from "../defineChain.js";

export const testnet = /*#__PURE__*/ defineChain({
  id: "dango-testnet-1",
  name: "Testnet",
  nativeCoin: "dango",
  url: "https://api-testnet.dango.zone",
  blockExplorer: {
    name: "Testnet Explorer",
    txPage: "/tx/${txHash}",
    accountPage: "/account/${address}",
    contractPage: "/contract/${address}",
  },
});
