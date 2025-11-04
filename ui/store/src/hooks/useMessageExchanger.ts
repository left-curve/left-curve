import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import { MessageExchanger } from "../messageExchanger.js";

export type UseMessageExchangerParamaters = {
  url: string;
  key?: string;
};

export type UseMessageExchangerReturnType = UseQueryResult<MessageExchanger, Error>;

export function useMessageExchanger(
  parameters: UseMessageExchangerParamaters,
): UseMessageExchangerReturnType {
  const { url, key } = parameters;
  return useQuery({
    queryKey: ["qr_connect", parameters, key || "key"],
    queryFn: async () => await MessageExchanger.create(url),
  });
}
