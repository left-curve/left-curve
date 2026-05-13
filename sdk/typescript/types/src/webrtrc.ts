import type { Json } from "./index.js";

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
