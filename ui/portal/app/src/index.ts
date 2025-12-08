import "./shim";
import "../assets/global.css";
import { configureReanimatedLogger, ReanimatedLogLevel } from "react-native-reanimated";
import { AppRegistry } from "react-native";
import { App } from "./app";

configureReanimatedLogger({
  level: ReanimatedLogLevel.warn,
  strict: false,
});

AppRegistry.registerComponent("main", () => App);
