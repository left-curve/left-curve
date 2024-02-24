import type { Addr, Hash, Message, Uint } from ".";

export type GenesisState = {
  config: Config;
  msgs: Message[];
};

export type Config = {
  owner?: Addr;
  bank: Addr;
};

export type BlockInfo = {
  height: Uint;
  timestamp: Uint;
  hash: Hash,
};
