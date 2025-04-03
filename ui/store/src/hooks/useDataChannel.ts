import type { DataChannelConfig } from "@left-curve/dango/types";
import { DataChannel } from "@left-curve/dango/utils";
import { type UseQueryResult, useQuery } from "@tanstack/react-query";

export type UseDataChannelParamaters = {
  url: string;
  key?: string;
  cfg?: Partial<DataChannelConfig>;
};

export type UseDataChannelReturnType = UseQueryResult<DataChannel, Error>;

export function useDataChannel(parameters: UseDataChannelParamaters): UseDataChannelReturnType {
  const { url, cfg, key } = parameters;
  return useQuery({
    queryKey: ["qr_connect", parameters, key || "key"],
    queryFn: async () => await DataChannel.create(url, cfg),
  });
}
