import { useApp } from "@left-curve/foundation";
import { useAccount, useConfig } from "@left-curve/store";
import { View } from "react-native";
import WebView, { type WebViewMessageEvent } from "react-native-webview";
import { deserializeJson, serializeJson } from "@left-curve/dango/encoding";
import { useCallback, useRef } from "react";

import type React from "react";
import type { RemoteRequest } from "@left-curve/store/src/types";

type AppletProps = {
  uri: string;
};

export const Applet: React.FC<AppletProps> = ({ uri }) => {
  const webViewRef = useRef<WebView>(null);
  const { settings } = useApp();
  const { chain, coins, state } = useConfig();
  const { navigate, toast, showModal, hideModal } = useApp();
  const { connector } = useAccount();

  const connection = state.connectors.get(state.current || "");

  const onMessage = useCallback(
    async (event: WebViewMessageEvent) => {
      const { data } = event.nativeEvent;
      const message = deserializeJson<RemoteRequest>(data);

      if (message?.type !== "dango-remote") return;

      const { id, method, type, args } = message;

      switch (method) {
        case "navigate":
          navigate(...(args as [string, object]));
          break;
        case "showModal":
          showModal(...(args as [string, Record<string, unknown>]));
          break;
        case "hideModal":
          hideModal();
          break;
        case "toast": {
          const [arg0, ...rest] = args;
          toast[arg0 as "success" | "error"](...rest);
          break;
        }
        case "connector":
          try {
            if (!connector) throw new Error("No connector");
            const [arg0, ...rest] = args;
            const result = await (
              connector[arg0 as keyof typeof connector] as (...args: unknown[]) => Promise<unknown>
            )(...rest);
            webViewRef.current?.postMessage(serializeJson({ id, type, result }));
          } catch (error) {
            webViewRef.current?.postMessage(serializeJson({ id, type, error }));
          }
          break;
      }
    },
    [toast, connector, navigate, showModal, hideModal],
  );

  return (
    <View className="flex-1 flex">
      <WebView
        ref={webViewRef}
        source={{ uri }}
        style={{ flex: 1 }}
        onMessage={onMessage}
        injectedJavaScriptBeforeContentLoaded={`
          window.dango = {
            settings: ${JSON.stringify(settings)},
            chain: ${JSON.stringify(chain)},
            coins: ${JSON.stringify(coins.byDenom)},
            connection: ${
              connection
                ? serializeJson({ connection: { ...connection, connector: undefined } })
                : "undefined"
            }
          };`}
      />
    </View>
  );
};
