import { install } from "react-native-quick-crypto";

import "../assets/global.css";

import { App } from "./app";
import { AppRegistry } from "react-native";
import { createMMKVStorage } from "~/storage";

import { configureReanimatedLogger, ReanimatedLogLevel } from "react-native-reanimated";

global.localStorage = createMMKVStorage() as Storage;

global.BroadcastChannel = class {
  constructor(public name: string) {}
  addEventListener() {}
  removeEventListener() {}
  postMessage() {}
  close() {}
} as unknown as typeof BroadcastChannel;

configureReanimatedLogger({
  level: ReanimatedLogLevel.warn,
  strict: false,
});

install();

AppRegistry.registerComponent("main", () => App);
