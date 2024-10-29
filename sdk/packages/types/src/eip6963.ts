import type { EIP1193Provider } from "./eip1193.js";

export type EIP6963ProviderDetail = {
  info: EIP6963ProviderInfo;
  provider: EIP1193Provider;
};

export type EIP6963ProviderInfo = {
  icon: `data:image/${string}`;
  name: string;
  rdns: string;
  uuid: string;
};
