import { useApp } from "@left-curve/foundation";
import { useConfig } from "@left-curve/store";
import { View } from "react-native";
import WebView from "react-native-webview";

import type React from "react";

type AppletProps = {
  id: string;
};

export const Applet: React.FC<AppletProps> = ({ id }) => {
  const { settings } = useApp();
  const { chain, coins } = useConfig();

  return (
    <View className="flex-1 flex">
      <WebView
        source={{ uri: "http://localhost:5180/" }}
        style={{ flex: 1 }}
        onMessage={(event) => {
          console.log(event);
        }}
        injectedJavaScriptBeforeContentLoaded={`
          window.dango = {
            settings: ${JSON.stringify(settings)}
            chain: ${JSON.stringify(chain)},
            coins: ${JSON.stringify(coins.byDenom)}
          };`}
      />
    </View>
  );
};
