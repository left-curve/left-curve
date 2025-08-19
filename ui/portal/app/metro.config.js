const { getDefaultConfig } = require("expo/metro-config");
const { withNativeWind } = require("nativewind/metro");
const path = require("node:path");

const workspaceRoot = path.resolve(__dirname, "../../../");
const projectRoot = __dirname;

const config = getDefaultConfig(projectRoot);

config.watchFolders = [workspaceRoot];

config.resolver.nodeModulesPaths = [
  path.resolve(projectRoot, "node_modules"),
  path.resolve(workspaceRoot, "node_modules"),
];

config.resolver.resolveRequest = (context, moduleName, platform) => {
  if (moduleName === "crypto") {
    // when importing crypto, resolve to react-native-quick-crypto
    return context.resolveRequest(context, "react-native-quick-crypto", platform);
  }
  // otherwise chain to the standard Metro resolver.
  return context.resolveRequest(context, moduleName, platform);
};

module.exports = withNativeWind(config, { input: "./assets/global.css" });
