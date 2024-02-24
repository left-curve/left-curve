import type { Addr, Hash, Message } from ".";

export type GenesisState = {
  config: Config;
  msgs: Message[];
};

export type Config = {
  owner?: Addr;
  bank: Addr;
};

export type BlockInfo = {
  height: number;
  timestamp: number;
  hash: Hash,
};
