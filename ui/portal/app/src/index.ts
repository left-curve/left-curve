import "../assets/global.css";

import { App } from "./app";
import { AppRegistry } from "react-native";

import { install } from "react-native-quick-crypto";
import { configureReanimatedLogger, ReanimatedLogLevel } from "react-native-reanimated";

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
