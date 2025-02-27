import { describe, expect, it } from "vitest";
import { getNavigatorOS, getRootDomain } from "./browser.js";

describe("getNavigatorOS", () => {
  it("should return Windows for Windows user agent", () => {
    Object.defineProperty(navigator, "userAgent", {
      value: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
      configurable: true,
    });
    expect(getNavigatorOS()).toBe("Windows");
  });

  it("should return MacOS for Macintosh user agent", () => {
    Object.defineProperty(navigator, "userAgent", {
      value: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)",
      configurable: true,
    });
    expect(getNavigatorOS()).toBe("MacOS");
  });

  it("should return Linux for Linux user agent", () => {
    Object.defineProperty(navigator, "userAgent", {
      value: "Mozilla/5.0 (X11; Linux x86_64)",
      configurable: true,
    });
    expect(getNavigatorOS()).toBe("Linux");
  });

  it("should return iOS for iOS user agent", () => {
    Object.defineProperty(navigator, "userAgent", {
      value: "Mozilla/5.0 (iPhone; CPU iPhone OS 14_0 like Mac OS X)",
      configurable: true,
    });
    expect(getNavigatorOS()).toBe("iOS");
  });

  it("should return ChromeOS for ChromeOS user agent", () => {
    Object.defineProperty(navigator, "userAgent", {
      value: "Mozilla/5.0 (X11; CrOS x86_64 13020.82.0)",
      configurable: true,
    });
    expect(getNavigatorOS()).toBe("ChromeOS");
  });

  it("should return Unknown for unknown user agent", () => {
    Object.defineProperty(navigator, "userAgent", {
      value: "Mozilla/5.0 (Unknown OS)",
      configurable: true,
    });
    expect(getNavigatorOS()).toBe("Unknown");
  });
});

describe("getRootDomain", () => {
  it("should return the root domain for a given hostname", () => {
    expect(getRootDomain("www.example.com")).toBe("example.com");
    expect(getRootDomain("subdomain.example.co.uk")).toBe("co.uk");
    expect(getRootDomain("example.co")).toBe("example.co");
  });

  it("should handle single part domains", () => {
    expect(getRootDomain("localhost")).toBe("localhost");
  });

  it("should handle empty hostname", () => {
    expect(getRootDomain("")).toBe("");
  });
});
