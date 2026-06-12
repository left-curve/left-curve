import { afterEach, describe, expect, it } from "vitest";

import { isFeatureEnabled } from "../src/featureFlags";

function setRuntimeFeatures(enabledFeatures?: unknown) {
  Object.defineProperty(window, "dango", {
    configurable: true,
    value: {
      enabledFeatures,
    },
  });
}

describe("runtime feature flags", () => {
  afterEach(() => {
    Reflect.deleteProperty(window, "dango");
  });

  it("keeps known features disabled when runtime config is missing or malformed", () => {
    Reflect.deleteProperty(window, "dango");

    expect(isFeatureEnabled("trade_history_export")).toBe(false);

    setRuntimeFeatures("trade_history_export");

    expect(isFeatureEnabled("trade_history_export")).toBe(false);
  });

  it("normalizes runtime feature ids and ignores unknown flags", () => {
    setRuntimeFeatures([" TRADE_HISTORY_EXPORT ", "unknown_feature", "trade_history_export"]);

    expect(isFeatureEnabled("trade_history_export")).toBe(true);
  });

  it("reads runtime feature changes on each check instead of caching stale config", () => {
    setRuntimeFeatures([]);

    expect(isFeatureEnabled("trade_history_export")).toBe(false);

    setRuntimeFeatures(["trade_history_export"]);

    expect(isFeatureEnabled("trade_history_export")).toBe(true);

    setRuntimeFeatures([]);

    expect(isFeatureEnabled("trade_history_export")).toBe(false);
  });
});
