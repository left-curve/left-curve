import { defineChain } from "../defineChain.js";

export const local = /*#__PURE__*/ defineChain({
  id: "localdango-1",
  name: "Local",
  nativeCoin: "dango",
  blockExplorer: {
    name: "Local Explorer",
    txPage: "/tx/${txHash}",
    accountPage: "/account/${address}",
    contractPage: "/contract/${address}",
  },
  urls: {
    indexer: "http://localhost:8080/graphql",
  },
});
