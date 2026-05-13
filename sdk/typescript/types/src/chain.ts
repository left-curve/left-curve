import type { Denom } from "./coins.js";

export type ChainId = string;

export type Chain = {
  id: ChainId;
  name: string;
  url: string;
  nativeCoin: Denom;
  blockExplorer: BlockExplorer;
};

type BlockExplorer = {
  name: string;
  txPage: string;
  contractPage: string;
  accountPage: string;
};
