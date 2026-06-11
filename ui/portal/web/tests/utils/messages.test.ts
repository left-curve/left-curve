import { m } from "@left-curve/foundation/paraglide/messages.js";
import { afterEach, describe, expect, it, vi } from "vitest";
import { message } from "./messages";

describe("test message helper", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("delegates to real Paraglide messages", () => {
    expect(message("common.send")).toBe(m["common.send"]());
  });

  it("repairs Playwright's Node-side localStorage shape before reading messages", () => {
    vi.stubGlobal("localStorage", {});

    const value = message("common.connectWallet");

    expect(value).toBe(m["common.connectWallet"]());
    expect(globalThis.localStorage.getItem("paraglide:languageTag")).toBeNull();
  });
});
