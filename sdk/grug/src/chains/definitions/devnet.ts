import { defineChain } from "../defineChain.js";

export const devnet = /*#__PURE__*/ defineChain({
  id: "dev-6",
  name: "Devnet",
  nativeCoin: "uusdc",
  blockExplorers: {
    default: {
      name: "Devnet Explorer",
      txPage: "/${tx_hash}",
      accountPage: "/${address}",
    },
  },
  urls: {
    indexer: "https://devnet.dango.exchange/graphql",
  },
});
