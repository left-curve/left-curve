import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

import {
  resetAppletsKitMocks,
  setAppletsKitUseCountdownFactory,
} from "./mocks/applets-kit";

import { SessionCountdown } from "../src/components/settings/SessionCountdown";

const sessionCountdownMocks = vi.hoisted(() => ({
  countdown: {
    hours: "00",
    minutes: "05",
    seconds: "09",
  },
  session: {
    sessionInfo: {
      expireAt: "1700003600",
    },
  } as { sessionInfo: { expireAt: string } } | null,
  useCountdown: vi.fn(),
}));

vi.mock("@left-curve/store", () => ({
  useSessionKey: () => ({
    session: sessionCountdownMocks.session,
  }),
}));

vi.mock("framer-motion", async () => {
  const React = await import("react");

  return {
    AnimatePresence: ({ children }: React.PropsWithChildren) => <>{children}</>,
    motion: {
      div: ({
        animate: _animate,
        children,
        exit: _exit,
        initial: _initial,
        transition: _transition,
        ...props
      }: React.ComponentProps<"div"> & {
        animate?: unknown;
        exit?: unknown;
        initial?: unknown;
        transition?: unknown;
      }) => <div {...props}>{children}</div>,
    },
  };
});

describe("SessionCountdown", () => {
  beforeEach(() => {
    resetAppletsKitMocks();
    sessionCountdownMocks.countdown = {
      hours: "00",
      minutes: "05",
      seconds: "09",
    };
    sessionCountdownMocks.session = {
      sessionInfo: {
        expireAt: "1700003600",
      },
    };
    sessionCountdownMocks.useCountdown.mockImplementation(() => sessionCountdownMocks.countdown);
    setAppletsKitUseCountdownFactory(sessionCountdownMocks.useCountdown);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("uses the session expiration timestamp as the countdown target", () => {
    render(<SessionCountdown />);

    expect(sessionCountdownMocks.useCountdown).toHaveBeenCalledWith({
      date: 1_700_003_600_000,
      showLeadingZeros: true,
    });
    expect(screen.getByText("05")).toBeInTheDocument();
    expect(screen.getByText(m["settings.session.time.minutes"]())).toBeInTheDocument();
    expect(screen.getByText("09")).toBeInTheDocument();
    expect(screen.getByText(m["settings.session.time.seconds"]())).toBeInTheDocument();
  });

  it("hides only the hour unit when the remaining hours are zero", () => {
    render(<SessionCountdown />);

    expect(screen.queryByText(m["settings.session.time.hours"]())).not.toBeInTheDocument();
    expect(screen.getByText(m["settings.session.time.minutes"]())).toBeInTheDocument();
    expect(screen.getByText(m["settings.session.time.seconds"]())).toBeInTheDocument();
  });

  it("renders the hour unit when the session has at least one hour remaining", () => {
    sessionCountdownMocks.countdown = {
      hours: "12",
      minutes: "00",
      seconds: "00",
    };

    render(<SessionCountdown />);

    expect(screen.getByText("12")).toBeInTheDocument();
    expect(screen.getByText(m["settings.session.time.hours"]())).toBeInTheDocument();
  });
});
