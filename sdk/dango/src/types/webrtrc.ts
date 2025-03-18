import type { Json } from "@left-curve/sdk/types";

export type DataChannelConfig = {
  rtcConfiguration: RTCConfiguration;
  channelName: string;
  logs: boolean;
};

export type DataChannelMessage = {
  id: string;
  type: string;
  message: Json;
};
