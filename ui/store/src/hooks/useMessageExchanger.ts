import { MessageExchanger } from "../messageExchanger.js";
import { useEffect, useState } from "react";

export type UseMessageExchangerParameters = {
  url: string;
  subscribe?: <T = any>(
    listener: { id: string; type: string; message: T },
    exchanger: MessageExchanger,
  ) => void;
};

export type UseMessageExchangerReturnType = {
  messageExchanger: MessageExchanger | null;
  isLoading: boolean;
};

export function useMessageExchanger(
  parameters: UseMessageExchangerParameters,
): UseMessageExchangerReturnType {
  const { url, subscribe } = parameters;

  const [isLoading, setIsLoading] = useState(false);
  const [messageExchanger, setMessageExchanger] = useState<MessageExchanger | null>(null);

  useEffect(() => {
    let c: MessageExchanger | null = null;
    (async () => {
      setIsLoading(true);
      try {
        const exchanger = await MessageExchanger.create(url);
        if (subscribe) exchanger.subscribe((msg) => subscribe(msg, exchanger));
        setMessageExchanger(exchanger);
        setIsLoading(false);
        c = exchanger;
      } catch (error) {
        console.error("Error creating message exchanger: ", error);
      } finally {
        setIsLoading(false);
      }
    })();

    return () => {
      if (!c) return;
      c.close();
      setMessageExchanger(null);
    };
  }, [url, setIsLoading, setMessageExchanger]);

  return { messageExchanger, isLoading };
}
