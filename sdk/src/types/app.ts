import type { Message } from ".";

export type GenesisState = {
  config: Config;
  msgs: Message[];
};

export type Config = {
  owner?: string;
  bank: string;
};

export type BlockInfo = {
  height: string;
  timestamp: string;
  hash: string;
};
