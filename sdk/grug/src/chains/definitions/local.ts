import { defineChain } from "../defineChain.js";

export const local = /*#__PURE__*/ defineChain({
  id: "localdango-1",
  name: "Local",
  nativeCoin: "uusdc",
  blockExplorers: {
    default: {
      name: "Devnet Explorer",
      txPage: "/${tx_hash}",
      accountPage: "/${address}",
    },
  },
  urls: {
    indexer: "http://localhost:8080/graphql",
  },
});
