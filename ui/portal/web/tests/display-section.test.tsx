import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";
import { getLocale } from "@left-curve/foundation/paraglide/runtime.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseApp,
  setAppletsKitUseMediaQuery,
  setAppletsKitUseTheme,
} from "./mocks/applets-kit";

import { DisplaySection } from "../src/components/settings/DisplaySection";

type DisplaySettings = {
  chart: "tradingview";
  dateFormat: string;
  formatNumberOptions: {
    language: string;
    mask: number;
  };
  isFirstVisit: boolean;
  showWelcome: boolean;
  timeFormat: string;
  timeZone: "local" | "utc";
  useSessionKey: boolean;
};

const defaultSettings = (): DisplaySettings => ({
  chart: "tradingview",
  dateFormat: "MM/dd/yyyy",
  formatNumberOptions: {
    language: "en-US",
    mask: 1,
  },
  isFirstVisit: true,
  showWelcome: true,
  timeFormat: "hh:mm a",
  timeZone: "local",
  useSessionKey: true,
});

const displayMocks = vi.hoisted(() => ({
  changeSettings: vi.fn(),
  setThemeSchema: vi.fn(),
  settings: {
    chart: "tradingview",
    dateFormat: "MM/dd/yyyy",
    formatNumberOptions: {
      language: "en-US",
      mask: 1,
    },
    isFirstVisit: true,
    showWelcome: true,
    timeFormat: "hh:mm a",
    timeZone: "local",
    useSessionKey: true,
  } as DisplaySettings,
  themeSchema: "system" as "dark" | "light" | "system",
}));

class MockResizeObserver {
  disconnect = vi.fn();
  observe = vi.fn();
  unobserve = vi.fn();
}

function selectSetting(label: string, option: string) {
  fireEvent.click(screen.getByText(label));
  fireEvent.click(screen.getByText(option));
}

describe("DisplaySection", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    displayMocks.settings = defaultSettings();
    displayMocks.themeSchema = "system";
    setAppletsKitUseApp({
      changeSettings: displayMocks.changeSettings,
      settings: displayMocks.settings,
    });
    setAppletsKitUseMediaQuery({
      isMd: true,
    });
    setAppletsKitUseTheme({
      setThemeSchema: displayMocks.setThemeSchema,
      themeSchema: displayMocks.themeSchema,
    });
    localStorage.removeItem("dango.locale");
    vi.stubGlobal("ResizeObserver", MockResizeObserver);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

  it("renders the current locale from the generated Paraglide runtime", () => {
    render(<DisplaySection.Language />);

    expect(screen.getByText(m["settings.language"]())).toBeInTheDocument();
    expect(
      screen.getByText(m["settings.languages"]({ language: getLocale() })),
    ).toBeInTheDocument();
  });

  it("updates number formatting while preserving the rest of the formatting options", () => {
    render(<DisplaySection.FormatNumber />);

    expect(screen.getByText(m["settings.number"]())).toBeInTheDocument();

    selectSetting(m["settings.number"](), "1.234,56");

    expect(displayMocks.changeSettings).toHaveBeenCalledWith({
      formatNumberOptions: {
        language: "en-US",
        mask: 2,
      },
    });
  });

  it("updates date, time, and timezone settings with their selected values", () => {
    render(
      <>
        <DisplaySection.DateFormat />
        <DisplaySection.TimeFormat />
        <DisplaySection.TimeZone />
      </>,
    );

    selectSetting(m["settings.date"](), "2025/08/29");
    selectSetting(m["settings.time"](), "21:18");
    selectSetting(m["settings.timeZone"](), "UTC");

    expect(displayMocks.changeSettings).toHaveBeenNthCalledWith(1, {
      dateFormat: "yyyy/MM/dd",
    });
    expect(displayMocks.changeSettings).toHaveBeenNthCalledWith(2, {
      timeFormat: "HH:mm",
    });
    expect(displayMocks.changeSettings).toHaveBeenNthCalledWith(3, {
      timeZone: "utc",
    });
  });

  it("switches the theme schema from the theme tabs", () => {
    render(<DisplaySection.Theme />);

    expect(screen.getByText(m["settings.theme"]())).toBeInTheDocument();

    const [systemTab, lightTab, darkTab] = screen.getAllByRole("button");

    fireEvent.click(lightTab);
    fireEvent.click(darkTab);
    fireEvent.click(systemTab);

    expect(displayMocks.setThemeSchema).toHaveBeenNthCalledWith(1, "light");
    expect(displayMocks.setThemeSchema).toHaveBeenNthCalledWith(2, "dark");
    expect(displayMocks.setThemeSchema).toHaveBeenNthCalledWith(3, "system");
  });
});
