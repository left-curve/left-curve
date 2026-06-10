import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { m } from "@left-curve/foundation/paraglide/messages.js";

const appUpdaterMocks = vi.hoisted(() => {
  const toastDismiss = vi.fn();
  const toastWarning = vi.fn();

  return {
    toast: {
      dismiss: toastDismiss,
      warning: toastWarning,
    },
    toastDismiss,
    toastWarning,
  };
});

vi.mock("@left-curve/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@left-curve/foundation")>();

  return {
    ...actual,
    useApp: () => ({
      toast: appUpdaterMocks.toast,
    }),
  };
});

import { AppUpdater, notifyUpdate } from "../src/app.updater";

describe("AppUpdater", () => {
  beforeEach(() => {
    notifyUpdate({
      waiting: null,
    } as unknown as ServiceWorkerRegistration);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("shows a persistent update toast and lets the waiting worker skip waiting", async () => {
    const postMessage = vi.fn();
    const registration = {
      waiting: {
        postMessage,
      },
    } as unknown as ServiceWorkerRegistration;

    render(<AppUpdater />);

    act(() => {
      notifyUpdate(registration);
    });

    await waitFor(() => {
      expect(appUpdaterMocks.toastWarning).toHaveBeenCalledWith(
        expect.objectContaining({
          description: expect.any(Function),
          title: m["appUpdate.title"](),
        }),
        { duration: Number.POSITIVE_INFINITY, id: "app-update" },
      );
    });
    expect(appUpdaterMocks.toastDismiss).toHaveBeenCalledWith("app-update");

    const toast = appUpdaterMocks.toastWarning.mock.calls.at(-1)?.[0];
    render(toast.description({ id: "app-update" }));

    expect(screen.getByText(m["appUpdate.description"]())).toBeInTheDocument();

    fireEvent.click(screen.getByText(m["appUpdate.updateButton"]()));

    expect(postMessage).toHaveBeenCalledWith({ type: "SKIP_WAITING" });
    expect(appUpdaterMocks.toastDismiss).toHaveBeenCalledWith("app-update");
  });

  it("shows the latest waiting-worker update that arrived before the updater mounted", async () => {
    const stalePostMessage = vi.fn();
    const latestPostMessage = vi.fn();
    const staleRegistration = {
      waiting: {
        postMessage: stalePostMessage,
      },
    } as unknown as ServiceWorkerRegistration;
    const latestRegistration = {
      waiting: {
        postMessage: latestPostMessage,
      },
    } as unknown as ServiceWorkerRegistration;

    act(() => {
      notifyUpdate(staleRegistration);
      notifyUpdate(latestRegistration);
    });

    render(<AppUpdater />);

    await waitFor(() => {
      expect(appUpdaterMocks.toastWarning).toHaveBeenCalledOnce();
    });

    const toast = appUpdaterMocks.toastWarning.mock.calls.at(-1)?.[0];
    render(toast.description({ id: "app-update" }));

    fireEvent.click(screen.getByText(m["appUpdate.updateButton"]()));

    expect(stalePostMessage).not.toHaveBeenCalled();
    expect(latestPostMessage).toHaveBeenCalledWith({ type: "SKIP_WAITING" });
  });

  it("ignores update notifications when the registration has no waiting worker", async () => {
    const registration = {
      waiting: null,
    } as unknown as ServiceWorkerRegistration;

    render(<AppUpdater />);

    act(() => {
      notifyUpdate(registration);
    });

    expect(appUpdaterMocks.toastWarning).not.toHaveBeenCalled();
    expect(appUpdaterMocks.toastDismiss).not.toHaveBeenCalled();
  });
});
