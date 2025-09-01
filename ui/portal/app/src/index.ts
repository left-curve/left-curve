import "../assets/global.css";

import { install } from "react-native-quick-crypto";
import { AppRegistry } from "react-native";
import { configureReanimatedLogger, ReanimatedLogLevel } from "react-native-reanimated";
import { App } from "./app";

global.BroadcastChannel = class {
  constructor(public name: string) {}
  addEventListener() {}
  removeEventListener() {}
  postMessage() {}
  close() {}
} as unknown as typeof global.BroadcastChannel;

configureReanimatedLogger({
  level: ReanimatedLogLevel.warn,
  strict: false,
});

install();

AppRegistry.registerComponent("main", () => App);
