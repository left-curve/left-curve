import type { EIP1193Provider } from "./eip1193.js";
import "./window.js";

export interface EIP6963AnnounceProviderEvent extends CustomEvent<EIP6963ProviderDetail> {
  type: "eip6963:announceProvider";
}

export interface EIP6963RequestProviderEvent extends Event {
  type: "eip6963:requestProvider";
}

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
