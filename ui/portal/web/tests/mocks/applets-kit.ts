import { vi } from "vitest";

import type { ReactNode, RefObject } from "react";

type MockAppState = Record<string, unknown> & {
  settings?: Record<string, unknown> & {
    dateFormat?: string;
    formatNumberOptions?: Record<string, unknown>;
    timeFormat?: string;
    timeZone?: string;
  };
};
type MockAppSelector = (state: MockAppState) => unknown;

type MockMediaQueryState = Record<string, unknown> & {
  isLg?: boolean;
  isMd?: boolean;
};

type MockThemeState = Record<string, unknown> & {
  setThemeSchema?: (themeSchema: "dark" | "light" | "system") => void;
  theme?: "dark" | "light";
  themeSchema?: "dark" | "light" | "system";
};

type MockCountdownState = Record<string, unknown> & {
  days?: string;
  hours?: string;
  minutes?: string;
  seconds?: string;
};

type MockCountdownFactory = (...args: unknown[]) => MockCountdownState;
type MockHeaderHeightFactory = () => number;
type MockInfiniteScrollFactory = (...args: unknown[]) => {
  loadMoreRef: RefObject<HTMLElement | null> | ((element: HTMLElement | null) => void);
};
type MockMarqueeFactory = (props: {
  className?: string;
  item: ReactNode;
  speed: number;
}) => ReactNode;
type MockPortalTargetFactory = (selector: string) => Element | null;
type MockQRCodeReaderFactory = (props: { onScan: (value: string) => void }) => ReactNode;
type MockTextLoopFactory = (props: { texts: string[] }) => ReactNode;
type MockUseAnimateOnceFactory = (...args: unknown[]) => boolean;
type MockUsePreserveScrollFactory = (...args: unknown[]) => RefObject<HTMLElement | null>;
type MockUnknownHookFactory = (...args: unknown[]) => unknown;

function defaultAppState(): MockAppState {
  return {
    settings: {
      dateFormat: "en-US",
      formatNumberOptions: {
        language: "en-US",
      },
      timeFormat: "HH:mm",
      timeZone: "UTC",
    },
  };
}

function withDefaultAppState(appState: MockAppState): MockAppState {
  const defaults = defaultAppState();

  return {
    ...defaults,
    ...appState,
    settings: {
      ...defaults.settings,
      ...appState.settings,
      formatNumberOptions: {
        ...defaults.settings?.formatNumberOptions,
        ...appState.settings?.formatNumberOptions,
      },
    },
  };
}

function selectAppState(appState: MockAppState, selector?: MockAppSelector) {
  const nextAppState = withDefaultAppState(appState);
  return typeof selector === "function" ? selector(nextAppState) : nextAppState;
}

function defaultMediaQueryState(): MockMediaQueryState {
  return {
    isLg: true,
    isMd: true,
  };
}

function defaultThemeState(): MockThemeState {
  return {
    theme: "light",
    themeSchema: "system",
  };
}

const appletsKitMocks = vi.hoisted(() => ({
  useApp: vi.fn((selector?: MockAppSelector) => {
    const appState = {
      settings: {
        dateFormat: "en-US",
        formatNumberOptions: {
          language: "en-US",
        },
        timeFormat: "HH:mm",
        timeZone: "UTC",
      },
    };

    return typeof selector === "function" ? selector(appState) : appState;
  }),
  useMediaQuery: vi.fn(() => ({
    isLg: true,
    isMd: true,
  })),
  useBodyScrollLock: undefined as MockUnknownHookFactory | undefined,
  useClickAway: undefined as MockUnknownHookFactory | undefined,
  useCountdown: undefined as MockCountdownFactory | undefined,
  useHeaderHeight: undefined as MockHeaderHeightFactory | undefined,
  useInfiniteScroll: undefined as MockInfiniteScrollFactory | undefined,
  Marquee: undefined as MockMarqueeFactory | undefined,
  usePortalTarget: undefined as MockPortalTargetFactory | undefined,
  QRCodeReader: undefined as MockQRCodeReaderFactory | undefined,
  TextLoop: undefined as MockTextLoopFactory | undefined,
  useAnimateOnce: undefined as MockUseAnimateOnceFactory | undefined,
  usePreserveScroll: undefined as MockUsePreserveScrollFactory | undefined,
  useTheme: vi.fn(() => ({
    theme: "light",
    themeSchema: "system",
  })),
}));

export function resetAppletsKitMocks() {
  appletsKitMocks.useApp.mockImplementation((selector?: MockAppSelector) =>
    selectAppState(defaultAppState(), selector),
  );
  appletsKitMocks.useMediaQuery.mockReturnValue(defaultMediaQueryState());
  appletsKitMocks.useBodyScrollLock = undefined;
  appletsKitMocks.useClickAway = undefined;
  appletsKitMocks.useCountdown = undefined;
  appletsKitMocks.useHeaderHeight = undefined;
  appletsKitMocks.useInfiniteScroll = undefined;
  appletsKitMocks.Marquee = undefined;
  appletsKitMocks.usePortalTarget = undefined;
  appletsKitMocks.QRCodeReader = undefined;
  appletsKitMocks.TextLoop = undefined;
  appletsKitMocks.useAnimateOnce = undefined;
  appletsKitMocks.usePreserveScroll = undefined;
  appletsKitMocks.useTheme.mockReturnValue(defaultThemeState());
}

export function setAppletsKitUseApp(appState: MockAppState) {
  appletsKitMocks.useApp.mockImplementation((selector?: MockAppSelector) =>
    selectAppState(appState, selector),
  );
}

export function setAppletsKitUseAppFactory(factory: () => MockAppState) {
  appletsKitMocks.useApp.mockImplementation((selector?: MockAppSelector) =>
    selectAppState(factory(), selector),
  );
}

export function setAppletsKitUseMediaQuery(mediaQueryState: MockMediaQueryState) {
  appletsKitMocks.useMediaQuery.mockReturnValue(mediaQueryState);
}

export function setAppletsKitUseMediaQueryFactory(factory: () => MockMediaQueryState) {
  appletsKitMocks.useMediaQuery.mockImplementation(factory);
}

export function setAppletsKitUseBodyScrollLockFactory(factory: MockUnknownHookFactory) {
  appletsKitMocks.useBodyScrollLock = factory;
}

export function setAppletsKitUseAnimateOnceFactory(factory: MockUseAnimateOnceFactory) {
  appletsKitMocks.useAnimateOnce = factory;
}

export function setAppletsKitUseClickAwayFactory(factory: MockUnknownHookFactory) {
  appletsKitMocks.useClickAway = factory;
}

export function setAppletsKitUseCountdown(countdownState: MockCountdownState) {
  appletsKitMocks.useCountdown = () => countdownState;
}

export function setAppletsKitUseCountdownFactory(factory: MockCountdownFactory) {
  appletsKitMocks.useCountdown = factory;
}

export function setAppletsKitUseHeaderHeight(headerHeight: number) {
  appletsKitMocks.useHeaderHeight = () => headerHeight;
}

export function setAppletsKitUseInfiniteScrollFactory(factory: MockInfiniteScrollFactory) {
  appletsKitMocks.useInfiniteScroll = factory;
}

export function setAppletsKitMarqueeFactory(factory: MockMarqueeFactory) {
  appletsKitMocks.Marquee = factory;
}

export function setAppletsKitUsePortalTargetFactory(factory: MockPortalTargetFactory) {
  appletsKitMocks.usePortalTarget = factory;
}

export function setAppletsKitQRCodeReaderFactory(factory: MockQRCodeReaderFactory) {
  appletsKitMocks.QRCodeReader = factory;
}

export function setAppletsKitTextLoopFactory(factory: MockTextLoopFactory) {
  appletsKitMocks.TextLoop = factory;
}

export function setAppletsKitUsePreserveScrollFactory(factory: MockUsePreserveScrollFactory) {
  appletsKitMocks.usePreserveScroll = factory;
}

export function setAppletsKitUseTheme(themeState: MockThemeState) {
  appletsKitMocks.useTheme.mockReturnValue(themeState);
}

export function setAppletsKitUseThemeFactory(factory: () => MockThemeState) {
  appletsKitMocks.useTheme.mockImplementation(factory);
}

vi.mock("@left-curve/applets-kit", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/applets-kit")>();

  return {
    ...actual,
    Marquee: (...args: Parameters<typeof actual.Marquee>) => {
      if (appletsKitMocks.Marquee) return appletsKitMocks.Marquee(...args);
      return actual.Marquee(...args);
    },
    QRCodeReader: (...args: Parameters<typeof actual.QRCodeReader>) => {
      if (appletsKitMocks.QRCodeReader) return appletsKitMocks.QRCodeReader(...args);
      return actual.QRCodeReader(...args);
    },
    TextLoop: (...args: Parameters<typeof actual.TextLoop>) => {
      if (appletsKitMocks.TextLoop) return appletsKitMocks.TextLoop(...args);
      return actual.TextLoop(...args);
    },
    useApp: appletsKitMocks.useApp,
    useAnimateOnce: (...args: unknown[]) => {
      if (appletsKitMocks.useAnimateOnce) return appletsKitMocks.useAnimateOnce(...args);
      return actual.useAnimateOnce(...(args as Parameters<typeof actual.useAnimateOnce>));
    },
    useBodyScrollLock: (...args: unknown[]) => {
      if (appletsKitMocks.useBodyScrollLock) return appletsKitMocks.useBodyScrollLock(...args);
      return actual.useBodyScrollLock(...(args as Parameters<typeof actual.useBodyScrollLock>));
    },
    useClickAway: (...args: unknown[]) => {
      if (appletsKitMocks.useClickAway) return appletsKitMocks.useClickAway(...args);
      return actual.useClickAway(...(args as Parameters<typeof actual.useClickAway>));
    },
    useCountdown: (...args: unknown[]) => {
      if (appletsKitMocks.useCountdown) return appletsKitMocks.useCountdown(...args);
      return actual.useCountdown(...(args as Parameters<typeof actual.useCountdown>));
    },
    useHeaderHeight: () => {
      if (appletsKitMocks.useHeaderHeight) return appletsKitMocks.useHeaderHeight();
      return actual.useHeaderHeight();
    },
    useInfiniteScroll: (...args: unknown[]) => {
      if (appletsKitMocks.useInfiniteScroll) return appletsKitMocks.useInfiniteScroll(...args);
      return actual.useInfiniteScroll(...(args as Parameters<typeof actual.useInfiniteScroll>));
    },
    useMediaQuery: appletsKitMocks.useMediaQuery,
    usePortalTarget: (selector: string) => {
      if (appletsKitMocks.usePortalTarget) return appletsKitMocks.usePortalTarget(selector);
      return actual.usePortalTarget(selector);
    },
    usePreserveScroll: (...args: unknown[]) => {
      if (appletsKitMocks.usePreserveScroll) return appletsKitMocks.usePreserveScroll(...args);
      return actual.usePreserveScroll(...(args as Parameters<typeof actual.usePreserveScroll>));
    },
    useTheme: appletsKitMocks.useTheme,
  };
});
