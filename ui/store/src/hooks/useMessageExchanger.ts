import { type UseQueryResult, useQuery } from "@tanstack/react-query";
import { MessageExchanger } from "../messageExchanger.js";

export type UseMessageExchangerParameters = {
  url: string;
};

export type UseMessageExchangerReturnType = UseQueryResult<MessageExchanger, Error>;

export function useMessageExchanger(
  parameters: UseMessageExchangerParameters,
): UseMessageExchangerReturnType {
  const { url } = parameters;
  return useQuery({
    queryKey: ["qr_connect", url],
    queryFn: async () => await MessageExchanger.create(url),
  });
}
