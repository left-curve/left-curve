export * from "@left-curve/sdk/utils";

export {
  getNavigatorOS,
  getRootDomain,
  isMobileOrTable,
} from "./browser.js";

export {
  getCoinsTypedData,
  composeTxTypedData,
  composeArbitraryTypedData,
} from "./typedData.js";

export {
  type FormatNumberOptions,
  type DisplayPart,
  formatNumber,
  formatDisplayNumber,
  formatDisplayString,
  bucketSizeToFractionDigits,
  truncateDec,
  formatUnits,
  parseUnits,
} from "./formatters.js";

export {
  calculateTradeSize,
  calculateFees,
  calculatePrice,
  formatOrderId,
  adjustPrice,
} from "./dex.js";
