import { install } from "react-native-quick-crypto";
import { configureReanimatedLogger, ReanimatedLogLevel } from "react-native-reanimated";

configureReanimatedLogger({
  level: ReanimatedLogLevel.warn,
  strict: false,
});

install();

import "../assets/global.css";

import "expo-router/entry";
