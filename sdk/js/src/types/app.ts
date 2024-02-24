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
  height: number;
  timestamp: number;
};

export type Account = {
  codeHash: string;
  admin?: string;
};
